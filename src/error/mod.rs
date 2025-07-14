//! 错误处理模块
//! 
//! 定义统一的错误类型和处理机制

use thiserror::Error;
use std::error::Error;

/// Claude Code 的主要错误类型
#[derive(Error, Debug)]
pub enum ClaudeError {
    /// 配置错误
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// 文件系统错误
    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),

    /// 网络错误
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON 序列化/反序列化错误
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML 序列化/反序列化错误
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// MCP 服务器错误
    #[error("MCP server error: {message}")]
    McpServer { message: String },

    /// 权限错误
    #[error("Permission denied: {operation}")]
    Permission { operation: String },

    /// 验证错误
    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    /// 通用错误
    #[error("General error: {0}")]
    General(String),

    /// 未实现的功能
    #[error("Feature not implemented: {feature}")]
    NotImplemented { feature: String },
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, ClaudeError>;

/// 错误处理工具函数
impl ClaudeError {
    /// 创建配置错误
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::General(format!("Configuration error: {}", msg.into()))
    }

    /// 创建网络错误
    pub fn network_error(msg: impl Into<String>) -> Self {
        Self::General(format!("Network error: {}", msg.into()))
    }

    /// 创建文件系统错误
    pub fn fs_error(msg: impl Into<String>) -> Self {
        Self::General(format!("File system error: {}", msg.into()))
    }

    /// 创建 MCP 错误
    pub fn mcp_error(msg: impl Into<String>) -> Self {
        Self::McpServer {
            message: msg.into(),
        }
    }

    /// 创建权限错误
    pub fn permission_error(operation: impl Into<String>) -> Self {
        Self::Permission {
            operation: operation.into(),
        }
    }

    /// 创建验证错误
    pub fn validation_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// 创建未实现错误
    pub fn not_implemented(feature: impl Into<String>) -> Self {
        Self::NotImplemented {
            feature: feature.into(),
        }
    }
}

impl Clone for ClaudeError {
    fn clone(&self) -> Self {
        match self {
            Self::Config(_) => Self::General("Configuration error".to_string()),
            Self::Io(_) => Self::General("IO error".to_string()),
            Self::Network(_) => Self::General("Network error".to_string()),
            Self::Json(_) => Self::General("JSON error".to_string()),
            Self::Yaml(_) => Self::General("YAML error".to_string()),
            Self::General(msg) => Self::General(msg.clone()),
            Self::Permission { operation } => Self::Permission { operation: operation.clone() },
            Self::Validation { field, message } => Self::Validation {
                field: field.clone(),
                message: message.clone()
            },
            Self::NotImplemented { feature } => Self::NotImplemented { feature: feature.clone() },
            Self::McpServer { message } => Self::McpServer { message: message.clone() },
        }
    }
}

/// 初始化日志系统
pub fn init_logging(debug: bool) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};
    use std::io;

    let filter = if debug {
        "claude_code_rust=debug,info"
    } else {
        "claude_code_rust=info,warn"
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| filter.into());

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(debug)
        .with_thread_names(debug)
        .with_file(debug)
        .with_line_number(debug)
        .compact();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

/// 初始化带文件输出的日志系统
pub fn init_logging_with_file(debug: bool, log_file: Option<&str>) -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};
    use tracing_appender::{rolling, non_blocking};
    use std::path::Path;

    let filter = if debug {
        "claude_code_rust=debug,info"
    } else {
        "claude_code_rust=info,warn"
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| filter.into());

    // 控制台输出
    let console_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(debug)
        .with_thread_names(debug)
        .with_file(debug)
        .with_line_number(debug)
        .compact();

    // 文件输出（如果指定）
    if let Some(log_path) = log_file {
        let log_dir = Path::new(log_path).parent().unwrap_or(Path::new("."));
        let log_name = Path::new(log_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("claude-code");

        let file_appender = rolling::daily(log_dir, log_name);
        let (non_blocking, _guard) = non_blocking(file_appender);

        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .json();

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .init();
    }

    Ok(())
}

/// 错误报告工具
pub fn report_error(error: &ClaudeError) {
    tracing::error!("Claude Code Error: {}", error);
    
    // 在调试模式下显示错误链
    let mut source = error.source();
    while let Some(err) = source {
        tracing::debug!("Caused by: {}", err);
        source = err.source();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ClaudeError::config_error("test config error");
        assert!(error.to_string().contains("Configuration error"));

        let error = ClaudeError::validation_error("field1", "invalid value");
        assert!(error.to_string().contains("Validation error"));
    }
}
