//! 对话历史管理模块
//! 
//! 实现对话历史的存储、检索、压缩和导出功能

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use crate::error::{ClaudeError, Result};

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// 消息ID
    pub id: String,
    /// 消息角色 (user, assistant, system)
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 创建时间
    pub timestamp: DateTime<Utc>,
    /// 消息元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// Token使用情况
    pub token_usage: Option<TokenUsage>,
}

/// Token使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 输入Token数
    pub input_tokens: u32,
    /// 输出Token数
    pub output_tokens: u32,
    /// 总Token数
    pub total_tokens: u32,
    /// 估算成本（美元）
    pub estimated_cost: f64,
}

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// 会话ID
    pub id: String,
    /// 会话标题
    pub title: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 消息列表
    pub messages: Vec<ConversationMessage>,
    /// 会话元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 会话标签
    pub tags: Vec<String>,
    /// 是否已归档
    pub archived: bool,
    /// 总Token使用
    pub total_token_usage: TokenUsage,
}

/// 对话历史管理器
pub struct ConversationManager {
    /// 存储目录
    storage_dir: PathBuf,
    /// 当前活跃会话
    current_conversation: Option<Conversation>,
    /// 会话缓存
    conversation_cache: HashMap<String, Conversation>,
    /// 最大缓存大小
    max_cache_size: usize,
}

impl ConversationManager {
    /// 创建新的对话管理器（简化版本）
    pub fn new() -> Self {
        Self {
            storage_dir: std::env::temp_dir().join("claude-conversations"),
            current_conversation: None,
            conversation_cache: HashMap::new(),
            max_cache_size: 100,
        }
    }

    /// 创建新的对话管理器
    pub fn with_storage_dir(storage_dir: PathBuf) -> Result<Self> {
        // 确保存储目录存在
        std::fs::create_dir_all(&storage_dir)
            .map_err(|e| ClaudeError::General(format!("Failed to create storage directory: {}", e)))?;

        Ok(Self {
            storage_dir,
            current_conversation: None,
            conversation_cache: HashMap::new(),
            max_cache_size: 100,
        })
    }

    /// 创建新对话
    pub fn create_conversation(&mut self, title: Option<String>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let conversation = Conversation {
            id: id.clone(),
            title: title.unwrap_or_else(|| format!("Conversation {}", now.format("%Y-%m-%d %H:%M"))),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            metadata: HashMap::new(),
            tags: Vec::new(),
            archived: false,
            total_token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                estimated_cost: 0.0,
            },
        };

        self.save_conversation(&conversation)?;
        self.current_conversation = Some(conversation.clone());
        self.add_to_cache(conversation);

