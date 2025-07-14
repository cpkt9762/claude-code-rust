//! 工具调用系统
//!
//! 基于原版 Claude Code 的工具调用机制，实现完整的工具注册、执行和管理系统

pub mod builtin;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use async_trait::async_trait;

use crate::error::{ClaudeError, Result};

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 执行是否成功
    pub success: bool,
    /// 结果数据
    pub data: Value,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 输出日志
    pub logs: Vec<String>,
}

impl ToolResult {
    /// 创建成功结果
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data,
            error: None,
            execution_time_ms: 0,
            logs: Vec::new(),
        }
    }

    /// 创建错误结果
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: Value::Null,
            error: Some(error),
            execution_time_ms: 0,
            logs: Vec::new(),
        }
    }

    /// 添加执行时间
    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.execution_time_ms = time_ms;
        self
    }

    /// 添加日志
    pub fn with_logs(mut self, logs: Vec<String>) -> Self {
        self.logs = logs;
        self
    }
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// 参数名称
    pub name: String,
    /// 参数类型
    pub param_type: String,
    /// 参数描述
    pub description: String,
    /// 是否必需
    pub required: bool,
    /// 默认值
    pub default: Option<Value>,
    /// 参数约束
    pub constraints: Option<Value>,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 工具版本
    pub version: String,
    /// 工具参数
    pub parameters: Vec<ToolParameter>,
    /// 工具类别
    pub category: String,
    /// 是否需要确认
    pub requires_confirmation: bool,
    /// 安全级别
    pub security_level: SecurityLevel,
}

/// 安全级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityLevel {
    /// 安全 - 只读操作
    Safe,
    /// 中等 - 可能修改文件
    Medium,
    /// 危险 - 可能执行系统命令
    Dangerous,
}

/// 工具执行上下文
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// 当前工作目录
    pub working_directory: String,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 用户权限
    pub permissions: Vec<String>,
    /// 会话ID
    pub session_id: String,
    /// 调试模式
    pub debug_mode: bool,
}

impl ToolContext {
    /// 创建新的工具上下文
    pub fn new(session_id: String) -> Self {
        Self {
            working_directory: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            environment: std::env::vars().collect(),
            permissions: vec!["read".to_string(), "write".to_string()],
            session_id,
            debug_mode: false,
        }
    }

    /// 检查权限
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}

/// 工具特征
#[async_trait]
pub trait Tool: Send + Sync {
    /// 获取工具定义
    fn definition(&self) -> ToolDefinition;

    /// 执行工具
    async fn execute(&self, parameters: Value, context: &ToolContext) -> Result<ToolResult>;

    /// 验证参数
    fn validate_parameters(&self, parameters: &Value) -> Result<()> {
        let definition = self.definition();
        
        // 检查必需参数
        for param in &definition.parameters {
            if param.required && !parameters.get(&param.name).is_some() {
                return Err(ClaudeError::Validation {
                    field: param.name.clone(),
                    message: "Required parameter missing".to_string(),
                });
            }
        }
        
        Ok(())
    }

    /// 检查安全性
    fn check_security(&self, context: &ToolContext) -> Result<()> {
        let definition = self.definition();
        
        match definition.security_level {
            SecurityLevel::Safe => Ok(()),
            SecurityLevel::Medium => {
                if context.has_permission("write") {
                    Ok(())
                } else {
                    Err(ClaudeError::Permission {
                        operation: "Medium security tool execution".to_string(),
                    })
                }
            }
            SecurityLevel::Dangerous => {
                if context.has_permission("execute") {
                    Ok(())
                } else {
                    Err(ClaudeError::Permission {
                        operation: "Dangerous tool execution".to_string(),
                    })
                }
            }
        }
    }
}

/// 工具注册表
pub struct ToolRegistry {
    /// 已注册的工具
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    /// 工具使用统计
    usage_stats: Mutex<HashMap<String, ToolUsageStats>>,
}

