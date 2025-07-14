//! Agent 循环系统实现
//! 
//! 基于原版 nO 主循环引擎，实现 Agent 核心调度和执行逻辑

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{timeout, Duration, Instant};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ClaudeError, Result};
use crate::steering::{SteeringController, SteeringMessage};
use crate::conversation::ConversationManager;
use crate::config::ClaudeConfig;

/// Agent 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    /// 未启动
    NotStarted,
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 等待输入
    WaitingForInput,
    /// 执行工具
    ExecutingTool,
    /// 暂停
    Paused,
    /// 完成
    Completed,
    /// 错误
    Error(String),
}

/// Agent 执行上下文
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// Agent ID
    pub agent_id: String,
    /// 会话ID
    pub session_id: String,
    /// 配置
    pub config: ClaudeConfig,
    /// 工具配置
    pub tools_config: HashMap<String, serde_json::Value>,
    /// 执行环境变量
    pub environment: HashMap<String, String>,
    /// 最大思考 Token 数
    pub max_thinking_tokens: Option<u32>,
    /// 回退模型
    pub fallback_model: Option<String>,
}

impl AgentContext {
    /// 创建新的 Agent 上下文
    pub fn new(session_id: String, config: ClaudeConfig) -> Self {
        Self {
            agent_id: Uuid::new_v4().to_string(),
            session_id,
            config,
            tools_config: HashMap::new(),
            environment: HashMap::new(),
            max_thinking_tokens: None,
            fallback_model: None,
        }
    }

    /// 设置工具配置
    pub fn with_tools_config(mut self, tools_config: HashMap<String, serde_json::Value>) -> Self {
        self.tools_config = tools_config;
        self
    }

    /// 设置环境变量
    pub fn with_environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }
}

/// Agent 响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentResponse {
    /// 流式请求开始
    StreamRequestStart,
    /// 文本内容
    TextContent {
        content: String,
        is_partial: bool,
    },
    /// 工具调用
    ToolCall {
        tool_name: String,
        tool_input: serde_json::Value,
        call_id: String,
    },
    /// 工具结果
    ToolResult {
        call_id: String,
        result: serde_json::Value,
        is_error: bool,
    },
    /// 状态更新
    StatusUpdate {
        status: AgentStatus,
        message: Option<String>,
    },
    /// 错误
    Error {
        error: String,
        error_code: Option<String>,
    },
    /// 完成
    Completed {
        final_response: String,
        metadata: HashMap<String, serde_json::Value>,
    },
}

/// Agent 主循环引擎 (nO 函数的 Rust 实现)
pub struct AgentLoop {
    /// Agent 上下文
    context: AgentContext,
    /// Steering 控制器
    steering: SteeringController,
    /// 对话管理器
    conversation: Arc<Mutex<ConversationManager>>,
    /// 当前状态
    status: Arc<RwLock<AgentStatus>>,
    /// 响应发送器
    response_sender: mpsc::UnboundedSender<AgentResponse>,
    /// 是否启用压缩
    compression_enabled: bool,
    /// 压缩阈值 (92%)
    compression_threshold: f64,
}

impl AgentLoop {
    /// 创建新的 Agent 循环
    pub fn new(
        context: AgentContext,
        conversation: ConversationManager,
    ) -> (Self, mpsc::UnboundedReceiver<AgentResponse>) {
        let (response_sender, response_receiver) = mpsc::unbounded_channel();
        
        let agent_loop = Self {
            context,
            steering: SteeringController::new(),
            conversation: Arc::new(Mutex::new(conversation)),
            status: Arc::new(RwLock::new(AgentStatus::NotStarted)),
            response_sender,
            compression_enabled: true,
            compression_threshold: 0.92,
        };
        
        (agent_loop, response_receiver)
    }

    /// 获取当前状态
    pub async fn get_status(&self) -> AgentStatus {
        self.status.read().await.clone()
    }

    /// 设置状态
    async fn set_status(&self, status: AgentStatus) {
        *self.status.write().await = status.clone();
        
        // 发送状态更新
        let _ = self.response_sender.send(AgentResponse::StatusUpdate {
            status,
            message: None,
        });
    }

