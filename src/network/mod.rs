//! 网络请求模块
//! 
//! 使用 reqwest 实现 HTTP 客户端，支持 API 调用和文件下载

use reqwest::{Client, Method, Response, header::HeaderMap};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, error, debug};

use crate::error::{ClaudeError, Result};

/// Claude API 请求结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeRequest {
    /// 模型名称
    pub model: String,
    /// 消息列表
    pub messages: Vec<Message>,
    /// 最大令牌数
    pub max_tokens: u32,
    /// 是否流式响应
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// 工具列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// 温度参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 系统提示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// 消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 角色 (user, assistant, system)
    pub role: String,
    /// 消息内容
    pub content: String,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入模式
    pub input_schema: serde_json::Value,
}

/// Claude API 响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    /// 响应 ID
    pub id: String,
    /// 响应类型
    #[serde(rename = "type")]
    pub response_type: String,
    /// 角色
    pub role: String,
    /// 响应内容
    pub content: String,
    /// 模型名称
    pub model: String,
    /// 停止原因
    pub stop_reason: Option<String>,
    /// 停止序列
    pub stop_sequence: Option<String>,
    /// 使用情况
    pub usage: Option<Usage>,
}

/// 使用情况统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// 输入令牌数
    pub input_tokens: u32,
    /// 输出令牌数
    pub output_tokens: u32,
}

/// 流式响应事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// 事件类型
    #[serde(rename = "type")]
    pub event_type: String,
    /// 事件数据
    pub data: Option<serde_json::Value>,
}

/// API 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// 错误类型
    #[serde(rename = "type")]
    pub error_type: String,
    /// 错误消息
    pub message: String,
}

/// HTTP 客户端管理器
pub struct NetworkManager {
    client: Client,
    base_url: String,
    default_headers: HashMap<String, String>,
}