        Ok(id)
    }

    /// 加载对话
    pub fn load_conversation(&mut self, id: &str) -> Result<()> {
        // 先检查缓存
        if let Some(conversation) = self.conversation_cache.get(id) {
            self.current_conversation = Some(conversation.clone());
            return Ok(());
        }

        // 从文件加载
        let conversation = self.load_conversation_from_file(id)?;
        self.current_conversation = Some(conversation.clone());
        self.add_to_cache(conversation);

        Ok(())
    }

    /// 添加消息到当前对话
    pub fn add_message(&mut self, role: &str, content: &str, token_usage: Option<TokenUsage>) -> Result<String> {
        let message_id = Uuid::new_v4().to_string();
        let message = ConversationMessage {
            id: message_id.clone(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            token_usage: token_usage.clone(),
        };

        if let Some(conversation) = self.current_conversation.as_mut() {
            conversation.messages.push(message);
            conversation.updated_at = Utc::now();

            // 更新总Token使用
            if let Some(usage) = token_usage {
                conversation.total_token_usage.input_tokens += usage.input_tokens;
                conversation.total_token_usage.output_tokens += usage.output_tokens;
                conversation.total_token_usage.total_tokens += usage.total_tokens;
                conversation.total_token_usage.estimated_cost += usage.estimated_cost;
            }

            // 克隆对话以避免借用冲突
            let conversation_clone = conversation.clone();
            self.save_conversation(&conversation_clone)?;
            Ok(message_id)
        } else {
            Err(ClaudeError::General("No active conversation".to_string()))
        }
    }

    /// 获取当前对话
    pub fn get_current_conversation(&self) -> Option<&Conversation> {
        self.current_conversation.as_ref()
    }

    /// 获取当前对话的消息历史
    pub fn get_conversation_messages(&self) -> Vec<ConversationMessage> {
        self.current_conversation
            .as_ref()
            .map(|c| c.messages.clone())
            .unwrap_or_default()
    }

    /// 清除当前对话历史
    pub fn clear_current_conversation(&mut self) -> Result<()> {
        if let Some(conversation) = self.current_conversation.as_mut() {
            conversation.messages.clear();
            conversation.updated_at = Utc::now();
            conversation.total_token_usage = TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                estimated_cost: 0.0,
            };

            // 克隆对话以避免借用冲突
            let conversation_clone = conversation.clone();
            self.save_conversation(&conversation_clone)?;
        }
        Ok(())
    }

    /// 压缩对话历史
    pub fn compact_conversation(&mut self, instructions: Option<&str>) -> Result<()> {
        if let Some(conversation) = self.current_conversation.as_mut() {
            // 简单的压缩策略：保留最近的消息和重要的系统消息
            let mut important_messages = Vec::new();
            let mut recent_messages = Vec::new();

            for message in &conversation.messages {
                if message.role == "system" || message.content.len() > 1000 {
                    important_messages.push(message.clone());
                }
            }

            // 保留最近的20条消息
            let recent_count = 20.min(conversation.messages.len());
            recent_messages.extend_from_slice(&conversation.messages[conversation.messages.len() - recent_count..]);

            // 合并重要消息和最近消息
            let mut compacted_messages = important_messages;
            for msg in recent_messages {
                if !compacted_messages.iter().any(|m| m.id == msg.id) {
                    compacted_messages.push(msg);
                }
            }

            // 按时间排序
            compacted_messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            conversation.messages = compacted_messages;
            conversation.updated_at = Utc::now();

            // 添加压缩说明
            if let Some(instr) = instructions {
                conversation.metadata.insert(
                    "last_compact_instructions".to_string(),
                    serde_json::Value::String(instr.to_string())
                );
            }

            // 克隆对话以避免借用冲突
            let conversation_clone = conversation.clone();
            self.save_conversation(&conversation_clone)?;
        }
        Ok(())
    }

    /// 列出所有对话
    pub fn list_conversations(&self) -> Result<Vec<ConversationSummary>> {
        let mut summaries = Vec::new();
        
        let entries = std::fs::read_dir(&self.storage_dir)
            .map_err(|e| ClaudeError::General(format!("Failed to read storage directory: {}", e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| ClaudeError::General(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(conversation) = self.load_conversation_from_file(stem) {
                        summaries.push(ConversationSummary {
                            id: conversation.id,
                            title: conversation.title,
                            created_at: conversation.created_at,
                            updated_at: conversation.updated_at,
                            message_count: conversation.messages.len(),
                            total_tokens: conversation.total_token_usage.total_tokens,
                            estimated_cost: conversation.total_token_usage.estimated_cost,
                            tags: conversation.tags,
                            archived: conversation.archived,
                        });
                    }
                }
            }
        }

        // 按更新时间排序
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    /// 保存对话到文件
    fn save_conversation(&mut self, conversation: &Conversation) -> Result<()> {
        let file_path = self.storage_dir.join(format!("{}.json", conversation.id));
        let json = serde_json::to_string_pretty(conversation)
            .map_err(|e| ClaudeError::General(format!("Failed to serialize conversation: {}", e)))?;
        
        std::fs::write(&file_path, json)
            .map_err(|e| ClaudeError::General(format!("Failed to write conversation file: {}", e)))?;

        // 更新缓存
        self.add_to_cache(conversation.clone());
        Ok(())
    }

    /// 从文件加载对话
    fn load_conversation_from_file(&self, id: &str) -> Result<Conversation> {
        let file_path = self.storage_dir.join(format!("{}.json", id));
        let json = std::fs::read_to_string(&file_path)
            .map_err(|e| ClaudeError::General(format!("Failed to read conversation file: {}", e)))?;
        
        let conversation: Conversation = serde_json::from_str(&json)
            .map_err(|e| ClaudeError::General(format!("Failed to deserialize conversation: {}", e)))?;
        
        Ok(conversation)
    }

    /// 添加到缓存
    fn add_to_cache(&mut self, conversation: Conversation) {
        if self.conversation_cache.len() >= self.max_cache_size {
            // 移除最旧的缓存项
            if let Some(oldest_id) = self.conversation_cache.keys().next().cloned() {
                self.conversation_cache.remove(&oldest_id);
            }
        }
        self.conversation_cache.insert(conversation.id.clone(), conversation);
    }

    /// 获取消息数量
    pub fn get_message_count(&self) -> usize {
        if let Some(conversation) = &self.current_conversation {
            conversation.messages.len()
        } else {
            0
        }
    }
}

/// 对话摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub total_tokens: u32,
    pub estimated_cost: f64,
    pub tags: Vec<String>,
    pub archived: bool,
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            estimated_cost: 0.0,
        }
    }
}