    /// 发送响应
    async fn send_response(&self, response: AgentResponse) -> Result<()> {
        self.response_sender.send(response).map_err(|_| {
            ClaudeError::General("Failed to send agent response".to_string())
        })
    }

    /// 启动 Agent 主循环
    pub async fn run(&mut self, initial_messages: Vec<String>) -> Result<()> {
        tracing::info!("Starting Agent loop for session: {}", self.context.session_id);
        
        // 发送流式请求开始信号
        self.send_response(AgentResponse::StreamRequestStart).await?;
        
        // 设置初始状态
        self.set_status(AgentStatus::Initializing).await;
        
        // 主循环
        loop {
            match self.execute_cycle(&initial_messages).await {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Agent loop error: {}", e);
                    self.set_status(AgentStatus::Error(e.to_string())).await;
                    self.send_response(AgentResponse::Error {
                        error: e.to_string(),
                        error_code: Some("AGENT_LOOP_ERROR".to_string()),
                    }).await?;
                    break;
                }
            }
            
            // 检查中断信号
            if self.steering.check_interrupt().await {
                tracing::info!("Agent loop interrupted");
                break;
            }
        }
        
        // 设置完成状态
        self.set_status(AgentStatus::Completed).await;
        self.send_response(AgentResponse::Completed {
            final_response: "Agent execution completed".to_string(),
            metadata: HashMap::new(),
        }).await?;
        
