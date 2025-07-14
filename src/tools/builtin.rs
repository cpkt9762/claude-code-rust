//! 内置工具实现
//! 
//! 实现 Claude Code 的核心内置工具

use super::*;
use crate::fs::FileSystemManager;
use std::path::Path;
use tokio::process::Command;

/// 文件读取工具
pub struct ReadTool {
    fs_manager: FileSystemManager,
}

impl ReadTool {
    pub fn new() -> Self {
        Self {
            fs_manager: FileSystemManager::new(vec![std::env::current_dir().unwrap_or_default()]),
        }
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read".to_string(),
            description: "Read the contents of a file".to_string(),
            version: "1.0.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the file to read".to_string(),
                    required: true,
                    default: None,
                    constraints: None,
                },
                ToolParameter {
                    name: "encoding".to_string(),
                    param_type: "string".to_string(),
                    description: "File encoding (default: utf-8)".to_string(),
                    required: false,
                    default: Some(Value::String("utf-8".to_string())),
                    constraints: None,
                },
            ],
            category: "filesystem".to_string(),
            requires_confirmation: false,
            security_level: SecurityLevel::Safe,
        }
    }

    async fn execute(&self, parameters: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = parameters.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClaudeError::Validation {
                field: "path".to_string(),
                message: "Path parameter is required".to_string(),
            })?;

        // 安全检查：确保路径在工作目录内
        let full_path = Path::new(&context.working_directory).join(path);
        if !full_path.starts_with(&context.working_directory) {
            return Ok(ToolResult::error("Path traversal not allowed".to_string()));
        }

        match self.fs_manager.read_file(&full_path).await {
            Ok(content) => {
                Ok(ToolResult::success(serde_json::json!({
                    "content": content,
                    "path": path,
                    "size": content.len()
                })))
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to read file: {}", e))),
        }
    }
}

/// 文件写入工具
pub struct WriteTool {
    fs_manager: FileSystemManager,
}

impl WriteTool {
    pub fn new() -> Self {
        Self {
            fs_manager: FileSystemManager::new(vec![std::env::current_dir().unwrap_or_default()]),
        }
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write".to_string(),
            description: "Write content to a file".to_string(),
            version: "1.0.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to the file to write".to_string(),
                    required: true,
                    default: None,
                    constraints: None,
                },
                ToolParameter {
                    name: "content".to_string(),
                    param_type: "string".to_string(),
                    description: "Content to write to the file".to_string(),
                    required: true,
                    default: None,
                    constraints: None,
                },
                ToolParameter {
                    name: "create_dirs".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Create parent directories if they don't exist".to_string(),
                    required: false,
                    default: Some(Value::Bool(false)),
                    constraints: None,
                },
            ],
            category: "filesystem".to_string(),
            requires_confirmation: true,
            security_level: SecurityLevel::Medium,
        }
    }

    async fn execute(&self, parameters: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = parameters.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClaudeError::Validation {
                field: "path".to_string(),
                message: "Path parameter is required".to_string(),
            })?;

        let content = parameters.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClaudeError::Validation {
                field: "content".to_string(),
                message: "Content parameter is required".to_string(),
            })?;

        let create_dirs = parameters.get("create_dirs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 安全检查
        let full_path = Path::new(&context.working_directory).join(path);
        if !full_path.starts_with(&context.working_directory) {
            return Ok(ToolResult::error("Path traversal not allowed".to_string()));
        }

        // 创建父目录（如果需要）
        if create_dirs {
            if let Some(parent) = full_path.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return Ok(ToolResult::error(format!("Failed to create directories: {}", e)));
                }
            }
        }

        match self.fs_manager.write_file(&full_path, content).await {
            Ok(_) => {
                Ok(ToolResult::success(serde_json::json!({
                    "path": path,
                    "bytes_written": content.len(),
                    "success": true
                })))
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to write file: {}", e))),
        }
    }
}

/// 目录列表工具
pub struct ListTool {
    fs_manager: FileSystemManager,
}

impl ListTool {
    pub fn new() -> Self {
        Self {
            fs_manager: FileSystemManager::new(vec![std::env::current_dir().unwrap_or_default()]),
        }
    }
}