impl NetworkManager {
    /// 创建新的网络管理器
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("claude-code-rust/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        let mut default_headers = HashMap::new();
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());
        default_headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());

        Self {
            client,
            base_url: "https://api.anthropic.com".to_string(),
            default_headers,
        }
    }

    /// 创建带自定义配置的网络管理器
    pub fn with_config(base_url: String, timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("claude-code-rust/0.1.0")
            .build()?;

        let mut default_headers = HashMap::new();
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());
        default_headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());

        Ok(Self {
            client,
            base_url,
            default_headers,
        })
    }

    /// 设置默认头部
    pub fn set_default_header(&mut self, key: String, value: String) {
        self.default_headers.insert(key, value);
    }

    /// 设置 API 密钥
    pub fn set_api_key(&mut self, api_key: String) {
        self.default_headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
    }

    /// 发送 GET 请求
    pub async fn get(&self, endpoint: &str) -> Result<Response> {
        self.request(Method::GET, endpoint, None::<&()>).await
    }

    /// 发送 POST 请求
    pub async fn post<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<Response> {
        self.request(Method::POST, endpoint, Some(body)).await
    }

    /// 发送 PUT 请求
    pub async fn put<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<Response> {
        self.request(Method::PUT, endpoint, Some(body)).await
    }

    /// 发送 DELETE 请求
    pub async fn delete(&self, endpoint: &str) -> Result<Response> {
        self.request(Method::DELETE, endpoint, None::<&()>).await
    }

    /// 通用请求方法
    async fn request<T: Serialize>(&self, method: Method, endpoint: &str, body: Option<&T>) -> Result<Response> {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'))
        };

        let mut request = self.client.request(method, &url);

        // 添加默认头部
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        // 添加请求体
        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(ClaudeError::network_error(format!(
                "HTTP request failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_else(|_| "Unknown error".to_string())
            )));
        }

        Ok(response)
    }

    /// 下载文件
    pub async fn download_file(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(ClaudeError::network_error(format!(
                "Download failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_else(|_| "Unknown error".to_string())
            )));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// 上传文件
    pub async fn upload_file(&self, endpoint: &str, file_data: Vec<u8>, file_name: &str) -> Result<Response> {
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(file_data)
                .file_name(file_name.to_string()));

        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));
        
        let mut request = self.client.post(&url).multipart(form);

        // 添加认证头部（排除 Content-Type，multipart 会自动设置）
        for (key, value) in &self.default_headers {
            if key != "Content-Type" {
                request = request.header(key, value);
            }
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(ClaudeError::network_error(format!(
                "Upload failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_else(|_| "Unknown error".to_string())
            )));
        }

        Ok(response)
    }

    /// 发送流式请求
    pub async fn post_stream<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<impl futures::Stream<Item = Result<bytes::Bytes>>> {
        use futures::StreamExt;

        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

        let mut request = self.client.post(&url);

        // 添加默认头部
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        // 添加流式请求头部
        request = request.header("Accept", "text/event-stream");
        request = request.header("Cache-Control", "no-cache");

        let response = request
            .json(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClaudeError::network_error(format!("HTTP {}: {}", status, text)));
        }

        Ok(response.bytes_stream().map(|result| {
            result.map_err(|e| ClaudeError::Network(e))
        }))
    }

    /// 发送 Server-Sent Events 流式请求
    pub async fn post_sse_stream<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<impl futures::Stream<Item = Result<String>>> {
        use futures::StreamExt;

        let stream = self.post_stream(endpoint, body).await?;

        Ok(stream.map(|chunk_result| {
            chunk_result.and_then(|chunk| {
                String::from_utf8(chunk.to_vec())
                    .map_err(|e| ClaudeError::General(format!("UTF-8 decode error: {}", e)))
            })
        }))
    }

    /// 发送请求到 Claude API
    pub async fn send_claude_request(&self, request: ClaudeRequest) -> Result<ClaudeResponse> {
        info!("Sending request to Claude API");
        debug!("Request: {:?}", request);

        // 获取 API 密钥
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ClaudeError::config_error("ANTHROPIC_API_KEY environment variable not set"))?;

        // 构建请求头
        let headers = self.build_claude_headers(&api_key)?;

        // 构建请求 URL
        let url = format!("{}/v1/messages", self.base_url);

        // 发送请求
        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        // 检查响应状态
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Claude API error: {}", error_text);
            return Err(ClaudeError::network_error(&format!("API request failed: {}", error_text)));
        }

        // 解析响应
        let response_text = response.text().await?;
        debug!("Response: {}", response_text);

        // 尝试解析为 Claude 响应
        match serde_json::from_str::<ClaudeResponse>(&response_text) {
            Ok(claude_response) => {
                info!("Successfully received Claude response");
                Ok(claude_response)
            },
            Err(e) => {
                // 如果解析失败，尝试解析为错误响应
                if let Ok(api_error) = serde_json::from_str::<ApiError>(&response_text) {
                    error!("Claude API error: {}", api_error.message);
                    Err(ClaudeError::network_error(&api_error.message))
                } else {
                    error!("Failed to parse Claude response: {}", e);
                    Err(ClaudeError::network_error(&format!("Failed to parse response: {}", e)))
                }
            }
        }
    }

    /// 构建 Claude API 请求头
    pub fn build_claude_headers(&self, api_key: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // 添加认证头
        headers.insert("x-api-key", api_key.parse()
            .map_err(|_| ClaudeError::config_error("Invalid API key format"))?);

        // 添加默认头
        for (key, value) in &self.default_headers {
            if let (Ok(header_name), Ok(header_value)) = (key.parse::<reqwest::header::HeaderName>(), value.parse::<reqwest::header::HeaderValue>()) {
                headers.insert(header_name, header_value);
            }
        }

        Ok(headers)
    }

    /// 测试与 Claude API 的连接
    pub async fn test_connection(&self) -> Result<()> {
        info!("Testing connection to Claude API");

        // 创建一个简单的测试请求
        let test_request = ClaudeRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 10,
            stream: None,
            tools: None,
            temperature: None,
            system: None,
        };

        // 尝试发送请求
        match self.send_claude_request(test_request).await {
            Ok(_) => {
                info!("Connection test successful");
                Ok(())
            },
            Err(e) => {
                warn!("Connection test failed: {}", e);
                Err(e)
            }
        }
    }
}

/// Claude API 客户端
pub struct ClaudeApiClient {
    network: NetworkManager,
    api_version: String,
    max_tokens: u32,
    temperature: f32,
    top_p: f32,
    top_k: u32,
}