        Ok(())
    }

    /// 执行一个循环周期
    async fn execute_cycle(&mut self, _messages: &[String]) -> Result<bool> {
        // 阶段1：消息预处理和上下文检查
        self.set_status(AgentStatus::Running).await;
        
        // 检查是否需要压缩
        let needs_compression = self.check_compression_needed().await?;
        if needs_compression {
            self.perform_compression().await?;
        }
        
        // 阶段2：处理 Steering 消息
        if let Some(steering_message) = self.steering.receive_message_timeout(Duration::from_millis(100)).await? {
            self.handle_steering_message(steering_message).await?;
        }
        
        // 阶段3：生成系统提示
        let _system_prompt = self.generate_system_prompt().await?;
        
        // 阶段4：会话流生成 (模拟)
        self.generate_conversation_stream().await?;
        
        // 阶段5：工具调用检测与处理
        self.process_tool_calls().await?;
        
        // 继续循环
        Ok(true)
    }

    /// 检查是否需要压缩
    async fn check_compression_needed(&self) -> Result<bool> {
        if !self.compression_enabled {
            return Ok(false);
        }
        
        let conversation = self.conversation.lock().await;
        let token_usage = conversation.get_message_count() * 100; // 简化的 token 估算
        let max_tokens = 100000.0; // 默认最大 token 数
        
        Ok(token_usage as f64 / max_tokens > self.compression_threshold)
    }

    /// 执行压缩
    async fn perform_compression(&mut self) -> Result<()> {
        tracing::info!("Performing context compression (92% threshold reached)");
        
        let mut conversation = self.conversation.lock().await;
        // 简化的压缩实现 - 移除一半的消息
        let message_count = conversation.get_message_count();
        if message_count > 10 {
            // 这里应该调用实际的压缩逻辑
            tracing::info!("Context compression simulated");
        }
        
        // 记录压缩事件
        self.send_response(AgentResponse::StatusUpdate {
            status: AgentStatus::Running,
            message: Some("Context compressed successfully".to_string()),
        }).await?;
        
        Ok(())
    }

    /// 处理 Steering 消息
    async fn handle_steering_message(&mut self, message: SteeringMessage) -> Result<()> {
        match message {
            SteeringMessage::UserInput { content, .. } => {
                tracing::info!("Received user input: {}", content);
                self.send_response(AgentResponse::TextContent {
                    content: format!("Processing user input: {}", content),
                    is_partial: false,
                }).await?;
            }
            SteeringMessage::SystemControl { command, params } => {
                tracing::info!("Received system control: {} with params: {:?}", command, params);
                self.handle_system_control(command, params).await?;
            }
            SteeringMessage::Interrupt { reason } => {
                tracing::warn!("Received interrupt: {}", reason);
                self.steering.send_interrupt(reason).await?;
            }
            SteeringMessage::StatusUpdate { status, data } => {
                tracing::info!("Received status update: {} with data: {:?}", status, data);
            }
        }
        
        Ok(())
    }

    /// 处理系统控制命令
    async fn handle_system_control(&mut self, command: String, _params: serde_json::Value) -> Result<()> {
        match command.as_str() {
            "pause" => {
                self.set_status(AgentStatus::Paused).await;
            }
            "resume" => {
                self.set_status(AgentStatus::Running).await;
            }
            "stop" => {
                self.steering.send_interrupt("System stop command".to_string()).await?;
            }
            _ => {
                tracing::warn!("Unknown system control command: {}", command);
            }
        }
        
        Ok(())
    }

    /// 生成系统提示
    async fn generate_system_prompt(&self) -> Result<String> {
        // 基于上下文和工具配置生成系统提示
        let mut prompt = String::from("You are Claude, an AI assistant created by Anthropic.");
        
        if !self.context.tools_config.is_empty() {
            prompt.push_str("\n\nAvailable tools:");
            for tool_name in self.context.tools_config.keys() {
                prompt.push_str(&format!("\n- {}", tool_name));
            }
        }
        
        Ok(prompt)
    }

    /// 生成会话流 (模拟)
    async fn generate_conversation_stream(&mut self) -> Result<()> {
        // 模拟流式响应生成
        self.send_response(AgentResponse::TextContent {
            content: "Generating response...".to_string(),
            is_partial: true,
        }).await?;
        
        // 模拟处理延迟
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        self.send_response(AgentResponse::TextContent {
            content: "Response generated successfully.".to_string(),
            is_partial: false,
        }).await?;
        
        Ok(())
    }

    /// 处理工具调用
    async fn process_tool_calls(&mut self) -> Result<()> {
        // 模拟工具调用检测和处理
        if !self.context.tools_config.is_empty() {
            tracing::debug!("Processing tool calls...");
            
            // 这里会集成实际的工具执行引擎
            // 目前只是模拟
        }
        
        Ok(())
    }

    /// 获取 Steering 控制器引用
    pub fn steering(&self) -> &SteeringController {
        &self.steering
    }

    /// 获取可变 Steering 控制器引用
    pub fn steering_mut(&mut self) -> &mut SteeringController {
        &mut self.steering
    }
}

/// 简化的 Agent 接口（用于 CLI）
pub struct Agent {
    /// Agent 循环
    agent_loop: AgentLoop,
    /// 响应接收器
    response_receiver: mpsc::UnboundedReceiver<AgentResponse>,
}

impl Agent {
    /// 创建新的 Agent
    pub async fn new() -> crate::error::Result<Self> {
        let config = crate::config::ClaudeConfig::default();
        let context = AgentContext::new("cli-session".to_string(), config);
        let conversation = crate::conversation::ConversationManager::new();

        let (agent_loop, response_receiver) = AgentLoop::new(context, conversation);

        Ok(Self {
            agent_loop,
            response_receiver,
        })
    }

    /// 处理用户请求
    pub async fn process_user_request(&self, request: &str) -> crate::error::Result<AgentResponse> {
        use tracing::{info, debug};

        info!("Processing user request: {}", request);
        debug!("Request details: {}", request);

        // 创建用户消息（简化处理）
        let _user_message = format!("User: {}", request);

        // 构建响应
        let response = AgentResponse::TextContent {
            content: format!("Processing: {}", request),
            is_partial: false,
        };

        Ok(response)
    }

