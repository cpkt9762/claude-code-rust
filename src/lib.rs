//! Claude Rust - A Rust implementation of Claude Code
//! 
//! This library provides a comprehensive set of tools for agentic coding,
//! including file system management, Git operations, syntax highlighting,
//! memory management, and more.

pub mod agent;
pub mod cli;
pub mod config;
pub mod context;
pub mod conversation;
pub mod cost;
pub mod error;
pub mod fs;
pub mod git;
pub mod mcp;
pub mod network;
pub mod plugins;
pub mod process;
pub mod refactor;
pub mod security;
pub mod steering;
pub mod streaming;
pub mod tools;
pub mod ui;
pub mod watcher;
pub mod web;

#[cfg(feature = "image-processing")]
pub mod image_processing;

#[cfg(feature = "syntax-highlighting")]
pub mod syntax_highlighting;

// Re-export commonly used types
pub use agent::{AgentLoop, AgentContext, AgentStatus, AgentResponse};
pub use context::{ContextManager, CompressedContext, ContextStats};
pub use error::{ClaudeError, Result};
pub use config::{ClaudeConfig, ConfigManager};
pub use fs::FileSystemManager;
pub use git::GitManager;
pub use steering::{SteeringController, SteeringSession, AsyncMessageQueue};
pub use tools::{Tool, ToolRegistry, ToolResult, ToolDefinition, ToolContext};
pub use ui::TerminalUI;

#[cfg(feature = "syntax-highlighting")]
pub use syntax_highlighting::SyntaxHighlighter;

#[cfg(feature = "image-processing")]
pub use image_processing::ImageProcessor;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Library description
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn test_name() {
        assert!(!NAME.is_empty());
        assert_eq!(NAME, "claude-code-rust");
    }

    #[test]
    fn test_description() {
        assert!(!DESCRIPTION.is_empty());
    }
}
