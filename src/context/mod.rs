//! 智能上下文管理模块
//! 
//! 基于原版 wU2 压缩算法，实现 92% 阈值自动压缩和 8 段式结构化压缩

use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};
use crate::error::{ClaudeError, Result};
use crate::conversation::ConversationManager;
use crate::network::Message;

/// 上下文压缩阈值 (92%)
const COMPRESSION_THRESHOLD: f64 = 0.92;

/// 警告阈值 (60%)
const WARNING_THRESHOLD: f64 = 0.6;

/// 错误阈值 (80%)
const ERROR_THRESHOLD: f64 = 0.8;

/// 8段式压缩结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedContext {
    /// 背景上下文
    pub background_context: String,
    /// 关键决策
    pub key_decisions: Vec<String>,
    /// 工具使用记录
    pub tool_usage: Vec<ToolUsageRecord>,
    /// 用户意图
    pub user_intent: String,
    /// 执行结果
    pub execution_results: Vec<String>,
    /// 错误处理记录
    pub error_cases: Vec<String>,
    /// 未解决问题
    pub open_issues: Vec<String>,
    /// 后续计划
    pub future_plans: Vec<String>,
    /// 压缩时间戳
    pub compressed_at: u64,
    /// 原始消息数量
    pub original_message_count: usize,
    /// 压缩后大小
    pub compressed_size: usize,
}

/// 工具使用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageRecord {
    pub tool_name: String,
    pub usage_count: u32,
    pub last_used: u64,
    pub success_rate: f64,
    pub key_results: Vec<String>,
}

/// 上下文统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    /// 总 Token 数
    pub total_tokens: u32,
    /// 最大 Token 限制
    pub max_tokens: u32,
    /// 使用率
    pub usage_ratio: f64,
    /// 消息数量
    pub message_count: usize,
    /// 压缩次数
    pub compression_count: u32,
    /// 最后压缩时间
    pub last_compression: Option<u64>,
}

/// 智能上下文管理器 (wU2 压缩器的 Rust 实现)
pub struct ContextManager {
    /// 当前上下文
    current_context: VecDeque<Message>,
    /// 压缩历史
    compressed_history: Vec<CompressedContext>,
    /// 最大 Token 限制
    max_tokens: u32,
    /// 压缩阈值
    compression_threshold: f64,
    /// 统计信息
    stats: ContextStats,
    /// 重要性评分缓存
    importance_cache: HashMap<String, f64>,
}

impl ContextManager {
    /// 创建新的上下文管理器
    pub fn new(max_tokens: u32) -> Self {
        Self {
            current_context: VecDeque::new(),
            compressed_history: Vec::new(),
            max_tokens,
            compression_threshold: COMPRESSION_THRESHOLD,
            stats: ContextStats {
                total_tokens: 0,
                max_tokens,
                usage_ratio: 0.0,
                message_count: 0,
                compression_count: 0,
                last_compression: None,
            },
            importance_cache: HashMap::new(),
        }
    }

    /// 添加消息到上下文
    pub async fn add_message(&mut self, message: Message) -> Result<()> {
        self.current_context.push_back(message);
        self.update_stats().await?;
        
        // 检查是否需要压缩
        if self.should_compress().await? {
            self.compress_context().await?;
        }
        
        Ok(())
    }