/// 工具使用统计
#[derive(Debug, Clone, Default)]
pub struct ToolUsageStats {
    /// 调用次数
    pub call_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub error_count: u64,
    /// 总执行时间
    pub total_execution_time_ms: u64,
    /// 平均执行时间
    pub average_execution_time_ms: f64,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            usage_stats: Mutex::new(HashMap::new()),
        }
    }

    /// 注册工具
    pub async fn register_tool(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let definition = tool.definition();
        let mut tools = self.tools.write().await;
        
        if tools.contains_key(&definition.name) {
            return Err(ClaudeError::General(
                format!("Tool '{}' is already registered", definition.name)
            ));
        }
        
        let tool_name = definition.name.clone();
        tools.insert(tool_name.clone(), tool);

        // 初始化统计信息
        let mut stats = self.usage_stats.lock().await;
        stats.insert(tool_name.clone(), ToolUsageStats::default());

        tracing::info!("Registered tool: {}", tool_name);
        Ok(())
    }

    /// 获取工具
    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// 列出所有工具
    pub async fn list_tools(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        let mut definitions = Vec::new();
        
        for tool in tools.values() {
            definitions.push(tool.definition());
        }
        
        definitions.sort_by(|a, b| a.name.cmp(&b.name));
        definitions
    }

    /// 执行工具
    pub async fn execute_tool(
        &self,
        name: &str,
        parameters: Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let tool = self.get_tool(name).await.ok_or_else(|| {
            ClaudeError::General(format!("Tool '{}' not found", name))
        })?;

        // 验证参数
        tool.validate_parameters(&parameters)?;

        // 检查安全性
        tool.check_security(context)?;

        // 记录开始时间
        let start_time = std::time::Instant::now();

        // 执行工具
        let result = tool.execute(parameters, context).await;

        // 计算执行时间
        let execution_time = start_time.elapsed().as_millis() as u64;

        // 更新统计信息
        self.update_stats(name, &result, execution_time).await;

        // 添加执行时间到结果
        match result {
            Ok(mut tool_result) => {
                tool_result.execution_time_ms = execution_time;
                Ok(tool_result)
            }
            Err(e) => {
                let error_result = ToolResult::error(e.to_string())
                    .with_execution_time(execution_time);
                Ok(error_result)
            }
        }
    }

    /// 更新统计信息
    async fn update_stats(&self, tool_name: &str, result: &Result<ToolResult>, execution_time: u64) {
        let mut stats = self.usage_stats.lock().await;
        if let Some(tool_stats) = stats.get_mut(tool_name) {
            tool_stats.call_count += 1;
            tool_stats.total_execution_time_ms += execution_time;
            tool_stats.average_execution_time_ms = 
                tool_stats.total_execution_time_ms as f64 / tool_stats.call_count as f64;

            match result {
                Ok(tool_result) => {
                    if tool_result.success {
                        tool_stats.success_count += 1;
                    } else {
                        tool_stats.error_count += 1;
                    }
                }
                Err(_) => {
                    tool_stats.error_count += 1;
                }
            }
        }
    }

    /// 获取工具统计信息
    pub async fn get_tool_stats(&self, name: &str) -> Option<ToolUsageStats> {
        let stats = self.usage_stats.lock().await;
        stats.get(name).cloned()
    }

    /// 获取所有统计信息
    pub async fn get_all_stats(&self) -> HashMap<String, ToolUsageStats> {
        let stats = self.usage_stats.lock().await;
        stats.clone()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试工具实现
    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                version: "1.0.0".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "input".to_string(),
                        param_type: "string".to_string(),
                        description: "Test input".to_string(),
                        required: true,
                        default: None,
                        constraints: None,
                    }
                ],
                category: "test".to_string(),
                requires_confirmation: false,
                security_level: SecurityLevel::Safe,
            }
        }

        async fn execute(&self, parameters: Value, _context: &ToolContext) -> Result<ToolResult> {
            let input = parameters.get("input")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            
            Ok(ToolResult::success(serde_json::json!({
                "output": format!("Processed: {}", input)
            })))
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(TestTool);
        
        // 注册工具
        registry.register_tool(tool).await.unwrap();
        
        // 列出工具
        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
        
        // 执行工具
        let context = ToolContext::new("test-session".to_string());
        let parameters = serde_json::json!({"input": "test"});
        let result = registry.execute_tool("test_tool", parameters, &context).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.data["output"], "Processed: test");
    }

    #[tokio::test]
    async fn test_tool_validation() {
        let tool = TestTool;
        
        // 测试有效参数
        let valid_params = serde_json::json!({"input": "test"});
        assert!(tool.validate_parameters(&valid_params).is_ok());
        
        // 测试缺失必需参数
        let invalid_params = serde_json::json!({});
        assert!(tool.validate_parameters(&invalid_params).is_err());
    }
}
