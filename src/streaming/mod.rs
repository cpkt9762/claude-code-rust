//! 流式响应处理系统
//! 
//! 实现 Server-Sent Events (SSE) 解析和实时输出处理

use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio::time::interval;

use crate::error::{ClaudeError, Result};

/// SSE 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SseEventType {
    /// 消息开始
    MessageStart,
    /// 内容块开始
    ContentBlockStart,
    /// 内容块增量
    ContentBlockDelta,
    /// 内容块结束
    ContentBlockStop,
    /// 消息增量
    MessageDelta,
    /// 消息结束
    MessageStop,
    /// 错误事件
    Error,
    /// 心跳事件
    Ping,
    /// 自定义事件
    Custom(String),
}

/// SSE 事件数据
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// 事件类型
    pub event_type: SseEventType,
    /// 事件数据
    pub data: serde_json::Value,
    /// 事件 ID
    pub id: Option<String>,
    /// 重试间隔
    pub retry: Option<u64>,
    /// 时间戳
    pub timestamp: Instant,
}

/// 流式响应状态
#[derive(Debug, Clone, PartialEq)]
pub enum StreamState {
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 流式传输中
    Streaming,
    /// 已完成
    Completed,
    /// 错误状态
    Error(String),
    /// 已断开
    Disconnected,
}

/// 流式响应统计
#[derive(Debug, Clone, Default)]
pub struct StreamStats {
    /// 接收的事件数量
    pub events_received: u64,
    /// 接收的字节数
    pub bytes_received: u64,
    /// 连接时间
    pub connection_time: Option<Instant>,
    /// 第一个事件时间
    pub first_event_time: Option<Instant>,
    /// 最后一个事件时间
    pub last_event_time: Option<Instant>,
    /// 错误计数
    pub error_count: u64,
    /// 重连次数
    pub reconnect_count: u64,
}

/// 流式响应配置
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 连接超时
    pub connect_timeout: Duration,
    /// 读取超时
    pub read_timeout: Duration,
    /// 重连间隔
    pub reconnect_interval: Duration,
    /// 最大重连次数
    pub max_reconnects: u32,
    /// 心跳间隔
    pub heartbeat_interval: Duration,
    /// 是否启用压缩
    pub enable_compression: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            reconnect_interval: Duration::from_secs(5),
            max_reconnects: 3,
            heartbeat_interval: Duration::from_secs(30),
            enable_compression: true,
        }
    }
}

/// SSE 解析器
pub struct SseParser {
    /// 缓冲区
    buffer: String,
    /// 当前事件
    current_event: Option<SseEvent>,
    /// 解析统计
    stats: StreamStats,
}

impl SseParser {
    /// 创建新的 SSE 解析器
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            current_event: None,
            stats: StreamStats::default(),
        }
    }

    /// 解析 SSE 数据块
    pub fn parse_chunk(&mut self, chunk: &str) -> Result<Vec<SseEvent>> {
        self.buffer.push_str(chunk);
        self.stats.bytes_received += chunk.len() as u64;
        
        let mut events = Vec::new();
        
        // 按行分割处理
        while let Some(line_end) = self.buffer.find('\n') {
            let line = self.buffer[..line_end].trim_end_matches('\r').to_string();
            self.buffer.drain(..=line_end);

            if line.is_empty() {
                // 空行表示事件结束
                if let Some(event) = self.current_event.take() {
                    events.push(event);
                    self.stats.events_received += 1;

                    if self.stats.first_event_time.is_none() {
                        self.stats.first_event_time = Some(Instant::now());
                    }
                    self.stats.last_event_time = Some(Instant::now());
                }
            } else {
                self.parse_line(&line)?;
            }
        }
        
        Ok(events)
    }

    /// 解析单行 SSE 数据
    fn parse_line(&mut self, line: &str) -> Result<()> {
        if line.starts_with(':') {
            // 注释行，忽略
            return Ok(());
        }
        
        let (field, value) = if let Some(colon_pos) = line.find(':') {
            let field = &line[..colon_pos];
            let value = line[colon_pos + 1..].trim_start();
            (field, value)
        } else {
            (line, "")
        };
        
        // 确保有当前事件
        if self.current_event.is_none() {
            self.current_event = Some(SseEvent {
                event_type: SseEventType::Custom("unknown".to_string()),
                data: serde_json::Value::Null,
                id: None,
                retry: None,
                timestamp: Instant::now(),
            });
        }
        
        let event = self.current_event.as_mut().unwrap();
        
        match field {
            "event" => {
                event.event_type = Self::parse_event_type(value);
            }
            "data" => {
                // 解析 JSON 数据
                match serde_json::from_str(value) {
                    Ok(json_value) => event.data = json_value,
                    Err(_) => {
                        // 如果不是有效的 JSON，作为字符串处理
                        event.data = serde_json::Value::String(value.to_string());
                    }
                }
            }
            "id" => {
                event.id = Some(value.to_string());
            }
            "retry" => {
                if let Ok(retry_ms) = value.parse::<u64>() {
                    event.retry = Some(retry_ms);
                }
            }
            _ => {
                // 未知字段，忽略
            }
        }
        
        Ok(())
    }

    /// 解析事件类型
    fn parse_event_type(event_str: &str) -> SseEventType {
        match event_str {
            "message_start" => SseEventType::MessageStart,
            "content_block_start" => SseEventType::ContentBlockStart,
            "content_block_delta" => SseEventType::ContentBlockDelta,
            "content_block_stop" => SseEventType::ContentBlockStop,
            "message_delta" => SseEventType::MessageDelta,
            "message_stop" => SseEventType::MessageStop,
            "error" => SseEventType::Error,
            "ping" => SseEventType::Ping,
            custom => SseEventType::Custom(custom.to_string()),
        }
    }

    /// 获取解析统计
    pub fn get_stats(&self) -> &StreamStats {
        &self.stats
    }

    /// 重置解析器
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.current_event = None;
        self.stats = StreamStats::default();
    }
}