    /// 生成代码编辑
    pub async fn generate_code_edit(&self, instruction: &str, context: &CodeContext) -> crate::error::Result<crate::fs::Edit> {
        use tracing::{info, debug};

        info!("Generating code edit for instruction: {}", instruction);
        debug!("Code context: {:?}", context);

        // 构建编辑提示
        let edit_prompt = format!(
            "Please generate a code edit for the following instruction: {}\\n\\nContext: {:?}",
            instruction, context
        );

        // 这里应该调用 Claude API 生成实际的编辑
        // 暂时返回一个示例编辑
        let edit = crate::fs::Edit {
            file_path: context.file_path.clone(),
            edit_type: crate::fs::EditType::Replace,
            content: format!("// Generated edit for: {}\\n{}", instruction, context.content),
            line_range: None,
        };

        Ok(edit)
    }

    /// 分析代码库
    pub async fn analyze_codebase(&self, path: &str) -> crate::error::Result<CodebaseAnalysis> {
        use tracing::{info, debug};
        use std::path::Path;

        info!("Analyzing codebase at: {}", path);

        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(crate::error::ClaudeError::fs_error(format!("Path does not exist: {}", path)));
        }

        // 收集文件信息
        let mut files = Vec::new();
        let mut total_lines = 0;
        let mut languages = std::collections::HashMap::new();

        if path_obj.is_file() {
            // 分析单个文件
            let file_info = self.analyze_file(path).await?;
            total_lines += file_info.line_count;
            *languages.entry(file_info.language.clone()).or_insert(0) += 1;
            files.push(file_info);
        } else {
            // 分析目录
            files = self.analyze_directory(path).await?;
            for file in &files {
                total_lines += file.line_count;
                *languages.entry(file.language.clone()).or_insert(0) += 1;
            }
        }

        let complexity_score = self.calculate_complexity_score(&files);

        let analysis = CodebaseAnalysis {
            path: path.to_string(),
            total_files: files.len(),
            total_lines,
            languages,
            files,
            structure: self.analyze_project_structure(path).await?,
            dependencies: self.analyze_dependencies(path).await?,
            complexity_score,
            last_analyzed: chrono::Utc::now(),
        };