impl ClaudeApiClient {
    /// 创建新的 Claude API 客户端
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self> {
        let base_url = base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string());
        let mut network = NetworkManager::with_config(base_url, Duration::from_secs(30))?;
        network.set_api_key(api_key);
        network.set_default_header("anthropic-version".to_string(), "2023-06-01".to_string());
        network.set_default_header("content-type".to_string(), "application/json".to_string());

        Ok(Self {
            network,
            api_version: "2023-06-01".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
        })
    }

    /// 设置 API 版本
    pub fn set_api_version(&mut self, version: String) {
        self.api_version = version.clone();
        self.network.set_default_header("anthropic-version".to_string(), version);
    }

    /// 设置默认参数
    pub fn set_defaults(&mut self, max_tokens: u32, temperature: f32, top_p: f32, top_k: u32) {
        self.max_tokens = max_tokens;
        self.temperature = temperature;
        self.top_p = top_p;
        self.top_k = top_k;
    }

    /// 发送消息到 Claude
    pub async fn send_message(&self, request: &MessageRequest) -> Result<MessageResponse> {
        let response = self.network.post("v1/messages", request).await?;
        let message_response: MessageResponse = response.json().await?;
        Ok(message_response)
    }

    /// 发送流式消息到 Claude
    pub async fn send_message_stream(&self, request: &MessageRequest) -> Result<impl futures::Stream<Item = Result<StreamEvent>>> {
        use futures::StreamExt;

        // 创建流式请求
        let mut stream_request = request.clone();
        stream_request.stream = Some(true);

        let stream = self.network.post_sse_stream("v1/messages", &stream_request).await?;

        Ok(stream.filter_map(|line_result| async move {
            match line_result {
                Ok(line) => {
                    // 解析 SSE 格式
                    if line.starts_with("data: ") {
                        let data = &line[6..]; // 移除 "data: " 前缀
                        if data == "[DONE]" {
                            return None; // 流结束
                        }

                        match serde_json::from_str::<StreamEvent>(data) {
                            Ok(event) => Some(Ok(event)),
                            Err(e) => Some(Err(ClaudeError::Json(e))),
                        }
                    } else {
                        None // 忽略非数据行
                    }
                }
                Err(e) => Some(Err(e)),
            }
        }))
    }

    /// 获取模型列表
    pub async fn list_models(&self) -> Result<Vec<Model>> {
        let response = self.network.get("v1/models").await?;
        let models_response: ModelsResponse = response.json().await?;
        Ok(models_response.data)
    }



    /// 创建简单文本消息请求
    pub fn create_text_request(&self, model: &str, messages: Vec<(String, String)>) -> MessageRequest {
        let messages: Vec<Message> = messages
            .into_iter()
            .map(|(role, content)| Message {
                role,
                content,
            })
            .collect();

        MessageRequest {
            model: model.to_string(),
            max_tokens: self.max_tokens,
            messages,
            system: None,
            temperature: Some(self.temperature),
            top_p: Some(self.top_p),
            top_k: Some(self.top_k),
            stream: None,
            tools: None,
            tool_choice: None,
            metadata: None,
            stop_sequences: None,
        }
    }

    /// 创建带工具的消息请求
    pub fn create_tool_request(
        &self,
        model: &str,
        messages: Vec<(String, String)>,
        tools: Vec<Tool>,
        tool_choice: Option<ToolChoice>,
    ) -> MessageRequest {
        let messages: Vec<Message> = messages
            .into_iter()
            .map(|(role, content)| Message {
                role,
                content,
            })
            .collect();

        MessageRequest {
            model: model.to_string(),
            max_tokens: self.max_tokens,
            messages,
            system: None,
            temperature: Some(self.temperature),
            top_p: Some(self.top_p),
            top_k: Some(self.top_k),
            stream: None,
            tools: Some(tools),
            tool_choice,
            metadata: None,
            stop_sequences: None,
        }
    }

    /// 创建多模态消息请求（支持图像）
    pub fn create_multimodal_request(
        &self,
        model: &str,
        role: String,
        content_blocks: Vec<ContentBlock>,
    ) -> MessageRequest {
        let message = Message {
            role,
            content: format!("Image content with {} blocks", content_blocks.len()),
        };

        MessageRequest {
            model: model.to_string(),
            max_tokens: self.max_tokens,
            messages: vec![message],
            system: None,
            temperature: Some(self.temperature),
            top_p: Some(self.top_p),
            top_k: Some(self.top_k),
            stream: None,
            tools: None,
            tool_choice: None,
            metadata: None,
            stop_sequences: None,
        }
    }

    /// 从文件创建图像内容块
    pub async fn create_image_block_from_file(&self, file_path: &str) -> Result<ContentBlock> {
        use std::path::Path;
        use tokio::fs;
        use base64::{Engine as _, engine::general_purpose};

        let path = Path::new(file_path);
        let file_data = fs::read(path).await?;
        let base64_data = general_purpose::STANDARD.encode(&file_data);

        // 根据文件扩展名确定媒体类型
        let media_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            _ => "image/jpeg", // 默认
        };

        Ok(ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: media_type.to_string(),
                data: base64_data,
            },
        })
    }

    /// 从 base64 数据创建图像内容块
    pub fn create_image_block_from_base64(
        &self,
        base64_data: String,
        media_type: String,
    ) -> ContentBlock {
        ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type,
                data: base64_data,
            },
        }
    }
}