/// 流式响应处理器
pub struct StreamProcessor {
    /// 配置
    config: StreamConfig,
    /// 当前状态
    state: StreamState,
    /// SSE 解析器
    parser: SseParser,
    /// 事件发送器
    event_sender: broadcast::Sender<SseEvent>,
    /// 状态发送器
    state_sender: broadcast::Sender<StreamState>,
    /// 统计信息
    stats: StreamStats,
}

impl StreamProcessor {
    /// 创建新的流式处理器
    pub fn new(config: StreamConfig) -> Self {
        let (event_sender, _) = broadcast::channel(config.buffer_size);
        let (state_sender, _) = broadcast::channel(16);
        
        Self {
            config,
            state: StreamState::Disconnected,
            parser: SseParser::new(),
            event_sender,
            state_sender,
            stats: StreamStats::default(),
        }
    }

    /// 获取事件接收器
    pub fn subscribe_events(&self) -> broadcast::Receiver<SseEvent> {
        self.event_sender.subscribe()
    }

    /// 获取状态接收器
    pub fn subscribe_state(&self) -> broadcast::Receiver<StreamState> {
        self.state_sender.subscribe()
    }

    /// 处理数据块
    pub async fn process_chunk(&mut self, chunk: &str) -> Result<()> {
        if self.state == StreamState::Disconnected {
            self.set_state(StreamState::Connected).await;
        }
        
        if self.state == StreamState::Connected {
            self.set_state(StreamState::Streaming).await;
        }
        
        let events = self.parser.parse_chunk(chunk)?;
        
        for event in events {
            // 发送事件
            if let Err(_) = self.event_sender.send(event.clone()) {
                tracing::warn!("No active event subscribers");
            }
            
            // 处理特殊事件
            match &event.event_type {
                SseEventType::Error => {
                    self.stats.error_count += 1;
                    let error_msg = event.data.as_str()
                        .unwrap_or("Unknown error")
                        .to_string();
                    self.set_state(StreamState::Error(error_msg)).await;
                }
                SseEventType::MessageStop => {
                    self.set_state(StreamState::Completed).await;
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    /// 设置状态
    async fn set_state(&mut self, new_state: StreamState) {
        if self.state != new_state {
            self.state = new_state.clone();
            if let Err(_) = self.state_sender.send(new_state) {
                tracing::warn!("No active state subscribers");
            }
        }
    }

    /// 获取当前状态
    pub fn get_state(&self) -> &StreamState {
        &self.state
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> StreamStats {
        let mut stats = self.stats.clone();
        let parser_stats = self.parser.get_stats();
        
        stats.events_received = parser_stats.events_received;
        stats.bytes_received = parser_stats.bytes_received;
        stats.first_event_time = parser_stats.first_event_time;
        stats.last_event_time = parser_stats.last_event_time;
        
        stats
    }

    /// 重置处理器
    pub async fn reset(&mut self) {
        self.parser.reset();
        self.stats = StreamStats::default();
        self.set_state(StreamState::Disconnected).await;
    }
}

/// 实时输出处理器
pub struct RealTimeOutput {
    /// 输出缓冲区
    buffer: String,
    /// 是否启用实时输出
    enabled: bool,
    /// 输出发送器
    output_sender: mpsc::UnboundedSender<String>,
    /// 输出接收器
    output_receiver: Option<mpsc::UnboundedReceiver<String>>,
    /// 刷新间隔
    flush_interval: Duration,
    /// 最后刷新时间
    last_flush: Instant,
}

impl RealTimeOutput {
    /// 创建新的实时输出处理器
    pub fn new(flush_interval: Duration) -> Self {
        let (output_sender, output_receiver) = mpsc::unbounded_channel();

        Self {
            buffer: String::new(),
            enabled: true,
            output_sender,
            output_receiver: Some(output_receiver),
            flush_interval,
            last_flush: Instant::now(),
        }
    }

    /// 添加文本到缓冲区
    pub fn append(&mut self, text: &str) {
        if !self.enabled {
            return;
        }

        self.buffer.push_str(text);

        // 检查是否需要刷新
        if self.should_flush() {
            self.flush();
        }
    }

    /// 检查是否应该刷新
    fn should_flush(&self) -> bool {
        // 如果缓冲区包含换行符或者超过刷新间隔
        self.buffer.contains('\n') ||
        self.last_flush.elapsed() >= self.flush_interval ||
        self.buffer.len() > 100 // 缓冲区太大时也刷新
    }

    /// 刷新缓冲区
    pub fn flush(&mut self) {
        if !self.buffer.is_empty() {
            if let Err(_) = self.output_sender.send(self.buffer.clone()) {
                tracing::warn!("No active output subscribers");
            }
            self.buffer.clear();
            self.last_flush = Instant::now();
        }
    }

    /// 强制刷新
    pub fn force_flush(&mut self) {
        self.flush();
    }

    /// 获取输出接收器
    pub fn take_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<String>> {
        self.output_receiver.take()
    }

    /// 启用/禁用实时输出
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.buffer.clear();
        }
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// 流式响应客户端
pub struct StreamingClient {
    /// HTTP 客户端
    client: reqwest::Client,
    /// 配置
    config: StreamConfig,
    /// 处理器
    processor: StreamProcessor,
    /// 实时输出
    output: RealTimeOutput,
}

impl StreamingClient {
    /// 创建新的流式客户端
    pub fn new(config: StreamConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.connect_timeout)
            .build()
            .expect("Failed to create HTTP client");

        let processor = StreamProcessor::new(config.clone());
        let output = RealTimeOutput::new(Duration::from_millis(100));

        Self {
            client,
            config,
            processor,
            output,
        }
    }

    /// 开始流式请求
    pub async fn start_stream(&mut self, url: &str, headers: HashMap<String, String>) -> Result<()> {
        let mut request = self.client.get(url);

        // 添加 SSE 相关头部
        request = request.header("Accept", "text/event-stream");
        request = request.header("Cache-Control", "no-cache");

        // 添加自定义头部
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        let response = request.send().await
            .map_err(|e| ClaudeError::network_error(format!("Failed to start stream: {}", e)))?;

        if !response.status().is_success() {
            return Err(ClaudeError::network_error(
                format!("HTTP error: {}", response.status())
            ));
        }

        // 处理流式响应
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| ClaudeError::network_error(format!("Stream error: {}", e)))?;

            let text = String::from_utf8_lossy(&chunk);

            // 处理 SSE 事件
            self.processor.process_chunk(&text).await?;

            // 处理实时输出
            self.process_output_events().await?;
        }

        Ok(())
    }

    /// 处理输出事件
    async fn process_output_events(&mut self) -> Result<()> {
        let mut event_receiver = self.processor.subscribe_events();

        // 非阻塞地处理事件
        while let Ok(event) = event_receiver.try_recv() {
            match event.event_type {
                SseEventType::ContentBlockDelta => {
                    if let Some(text) = event.data.get("delta").and_then(|d| d.get("text")) {
                        if let Some(text_str) = text.as_str() {
                            self.output.append(text_str);
                        }
                    }
                }
                SseEventType::MessageStop => {
                    self.output.force_flush();
                }
                SseEventType::Error => {
                    let error_msg = event.data.as_str().unwrap_or("Unknown error");
                    self.output.append(&format!("\n❌ Error: {}\n", error_msg));
                    self.output.force_flush();
                }
                _ => {
                    // 其他事件类型暂时忽略
                }
            }
        }

        Ok(())
    }

    /// 获取事件订阅器
    pub fn subscribe_events(&self) -> broadcast::Receiver<SseEvent> {
        self.processor.subscribe_events()
    }

    /// 获取状态订阅器
    pub fn subscribe_state(&self) -> broadcast::Receiver<StreamState> {
        self.processor.subscribe_state()
    }

    /// 获取输出接收器
    pub fn take_output_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<String>> {
        self.output.take_receiver()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> StreamStats {
        self.processor.get_stats()
    }

    /// 处理数据块（用于测试和模拟）
    pub async fn process_chunk(&mut self, chunk: &str) -> Result<()> {
        self.processor.process_chunk(chunk).await
    }

    /// 重置客户端
    pub async fn reset(&mut self) {
        self.processor.reset().await;
        self.output = RealTimeOutput::new(Duration::from_millis(100));
    }
}