#[async_trait]
impl Tool for ListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list".to_string(),
            description: "List files and directories".to_string(),
            version: "1.0.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Path to list (default: current directory)".to_string(),
                    required: false,
                    default: Some(Value::String(".".to_string())),
                    constraints: None,
                },
                ToolParameter {
                    name: "recursive".to_string(),
                    param_type: "boolean".to_string(),
                    description: "List recursively".to_string(),
                    required: false,
                    default: Some(Value::Bool(false)),
                    constraints: None,
                },
                ToolParameter {
                    name: "show_hidden".to_string(),
                    param_type: "boolean".to_string(),
                    description: "Show hidden files".to_string(),
                    required: false,
                    default: Some(Value::Bool(false)),
                    constraints: None,
                },
            ],
            category: "filesystem".to_string(),
            requires_confirmation: false,
            security_level: SecurityLevel::Safe,
        }
    }

    async fn execute(&self, parameters: Value, context: &ToolContext) -> Result<ToolResult> {
        let path = parameters.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let recursive = parameters.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let show_hidden = parameters.get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 安全检查
        let full_path = Path::new(&context.working_directory).join(path);
        if !full_path.starts_with(&context.working_directory) {
            return Ok(ToolResult::error("Path traversal not allowed".to_string()));
        }

        match self.fs_manager.list_directory(&full_path).await {
            Ok(entries) => {
                let filtered_entries: Vec<_> = entries
                    .into_iter()
                    .filter(|entry| {
                        if show_hidden {
                            true
                        } else {
                            // 获取文件名并检查是否以 . 开头
                            if let Some(file_name) = entry.file_name() {
                                if let Some(name_str) = file_name.to_str() {
                                    !name_str.starts_with('.')
                                } else {
                                    true
                                }
                            } else {
                                true
                            }
                        }
                    })
                    .map(|entry| serde_json::json!({
                        "path": entry.to_string_lossy(),
                        "name": entry.file_name().unwrap_or_default().to_string_lossy(),
                        "is_dir": entry.is_dir()
                    }))
                    .collect();

                Ok(ToolResult::success(serde_json::json!({
                    "path": path,
                    "entries": filtered_entries,
                    "count": filtered_entries.len()
                })))
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to list directory: {}", e))),
        }
    }
}

/// Bash 命令执行工具
pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bash".to_string(),
            description: "Execute bash commands".to_string(),
            version: "1.0.0".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "Bash command to execute".to_string(),
                    required: true,
                    default: None,
                    constraints: None,
                },
                ToolParameter {
                    name: "timeout".to_string(),
                    param_type: "number".to_string(),
                    description: "Timeout in seconds (default: 30)".to_string(),
                    required: false,
                    default: Some(Value::Number(serde_json::Number::from(30))),
                    constraints: None,
                },
            ],
            category: "system".to_string(),
            requires_confirmation: true,
            security_level: SecurityLevel::Dangerous,
        }
    }

    async fn execute(&self, parameters: Value, context: &ToolContext) -> Result<ToolResult> {
        let command = parameters.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClaudeError::Validation {
                field: "command".to_string(),
                message: "Command parameter is required".to_string(),
            })?;

        let timeout = parameters.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        // 安全检查：禁止危险命令
        let dangerous_commands = ["rm -rf", "sudo", "su", "chmod 777", "mkfs", "dd"];
        for dangerous in &dangerous_commands {
            if command.contains(dangerous) {
                return Ok(ToolResult::error(format!("Dangerous command not allowed: {}", dangerous)));
            }
        }

        let mut cmd = Command::new("bash");
        cmd.arg("-c")
           .arg(command)
           .current_dir(&context.working_directory);

        // 设置环境变量
        for (key, value) in &context.environment {
            cmd.env(key, value);
        }

        let start_time = std::time::Instant::now();
        
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            cmd.output()
        ).await {
            Ok(Ok(output)) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                Ok(ToolResult::success(serde_json::json!({
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": output.status.code().unwrap_or(-1),
                    "success": output.status.success(),
                    "execution_time_ms": execution_time
                })))
            }
            Ok(Err(e)) => Ok(ToolResult::error(format!("Failed to execute command: {}", e))),
            Err(_) => Ok(ToolResult::error(format!("Command timed out after {} seconds", timeout))),
        }
    }
}

/// 注册所有内置工具
pub async fn register_builtin_tools(registry: &ToolRegistry) -> Result<()> {
    registry.register_tool(Arc::new(ReadTool::new())).await?;
    registry.register_tool(Arc::new(WriteTool::new())).await?;
    registry.register_tool(Arc::new(ListTool::new())).await?;
    registry.register_tool(Arc::new(BashTool)).await?;
    
    tracing::info!("Registered {} builtin tools", 4);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_tool() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        let tool = ReadTool::new();
        let context = ToolContext {
            working_directory: temp_dir.path().to_string_lossy().to_string(),
            ..ToolContext::new("test".to_string())
        };

        let parameters = serde_json::json!({
            "path": "test.txt"
        });

        let result = tool.execute(parameters, &context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["content"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_tool() {
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteTool::new();
        let context = ToolContext {
            working_directory: temp_dir.path().to_string_lossy().to_string(),
            ..ToolContext::new("test".to_string())
        };

        let parameters = serde_json::json!({
            "path": "test.txt",
            "content": "Hello, Rust!"
        });

        let result = tool.execute(parameters, &context).await.unwrap();
        assert!(result.success);

        // 验证文件内容
        let content = tokio::fs::read_to_string(temp_dir.path().join("test.txt")).await.unwrap();
        assert_eq!(content, "Hello, Rust!");
    }
}