    /// 获取当前上下文
    pub fn get_current_context(&self) -> &VecDeque<Message> {
        &self.current_context
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &ContextStats {
        &self.stats
    }

    /// 检查是否需要压缩
    pub async fn should_compress(&self) -> Result<bool> {
        Ok(self.stats.usage_ratio > self.compression_threshold)
    }

    /// 执行上下文压缩 (AU2 算法)
    pub async fn compress_context(&mut self) -> Result<CompressedContext> {
        tracing::info!("Starting context compression (92% threshold reached)");
        
        let start_time = Instant::now();
        let original_count = self.current_context.len();
        
        // 8段式结构化压缩
        let compressed = self.perform_structured_compression().await?;
        
        // 保留最重要的消息
        self.retain_important_messages().await?;
        
        // 更新统计信息
        self.stats.compression_count += 1;
        self.stats.last_compression = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        
        // 存储压缩历史
        self.compressed_history.push(compressed.clone());
        
        // 更新统计
        self.update_stats().await?;
        
        let duration = start_time.elapsed();
        tracing::info!(
            "Context compression completed in {:?}. Reduced from {} to {} messages",
            duration,
            original_count,
            self.current_context.len()
        );
        
        Ok(compressed)
    }

    /// 执行8段式结构化压缩
    async fn perform_structured_compression(&self) -> Result<CompressedContext> {
        let messages: Vec<&Message> = self.current_context.iter().collect();
        
        // 1. 提取背景上下文
        let background_context = self.extract_background_context(&messages).await?;
        
        // 2. 识别关键决策
        let key_decisions = self.extract_key_decisions(&messages).await?;
        
        // 3. 分析工具使用
        let tool_usage = self.analyze_tool_usage(&messages).await?;
        
        // 4. 理解用户意图
        let user_intent = self.extract_user_intent(&messages).await?;
        
        // 5. 总结执行结果
        let execution_results = self.summarize_execution_results(&messages).await?;
        
        // 6. 记录错误处理
        let error_cases = self.extract_error_cases(&messages).await?;
        
        // 7. 识别未解决问题
        let open_issues = self.identify_open_issues(&messages).await?;
        
        // 8. 制定后续计划
        let future_plans = self.generate_future_plans(&messages).await?;
        
        Ok(CompressedContext {
            background_context,
            key_decisions,
            tool_usage,
            user_intent,
            execution_results,
            error_cases,
            open_issues,
            future_plans,
            compressed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            original_message_count: messages.len(),
            compressed_size: 0, // 将在序列化后计算
        })
    }

    /// 提取背景上下文
    async fn extract_background_context(&self, messages: &[&Message]) -> Result<String> {
        // 分析消息中的背景信息
        let mut context_parts = Vec::new();

        for message in messages.iter().take(5) { // 取前5条消息作为背景
            if message.role == "user" || message.role == "system" {
                context_parts.push(message.content.as_str());
            }
        }

        Ok(context_parts.join(" | "))
    }

    /// 识别关键决策
    async fn extract_key_decisions(&self, messages: &[&Message]) -> Result<Vec<String>> {
        let mut decisions = Vec::new();

        for message in messages {
            if message.role == "assistant" && (message.content.contains("决定") || message.content.contains("选择")) {
                decisions.push(message.content.clone());
            }
        }

        Ok(decisions)
    }

    /// 分析工具使用
    async fn analyze_tool_usage(&self, messages: &[&Message]) -> Result<Vec<ToolUsageRecord>> {
        let mut tool_usage: HashMap<String, ToolUsageRecord> = HashMap::new();
        
        for message in messages {
            // 这里需要解析工具调用信息
            // 简化实现，实际需要更复杂的解析逻辑
            if message.content.contains("tool_use") {
                // 解析工具名称和使用情况
                // 暂时使用简化逻辑
            }
        }
        
        Ok(Vec::new()) // 简化返回
    }

    /// 提取用户意图
    async fn extract_user_intent(&self, messages: &[&Message]) -> Result<String> {
        // 分析用户消息，提取主要意图
        let user_messages: Vec<String> = messages
            .iter()
            .filter(|m| m.role == "user")
            .map(|m| m.content.clone())
            .collect();
        
        if let Some(last_user_message) = user_messages.last() {
            Ok(last_user_message.clone())
        } else {
            Ok("No clear user intent identified".to_string())
        }
    }

    /// 总结执行结果
    async fn summarize_execution_results(&self, messages: &[&Message]) -> Result<Vec<String>> {
        let mut results = Vec::new();
        
        for message in messages {
            if message.role == "assistant" && message.content.contains("结果") {
                results.push(message.content.clone());
            }
        }
        
        Ok(results)
    }

    /// 提取错误处理记录
    async fn extract_error_cases(&self, messages: &[&Message]) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        
        for message in messages {
            if message.content.contains("错误") || message.content.contains("error") {
                errors.push(message.content.clone());
            }
        }
        
        Ok(errors)
    }