/// 消息请求结构
#[derive(Debug, Clone, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RequestMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// 工具选择
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

/// 请求元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// 消息内容（支持文本和图像）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl MessageContent {
    /// 检查内容是否包含指定文本
    pub fn contains(&self, text: &str) -> bool {
        match self {
            MessageContent::Text(s) => s.contains(text),
            MessageContent::Blocks(blocks) => {
                blocks.iter().any(|block| match block {
                    ContentBlock::Text { text: t } => t.contains(text),
                    ContentBlock::ToolResult { content, .. } => content.contains(text),
                    _ => false,
                })
            }
        }
    }

    /// 获取内容长度
    pub fn len(&self) -> usize {
        match self {
            MessageContent::Text(s) => s.len(),
            MessageContent::Blocks(blocks) => {
                blocks.iter().map(|block| match block {
                    ContentBlock::Text { text } => text.len(),
                    ContentBlock::ToolResult { content, .. } => content.len(),
                    _ => 0,
                }).sum()
            }
        }
    }

    /// 检查内容是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 转换为字符串表示
    pub fn as_str(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Blocks(blocks) => {
                blocks.iter().map(|block| match block {
                    ContentBlock::Text { text } => text.clone(),
                    ContentBlock::ToolResult { content, .. } => content.clone(),
                    ContentBlock::ToolUse { name, .. } => format!("[Tool: {}]", name),
                    ContentBlock::Image { .. } => "[Image]".to_string(),
                }).collect::<Vec<_>>().join(" ")
            }
        }
    }

    /// 转换为字符串（用于兼容性）
    pub fn to_string(&self) -> String {
        self.as_str()
    }
}

/// 内容块（支持文本和图像）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: Option<bool>,
    },
}

/// 图像源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // "base64"
    pub media_type: String,   // "image/jpeg", "image/png", etc.
    pub data: String,         // base64 encoded image data
}

/// 消息响应结构
#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub content: Vec<ResponseContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// 响应内容块
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value
    },
}

/// 模型信息
#[derive(Debug, Deserialize)]
pub struct Model {
    pub id: String,
    pub r#type: String,
    pub display_name: String,
}

/// 模型列表响应
#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<Model>,
}

/// 流式响应增量
#[derive(Debug, Deserialize)]
pub struct StreamDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let manager = NetworkManager::new(
            "https://api.example.com".to_string(),
            Duration::from_secs(30)
        );
        assert!(manager.is_ok());
    }

    #[test]
    fn test_message_request_serialization() {
        let request = MessageRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            max_tokens: 1000,
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello, Claude!".to_string()),
            }],
            system: Some("You are a helpful assistant.".to_string()),
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(40),
            stream: None,
            tools: None,
            tool_choice: None,
            metadata: None,
            stop_sequences: None,
        };

        let json = serde_json::to_string(&request);
        assert!(json.is_ok());
    }
}



/// Claude 流式响应块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaudeStreamChunk {
    MessageStart {
        message: serde_json::Value,
    },
    ContentBlockStart {
        index: usize,
        content_block: serde_json::Value,
    },
    ContentBlockDelta {
        index: usize,
        delta: serde_json::Value,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: serde_json::Value,
    },
    MessageStop,
    Done,
    Unknown {
        data: serde_json::Value,
    },
}