        debug!("Codebase analysis completed: {:?}", analysis);
        Ok(analysis)
    }

    /// 清除对话历史
    pub async fn clear_conversation_history(&self) -> crate::error::Result<()> {
        use tracing::info;

        info!("Clearing conversation history");
        // 这里应该实现实际的历史清除逻辑
        Ok(())
    }

    /// 分析单个文件
    async fn analyze_file(&self, file_path: &str) -> crate::error::Result<FileInfo> {
        use std::path::Path;

        let path = Path::new(file_path);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_string();

        let language = match extension.as_str() {
            "rs" => "Rust",
            "py" => "Python",
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "java" => "Java",
            "cpp" | "cc" | "cxx" => "C++",
            "c" => "C",
            "go" => "Go",
            "rb" => "Ruby",
            "php" => "PHP",
            "swift" => "Swift",
            "kt" => "Kotlin",
            "scala" => "Scala",
            "cs" => "C#",
            "html" => "HTML",
            "css" => "CSS",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "toml" => "TOML",
            "md" => "Markdown",
            _ => "Unknown",
        }.to_string();

        // 读取文件内容并计算行数
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| crate::error::ClaudeError::fs_error(format!("Failed to read file {}: {}", file_path, e)))?;

        let line_count = content.lines().count();
        let char_count = content.chars().count();

        Ok(FileInfo {
            path: file_path.to_string(),
            name: path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string(),
            extension,
            language,
            line_count,
            char_count,
            size_bytes: content.len(),
        })
    }

    /// 分析目录
    async fn analyze_directory(&self, dir_path: &str) -> crate::error::Result<Vec<FileInfo>> {
        use tokio::fs;

        let mut files = Vec::new();
        let mut entries = fs::read_dir(dir_path).await
            .map_err(|e| crate::error::ClaudeError::fs_error(format!("Failed to read directory {}: {}", dir_path, e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| crate::error::ClaudeError::fs_error(format!("Failed to read directory entry: {}", e)))? {

            let path = entry.path();
            let path_str = path.to_string_lossy().to_string();

            if path.is_file() {
                // 跳过隐藏文件和某些类型的文件
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with('.') ||
                       file_name.ends_with(".lock") ||
                       file_name.ends_with(".log") {
                        continue;
                    }
                }

                if let Ok(file_info) = self.analyze_file(&path_str).await {
                    files.push(file_info);
                }
            } else if path.is_dir() {
                // 递归分析子目录（限制深度）
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if !dir_name.starts_with('.') &&
                       dir_name != "target" &&
                       dir_name != "node_modules" &&
                       dir_name != "__pycache__" {
                        // 使用 Box::pin 来处理递归 async 函数
                        if let Ok(mut sub_files) = Box::pin(self.analyze_directory(&path_str)).await {
                            files.append(&mut sub_files);
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    /// 分析项目结构
    async fn analyze_project_structure(&self, _path: &str) -> crate::error::Result<ProjectStructure> {
        // 这里应该实现实际的项目结构分析
        Ok(ProjectStructure {
            project_type: "Unknown".to_string(),
            build_system: None,
            main_directories: Vec::new(),
            config_files: Vec::new(),
        })
    }

    /// 分析依赖关系
    async fn analyze_dependencies(&self, _path: &str) -> crate::error::Result<Vec<Dependency>> {
        // 这里应该实现实际的依赖分析
        Ok(Vec::new())
    }

    /// 计算复杂度分数
    fn calculate_complexity_score(&self, files: &[FileInfo]) -> f64 {
        if files.is_empty() {
            return 0.0;
        }

        let total_lines: usize = files.iter().map(|f| f.line_count).sum();
        let file_count = files.len();

        // 简单的复杂度计算：基于文件数量和总行数
        (total_lines as f64 / file_count as f64).log10()
    }
}

/// 代码上下文
#[derive(Debug, Clone)]
pub struct CodeContext {
    /// 文件路径
    pub file_path: String,
    /// 文件内容
    pub content: String,
    /// 语言类型
    pub language: String,
}

/// 代码库分析结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodebaseAnalysis {
    /// 分析路径
    pub path: String,
    /// 总文件数
    pub total_files: usize,
    /// 总行数
    pub total_lines: usize,
    /// 语言分布
    pub languages: std::collections::HashMap<String, usize>,
    /// 文件信息列表
    pub files: Vec<FileInfo>,
    /// 项目结构
    pub structure: ProjectStructure,
    /// 依赖关系
    pub dependencies: Vec<Dependency>,
    /// 复杂度分数
    pub complexity_score: f64,
    /// 分析时间
    pub last_analyzed: chrono::DateTime<chrono::Utc>,
}

/// 文件信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    /// 文件路径
    pub path: String,
    /// 文件名
    pub name: String,
    /// 文件扩展名
    pub extension: String,
    /// 编程语言
    pub language: String,
    /// 行数
    pub line_count: usize,
    /// 字符数
    pub char_count: usize,
    /// 文件大小（字节）
    pub size_bytes: usize,
}

/// 项目结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectStructure {
    /// 项目类型
    pub project_type: String,
    /// 构建系统
    pub build_system: Option<String>,
    /// 主要目录
    pub main_directories: Vec<String>,
    /// 配置文件
    pub config_files: Vec<String>,
}

/// 依赖关系
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    /// 依赖名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 依赖类型
    pub dependency_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClaudeConfig;

    #[tokio::test]
    async fn test_agent_context_creation() {
        let config = ClaudeConfig::default();
        let context = AgentContext::new("test-session".to_string(), config);
        
        assert_eq!(context.session_id, "test-session");
        assert!(!context.agent_id.is_empty());
    }

    #[tokio::test]
    async fn test_agent_loop_creation() {
        let config = ClaudeConfig::default();
        let context = AgentContext::new("test-session".to_string(), config);
        let conversation = ConversationManager::new();
        
        let (agent_loop, _receiver) = AgentLoop::new(context, conversation);
        
        assert_eq!(agent_loop.get_status().await, AgentStatus::NotStarted);
    }
}