    /// 识别未解决问题
    async fn identify_open_issues(&self, messages: &[&Message]) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        for message in messages {
            if message.content.contains("问题") || message.content.contains("待解决") {
                issues.push(message.content.clone());
            }
        }
        
        Ok(issues)
    }

    /// 生成后续计划
    async fn generate_future_plans(&self, messages: &[&Message]) -> Result<Vec<String>> {
        let mut plans = Vec::new();
        
        for message in messages {
            if message.content.contains("计划") || message.content.contains("下一步") {
                plans.push(message.content.clone());
            }
        }
        
        Ok(plans)
    }

    /// 保留重要消息
    async fn retain_important_messages(&mut self) -> Result<()> {
        let mut important_messages = VecDeque::new();
        
        // 计算每条消息的重要性评分
        let messages_to_check: Vec<_> = self.current_context.iter().cloned().collect();
        for message in messages_to_check {
            let importance = self.calculate_importance_score(&message).await?;
            if importance > 0.7 { // 保留重要性评分 > 0.7 的消息
                important_messages.push_back(message);
            }
        }
        
        // 至少保留最后几条消息
        let min_retain = 5;
        while important_messages.len() < min_retain && !self.current_context.is_empty() {
            if let Some(message) = self.current_context.pop_back() {
                important_messages.push_front(message);
            }
        }
        
        self.current_context = important_messages;
        Ok(())
    }

    /// 计算消息重要性评分
    async fn calculate_importance_score(&mut self, message: &Message) -> Result<f64> {
        // 检查缓存
        let content_key = message.content.as_str();
        if let Some(&score) = self.importance_cache.get(content_key) {
            return Ok(score);
        }
        
        let mut score = 0.0;
        
        // 基于角色的基础分数
        match message.role.as_str() {
            "system" => score += 0.8,
            "user" => score += 0.6,
            "assistant" => score += 0.4,
            _ => score += 0.2,
        }
        
        // 基于内容的评分
        if message.content.contains("重要") || message.content.contains("关键") {
            score += 0.3;
        }
        
        if message.content.contains("错误") || message.content.contains("error") {
            score += 0.2;
        }
        
        if message.content.len() > 100 {
            score += 0.1;
        }
        
        // 缓存评分
        self.importance_cache.insert(message.content.clone(), score);
        
        Ok(score.min(1.0))
    }

    /// 更新统计信息
    async fn update_stats(&mut self) -> Result<()> {
        self.stats.message_count = self.current_context.len();
        self.stats.total_tokens = self.estimate_token_count();
        self.stats.usage_ratio = self.stats.total_tokens as f64 / self.stats.max_tokens as f64;
        
        Ok(())
    }

    /// 估算 Token 数量
    fn estimate_token_count(&self) -> u32 {
        // 简化的 Token 估算：大约 4 个字符 = 1 个 Token
        let total_chars: usize = self.current_context
            .iter()
            .map(|m| m.content.len())
            .sum();
        
        (total_chars / 4) as u32
    }

    /// 获取压缩历史
    pub fn get_compression_history(&self) -> &[CompressedContext] {
        &self.compressed_history
    }

    /// 清除压缩历史
    pub fn clear_compression_history(&mut self) {
        self.compressed_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_manager_creation() {
        let manager = ContextManager::new(100000);
        assert_eq!(manager.max_tokens, 100000);
        assert_eq!(manager.stats.message_count, 0);
    }

    #[tokio::test]
    async fn test_add_message() {
        let mut manager = ContextManager::new(100000);
        let message = Message {
            role: "user".to_string(),
            content: crate::network::MessageContent::Text("Hello, Claude!".to_string()),
        };
        
        manager.add_message(message).await.unwrap();
        assert_eq!(manager.stats.message_count, 1);
    }

    #[tokio::test]
    async fn test_importance_scoring() {
        let mut manager = ContextManager::new(100000);
        let important_message = Message {
            role: "system".to_string(),
            content: crate::network::MessageContent::Text("这是一个重要的系统消息".to_string()),
        };
        
        let score = manager.calculate_importance_score(&important_message).await.unwrap();
        assert!(score > 0.8);
    }
}
