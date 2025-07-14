//! MCP (Model Context Protocol) 服务器管理模块
//! 
//! 实现 MCP 服务器的启动、停止、配置管理和通信协议

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child as AsyncChild, Command as AsyncCommand};
use tokio::sync::mpsc;

use crate::config::McpServerConfig;
use crate::error::{ClaudeError, Result};

/// MCP 服务器管理器
pub struct McpManager {
    /// 运行中的服务器
    running_servers: Arc<Mutex<HashMap<String, McpServerInstance>>>,
}

/// MCP 服务器实例
pub struct McpServerInstance {
    /// 服务器配置
    config: McpServerConfig,
    /// 子进程
    process: Option<AsyncChild>,
    /// 状态
    status: McpServerStatus,
    /// 消息发送通道
    message_sender: Option<mpsc::UnboundedSender<McpMessage>>,
    /// 消息接收通道
    message_receiver: Option<mpsc::UnboundedReceiver<McpMessage>>,
}

/// MCP 服务器状态
#[derive(Debug, Clone, PartialEq)]
pub enum McpServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

/// MCP 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpMessage {
    Request {
        id: Option<String>,
        method: String,
        params: serde_json::Value,
    },
    Response {
        id: Option<String>,
        result: Option<serde_json::Value>,
        error: Option<McpError>,
    },
    Notification {
        method: String,
        params: serde_json::Value,
    },
}



/// MCP 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl McpManager {
    /// 创建新的 MCP 管理器
    pub fn new() -> Self {
        Self {
            running_servers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 启动 MCP 服务器
    pub async fn start_server(&self, config: McpServerConfig) -> Result<()> {
        let server_name = config.name.clone();
        
        tracing::info!("Starting MCP server: {}", server_name);
        
        // 检查服务器是否已经在运行
        {
            let servers = self.running_servers.lock().unwrap();
            if let Some(instance) = servers.get(&server_name) {
                if instance.status == McpServerStatus::Running {
                    return Err(ClaudeError::mcp_error(format!(
                        "Server '{}' is already running", server_name
                    )));
                }
            }
        }

        // 创建服务器实例
        let mut instance = McpServerInstance {
            config: config.clone(),
            process: None,
            status: McpServerStatus::Starting,
            message_sender: None,
            message_receiver: None,
        };

        // 启动子进程
        let mut cmd = AsyncCommand::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // 设置环境变量
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // 设置工作目录
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        let mut child = cmd.spawn()?;
        
        // 设置通信通道
        let (tx, rx) = mpsc::unbounded_channel();
        instance.message_sender = Some(tx);
        instance.message_receiver = Some(rx);

        // 启动消息处理任务
        let stdin = child.stdin.take().ok_or_else(|| {
            ClaudeError::mcp_error("Failed to get stdin handle")
        })?;
        
        let stdout = child.stdout.take().ok_or_else(|| {
            ClaudeError::mcp_error("Failed to get stdout handle")
        })?;

        let stderr = child.stderr.take().ok_or_else(|| {
            ClaudeError::mcp_error("Failed to get stderr handle")
        })?;

        // 启动输出读取任务
        let server_name_clone = server_name.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                
                tracing::debug!("MCP server '{}' stdout: {}", server_name_clone, line.trim());
                
                // 尝试解析 JSON-RPC 消息
                if let Ok(message) = serde_json::from_str::<McpMessage>(&line) {
                    tracing::debug!("Received MCP message: {:?}", message);

                    // 处理接收到的消息
                    match message {
                        McpMessage::Request { id, method, params } => {
                            tracing::info!("Received MCP request: method={}, id={:?}", method, id);
                            // 可以在这里添加请求处理逻辑
                        }
                        McpMessage::Response { id, result, error } => {
                            tracing::info!("Received MCP response: id={:?}", id);
                            if let Some(error) = error {
                                tracing::warn!("MCP response error: {:?}", error);
                            }
                        }
                        McpMessage::Notification { method, params } => {
                            tracing::info!("Received MCP notification: method={}", method);
                            // 处理通知消息
                        }
                    }
                }
                
                line.clear();
            }
        });

        // 启动错误输出读取任务
        let server_name_clone = server_name.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                
                tracing::warn!("MCP server '{}' stderr: {}", server_name_clone, line.trim());
                line.clear();
            }
        });

        instance.process = Some(child);
        instance.status = McpServerStatus::Running;

        // 存储服务器实例
        {
            let mut servers = self.running_servers.lock().unwrap();
            servers.insert(server_name.clone(), instance);
        }

        tracing::info!("MCP server '{}' started successfully", server_name);
        Ok(())
    }

    /// 停止 MCP 服务器
    pub async fn stop_server(&self, server_name: &str) -> Result<()> {
        tracing::info!("Stopping MCP server: {}", server_name);

        let mut instance = {
            let mut servers = self.running_servers.lock().unwrap();
            servers.remove(server_name).ok_or_else(|| {
                ClaudeError::mcp_error(format!("Server '{}' not found", server_name))
            })?
        };

        instance.status = McpServerStatus::Stopping;

        if let Some(mut process) = instance.process.take() {
            // 尝试优雅关闭
            if let Err(e) = process.kill().await {
                tracing::warn!("Failed to kill MCP server '{}': {}", server_name, e);
            }

            // 等待进程结束
            match process.wait().await {
                Ok(status) => {
                    tracing::info!("MCP server '{}' exited with status: {}", server_name, status);
                }
                Err(e) => {
                    tracing::error!("Error waiting for MCP server '{}': {}", server_name, e);
                }
            }
        }

        tracing::info!("MCP server '{}' stopped", server_name);
        Ok(())
    }

    /// 发送消息到 MCP 服务器
    pub async fn send_message(&self, server_name: &str, message: McpMessage) -> Result<()> {
        let servers = self.running_servers.lock().unwrap();
        let instance = servers.get(server_name).ok_or_else(|| {
            ClaudeError::mcp_error(format!("Server '{}' not found", server_name))
        })?;

        if instance.status != McpServerStatus::Running {
            return Err(ClaudeError::mcp_error(format!(
                "Server '{}' is not running", server_name
            )));
        }

        if let Some(sender) = &instance.message_sender {
            sender.send(message).map_err(|e| {
                ClaudeError::mcp_error(format!("Failed to send message: {}", e))
            })?;
        }

        Ok(())
    }

    /// 获取服务器状态
    pub fn get_server_status(&self, server_name: &str) -> Option<McpServerStatus> {
        let servers = self.running_servers.lock().unwrap();
        servers.get(server_name).map(|instance| instance.status.clone())
    }

    /// 列出所有运行中的服务器
    pub fn list_running_servers(&self) -> Vec<String> {
        let servers = self.running_servers.lock().unwrap();
        servers.keys().cloned().collect()
    }

    /// 停止所有服务器
    pub async fn stop_all_servers(&self) -> Result<()> {
        let server_names: Vec<String> = {
            let servers = self.running_servers.lock().unwrap();
            servers.keys().cloned().collect()
        };

        for server_name in server_names {
            if let Err(e) = self.stop_server(&server_name).await {
                tracing::error!("Failed to stop server '{}': {}", server_name, e);
            }
        }

        Ok(())
    }
}

impl Drop for McpManager {
    fn drop(&mut self) {
        // 在析构时尝试停止所有服务器
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = self.stop_all_servers().await {
                tracing::error!("Failed to stop all MCP servers during cleanup: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_mcp_manager_creation() {
        let manager = McpManager::new();
        assert!(manager.list_running_servers().is_empty());
    }

    #[test]
    fn test_mcp_message_serialization() {
        let message = McpMessage::Request {
            id: Some("test-id".to_string()),
            method: "test-method".to_string(),
            params: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: McpMessage = serde_json::from_str(&json).unwrap();

        match (&message, &deserialized) {
            (
                McpMessage::Request { id: id1, method: method1, .. },
                McpMessage::Request { id: id2, method: method2, .. }
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(method1, method2);
            }
            _ => panic!("Message types don't match"),
        }
    }
}
