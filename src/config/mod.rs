//! 配置管理模块
//! 
//! 处理配置文件读写、环境变量和用户设置

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::env;
use tokio::fs;

use crate::error::{ClaudeError, Result};

/// Claude Code 主配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    /// API 配置
    pub api: ApiConfig,
    /// MCP 服务器配置
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    /// UI 配置
    #[serde(default)]
    pub ui: UiConfig,
    /// 权限配置
    #[serde(default)]
    pub permissions: PermissionConfig,
    /// 工作目录
    #[serde(default)]
    pub working_dirs: Vec<PathBuf>,
    /// 内存设置
    #[serde(default)]
    pub memory: MemoryConfig,
    /// 日志配置
    #[serde(default)]
    pub logging: LoggingConfig,
    /// 性能配置
    #[serde(default)]
    pub performance: PerformanceConfig,
    /// 用户偏好
    #[serde(default)]
    pub preferences: UserPreferences,
    /// AI 模型设置
    #[serde(default)]
    pub model: Option<String>,
}

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Anthropic API 密钥
    pub anthropic_api_key: Option<String>,
    /// API 基础 URL
    #[serde(default = "default_api_base_url")]
    pub base_url: String,
    /// 默认模型
    #[serde(default = "default_model")]
    pub default_model: String,
    /// 最大 tokens
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 温度参数
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Top-p 参数
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    /// Top-k 参数
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    /// 请求超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 是否启用流式响应
    #[serde(default = "default_stream")]
    pub stream: bool,
    /// API 版本
    #[serde(default = "default_api_version")]
    pub api_version: String,
}

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// 服务器名称
    pub name: String,
    /// 执行命令
    pub command: String,
    /// 命令参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: HashMap<String, String>,
    /// 工作目录
    pub working_dir: Option<PathBuf>,
    /// 是否自动启动
    pub auto_start: bool,
}

/// UI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// 主题
    pub theme: String,
    /// 是否启用 Vim 模式
    pub vim_mode: bool,
    /// 终端宽度
    pub terminal_width: Option<u16>,
    /// 是否显示行号
    pub show_line_numbers: bool,
    /// 是否启用TUI模式
    #[serde(default)]
    pub enable_tui: bool,
}

/// 权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// 允许的工具
    pub allowed_tools: Vec<String>,
    /// 拒绝的工具
    pub denied_tools: Vec<String>,
    /// 是否需要确认
    pub require_confirmation: bool,
}

/// 内存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// 最大内存条目数
    pub max_entries: usize,
    /// 内存文件路径
    pub memory_file: Option<PathBuf>,
    /// 是否自动保存
    pub auto_save: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api: ApiConfig::default(),
            mcp_servers: HashMap::new(),
            ui: UiConfig::default(),
            permissions: PermissionConfig::default(),
            working_dirs: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            memory: MemoryConfig::default(),
            logging: LoggingConfig::default(),
            performance: PerformanceConfig::default(),
            preferences: UserPreferences::default(),
            model: None,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            base_url: default_api_base_url(),
            default_model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            top_p: default_top_p(),
            top_k: default_top_k(),
            timeout: default_timeout(),
            max_retries: default_max_retries(),
            stream: default_stream(),
            api_version: default_api_version(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            vim_mode: false,
            terminal_width: None,
            show_line_numbers: true,
            enable_tui: false,
        }
    }
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            allowed_tools: vec![
                "file_read".to_string(),
                "file_write".to_string(),
                "network_request".to_string(),
            ],
            denied_tools: vec![],
            require_confirmation: true,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            memory_file: None,
            auto_save: true,
        }
    }
}

/// 配置管理器
pub struct ConfigManager {
    config: ClaudeConfig,
    config_path: PathBuf,
    config_format: ConfigFormat,
}

/// 配置文件格式
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
    Rc, // .clauderc 格式（类似 .bashrc）
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        let config = Self::load_config(&config_path)?;
        
        Ok(Self {
            config,
            config_path: config_path.clone(),
            config_format: Self::detect_format(&config_path)?,
        })
    }

    /// 从指定路径创建配置管理器
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let config = Self::load_config(&path)?;
        
        Ok(Self {
            config,
            config_path: path.clone(),
            config_format: Self::detect_format(&path)?,
        })
    }

    /// 获取配置文件路径
    fn get_config_path() -> Result<PathBuf> {
        // 查找现有配置文件
        if let Ok((path, _)) = Self::find_config_file() {
            return Ok(path);
        }

        // 如果没有找到，创建默认配置文件路径
        let config_dir = dirs::config_dir()
            .ok_or_else(|| ClaudeError::config_error("Cannot find config directory"))?;

        let claude_dir = config_dir.join("claude-code");
        std::fs::create_dir_all(&claude_dir)?;

        Ok(claude_dir.join("config.yaml"))
    }

    /// 查找配置文件
    fn find_config_file() -> Result<(PathBuf, ConfigFormat)> {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());

        // 配置文件搜索顺序
        let config_candidates = vec![
            // 当前目录
            ("./claude.json".to_string(), ConfigFormat::Json),
            ("./claude.yaml".to_string(), ConfigFormat::Yaml),
            ("./claude.yml".to_string(), ConfigFormat::Yaml),
            ("./claude.toml".to_string(), ConfigFormat::Toml),
            ("./.clauderc".to_string(), ConfigFormat::Rc),
            // 用户主目录
            (format!("{}/.claude/config.json", home), ConfigFormat::Json),
            (format!("{}/.claude/config.yaml", home), ConfigFormat::Yaml),
            (format!("{}/.claude/config.yml", home), ConfigFormat::Yaml),
            (format!("{}/.claude/config.toml", home), ConfigFormat::Toml),
            (format!("{}/.clauderc", home), ConfigFormat::Rc),
            // XDG 配置目录
            (format!("{}/.config/claude/config.json", home), ConfigFormat::Json),
            (format!("{}/.config/claude/config.yaml", home), ConfigFormat::Yaml),
            (format!("{}/.config/claude/config.toml", home), ConfigFormat::Toml),
        ];

        for (path_str, format) in config_candidates {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Ok((path, format));
            }
        }

        Err(ClaudeError::General("No config file found".to_string()))
    }

    /// 检测配置文件格式
    fn detect_format(path: &Path) -> Result<ConfigFormat> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "json" => Ok(ConfigFormat::Json),
                "yaml" | "yml" => Ok(ConfigFormat::Yaml),
                "toml" => Ok(ConfigFormat::Toml),
                _ => {
                    if path.file_name().and_then(|n| n.to_str()) == Some(".clauderc") {
                        Ok(ConfigFormat::Rc)
                    } else {
                        Err(ClaudeError::General(format!(
                            "Unsupported config file format: {}", ext
                        )))
                    }
                }
            }
        } else {
            // 没有扩展名，检查文件名
            if path.file_name().and_then(|n| n.to_str()) == Some(".clauderc") {
                Ok(ConfigFormat::Rc)
            } else {
                Err(ClaudeError::General(
                    "Cannot detect config file format".to_string()
                ))
            }
        }
    }

    /// 加载配置文件
    fn load_config(path: &PathBuf) -> Result<ClaudeConfig> {
        if path.exists() {
            let format = Self::detect_format(path)?;
            Self::load_config_file(path, &format)
        } else {
            // 创建默认配置文件
            let config = ClaudeConfig::default();
            let format = Self::detect_format(path).unwrap_or(ConfigFormat::Yaml);
            Self::save_config_file(&config, path, &format)?;
            Ok(config)
        }
    }

    /// 加载指定格式的配置文件
    fn load_config_file(path: &Path, format: &ConfigFormat) -> Result<ClaudeConfig> {
        let content = std::fs::read_to_string(path)?;

        let config = match format {
            ConfigFormat::Json => {
                serde_json::from_str(&content)
                    .map_err(|e| ClaudeError::General(format!("JSON parse error: {}", e)))?
            }
            ConfigFormat::Yaml => {
                serde_yaml::from_str(&content)
                    .map_err(|e| ClaudeError::General(format!("YAML parse error: {}", e)))?
            }
            ConfigFormat::Toml => {
                toml::from_str(&content)
                    .map_err(|e| ClaudeError::General(format!("TOML parse error: {}", e)))?
            }
            ConfigFormat::Rc => {
                Self::parse_rc_format(&content)?
            }
        };

        Ok(config)
    }

    /// 解析 .clauderc 格式
    fn parse_rc_format(content: &str) -> Result<ClaudeConfig> {
        let mut config = ClaudeConfig::default();

        for line in content.lines() {
            let line = line.trim();

            // 跳过注释和空行
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 解析 key=value 格式
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "ANTHROPIC_API_KEY" => config.api.anthropic_api_key = Some(value.to_string()),
                    "API_BASE_URL" => config.api.base_url = value.to_string(),
                    "DEFAULT_MODEL" => config.api.default_model = value.to_string(),
                    "MAX_TOKENS" => config.api.max_tokens = value.parse().unwrap_or(4096),
                    "TEMPERATURE" => config.api.temperature = value.parse().unwrap_or(0.7),
                    "STREAM" => config.api.stream = value.parse().unwrap_or(true),
                    "LOG_LEVEL" => config.logging.level = value.to_string(),
                    "EDITOR" => config.preferences.editor = Some(value.to_string()),
                    "SHELL" => config.preferences.shell = Some(value.to_string()),
                    _ => {
                        // 忽略未知的配置项
                        eprintln!("Warning: Unknown config key: {}", key);
                    }
                }
            }
        }

        Ok(config)
    }

    /// 保存配置文件
    pub fn save(&self) -> Result<()> {
        Self::save_config_file(&self.config, &self.config_path, &self.config_format)
    }

    /// 保存指定格式的配置文件
    fn save_config_file(config: &ClaudeConfig, path: &Path, format: &ConfigFormat) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = match format {
            ConfigFormat::Json => {
                serde_json::to_string_pretty(config)
                    .map_err(|e| ClaudeError::General(format!("JSON serialize error: {}", e)))?
            }
            ConfigFormat::Yaml => {
                serde_yaml::to_string(config)
                    .map_err(|e| ClaudeError::General(format!("YAML serialize error: {}", e)))?
            }
            ConfigFormat::Toml => {
                toml::to_string_pretty(config)
                    .map_err(|e| ClaudeError::General(format!("TOML serialize error: {}", e)))?
            }
            ConfigFormat::Rc => {
                Self::serialize_rc_format(config)?
            }
        };

        std::fs::write(path, content)?;
        Ok(())
    }

    /// 序列化为 .clauderc 格式
    fn serialize_rc_format(config: &ClaudeConfig) -> Result<String> {
        let mut lines = vec![
            "# Claude Code Configuration".to_string(),
            "# This file is automatically generated".to_string(),
            "".to_string(),
        ];

        // API 配置
        lines.push("# API Configuration".to_string());
        if let Some(ref api_key) = config.api.anthropic_api_key {
            lines.push(format!("ANTHROPIC_API_KEY=\"{}\"", api_key));
        }
        lines.push(format!("API_BASE_URL=\"{}\"", config.api.base_url));
        lines.push(format!("DEFAULT_MODEL=\"{}\"", config.api.default_model));
        lines.push(format!("MAX_TOKENS={}", config.api.max_tokens));
        lines.push(format!("TEMPERATURE={}", config.api.temperature));
        lines.push(format!("STREAM={}", config.api.stream));
        lines.push("".to_string());

        // 日志配置
        lines.push("# Logging Configuration".to_string());
        lines.push(format!("LOG_LEVEL=\"{}\"", config.logging.level));
        lines.push("".to_string());

        // 用户偏好
        lines.push("# User Preferences".to_string());
        if let Some(ref editor) = config.preferences.editor {
            lines.push(format!("EDITOR=\"{}\"", editor));
        }
        if let Some(ref shell) = config.preferences.shell {
            lines.push(format!("SHELL=\"{}\"", shell));
        }

        Ok(lines.join("\n"))
    }

    /// 获取配置
    pub fn get_config(&self) -> &ClaudeConfig {
        &self.config
    }

    /// 获取可变配置
    pub fn get_config_mut(&mut self) -> &mut ClaudeConfig {
        &mut self.config
    }

    /// 设置配置值
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            // API 配置
            "api.anthropic_api_key" => self.config.api.anthropic_api_key = Some(value.to_string()),
            "api.base_url" => self.config.api.base_url = value.to_string(),
            "api.default_model" => self.config.api.default_model = value.to_string(),
            "api.max_tokens" => self.config.api.max_tokens = value.parse().unwrap_or(4096),
            "api.temperature" => self.config.api.temperature = value.parse().unwrap_or(0.7),
            "api.top_p" => self.config.api.top_p = value.parse().unwrap_or(0.9),
            "api.top_k" => self.config.api.top_k = value.parse().unwrap_or(40),
            "api.timeout" => self.config.api.timeout = value.parse().unwrap_or(30),
            "api.stream" => self.config.api.stream = value.parse().unwrap_or(true),

            // UI 配置
            "ui.theme" => self.config.ui.theme = value.to_string(),
            "ui.vim_mode" => self.config.ui.vim_mode = value.parse().unwrap_or(false),

            // 日志配置
            "logging.level" => self.config.logging.level = value.to_string(),
            "logging.console" => self.config.logging.console = value.parse().unwrap_or(true),
            "logging.structured" => self.config.logging.structured = value.parse().unwrap_or(false),

            // 性能配置
            "performance.max_concurrent_requests" => {
                self.config.performance.max_concurrent_requests = value.parse().unwrap_or(10);
            }
            "performance.cache_size_mb" => {
                self.config.performance.cache_size_mb = value.parse().unwrap_or(100);
            }
            "performance.enable_monitoring" => {
                self.config.performance.enable_monitoring = value.parse().unwrap_or(false);
            }

            // 用户偏好
            "preferences.editor" => self.config.preferences.editor = Some(value.to_string()),
            "preferences.shell" => self.config.preferences.shell = Some(value.to_string()),
            "preferences.enable_autocomplete" => {
                self.config.preferences.enable_autocomplete = value.parse().unwrap_or(true);
            }
            "preferences.enable_syntax_highlighting" => {
                self.config.preferences.enable_syntax_highlighting = value.parse().unwrap_or(true);
            }

            // 代码风格
            "preferences.code_style.indent_size" => {
                self.config.preferences.code_style.indent_size = value.parse().unwrap_or(4);
            }
            "preferences.code_style.use_tabs" => {
                self.config.preferences.code_style.use_tabs = value.parse().unwrap_or(false);
            }
            "preferences.code_style.max_line_length" => {
                self.config.preferences.code_style.max_line_length = value.parse().unwrap_or(100);
            }
            "preferences.code_style.auto_format" => {
                self.config.preferences.code_style.auto_format = value.parse().unwrap_or(true);
            }

            _ => return Err(ClaudeError::validation_error("key", "Unknown configuration key")),
        }
        Ok(())
    }

    /// 获取配置值
    pub fn get_value(&self, key: &str) -> Result<String> {
        let value = match key {
            // API 配置
            "api.anthropic_api_key" => self.config.api.anthropic_api_key.as_deref().unwrap_or("").to_string(),
            "api.base_url" => self.config.api.base_url.clone(),
            "api.default_model" => self.config.api.default_model.clone(),
            "api.max_tokens" => self.config.api.max_tokens.to_string(),
            "api.temperature" => self.config.api.temperature.to_string(),
            "api.top_p" => self.config.api.top_p.to_string(),
            "api.top_k" => self.config.api.top_k.to_string(),
            "api.timeout" => self.config.api.timeout.to_string(),
            "api.stream" => self.config.api.stream.to_string(),

            // UI 配置
            "ui.theme" => self.config.ui.theme.clone(),
            "ui.vim_mode" => self.config.ui.vim_mode.to_string(),

            // 日志配置
            "logging.level" => self.config.logging.level.clone(),
            "logging.console" => self.config.logging.console.to_string(),
            "logging.structured" => self.config.logging.structured.to_string(),

            // 性能配置
            "performance.max_concurrent_requests" => self.config.performance.max_concurrent_requests.to_string(),
            "performance.cache_size_mb" => self.config.performance.cache_size_mb.to_string(),
            "performance.enable_monitoring" => self.config.performance.enable_monitoring.to_string(),

            // 用户偏好
            "preferences.editor" => self.config.preferences.editor.as_deref().unwrap_or("").to_string(),
            "preferences.shell" => self.config.preferences.shell.as_deref().unwrap_or("").to_string(),
            "preferences.enable_autocomplete" => self.config.preferences.enable_autocomplete.to_string(),
            "preferences.enable_syntax_highlighting" => self.config.preferences.enable_syntax_highlighting.to_string(),

            // 代码风格
            "preferences.code_style.indent_size" => self.config.preferences.code_style.indent_size.to_string(),
            "preferences.code_style.use_tabs" => self.config.preferences.code_style.use_tabs.to_string(),
            "preferences.code_style.max_line_length" => self.config.preferences.code_style.max_line_length.to_string(),
            "preferences.code_style.auto_format" => self.config.preferences.code_style.auto_format.to_string(),

            _ => return Err(ClaudeError::validation_error("key", "Unknown configuration key")),
        };
        Ok(value)
    }

    /// 从环境变量加载配置
    pub fn load_from_env(&mut self) -> Result<()> {
        if let Ok(api_key) = env::var("ANTHROPIC_API_KEY") {
            self.config.api.anthropic_api_key = Some(api_key);
        }

        if let Ok(base_url) = env::var("ANTHROPIC_BASE_URL") {
            self.config.api.base_url = base_url;
        }

        if let Ok(model) = env::var("CLAUDE_DEFAULT_MODEL") {
            self.config.api.default_model = model;
        }

        if let Ok(log_level) = env::var("CLAUDE_LOG_LEVEL") {
            self.config.logging.level = log_level;
        }

        if let Ok(editor) = env::var("EDITOR") {
            self.config.preferences.editor = Some(editor);
        }

        if let Ok(shell) = env::var("SHELL") {
            self.config.preferences.shell = Some(shell);
        }

        Ok(())
    }

    /// 创建示例配置文件
    pub async fn create_example_config(path: &Path, format: ConfigFormat) -> Result<()> {
        let mut config = ClaudeConfig::default();

        // 设置一些示例值
        config.api.anthropic_api_key = Some("your-api-key-here".to_string());
        config.preferences.editor = Some("code".to_string());
        config.preferences.shell = Some("/bin/zsh".to_string());

        Self::save_config_file(&config, path, &format)?;

        println!("Created example config file: {}", path.display());
        println!("Please edit the file to set your actual API key and preferences.");

        Ok(())
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证 API 配置
        if self.config.api.anthropic_api_key.is_none() {
            return Err(ClaudeError::validation_error(
                "api.anthropic_api_key",
                "API key is required"
            ));
        }

        // 验证模型名称
        if self.config.api.default_model.is_empty() {
            return Err(ClaudeError::validation_error(
                "api.default_model",
                "Default model cannot be empty"
            ));
        }

        // 验证数值范围
        if self.config.api.temperature < 0.0 || self.config.api.temperature > 1.0 {
            return Err(ClaudeError::validation_error(
                "api.temperature",
                "Temperature must be between 0.0 and 1.0"
            ));
        }

        if self.config.api.top_p < 0.0 || self.config.api.top_p > 1.0 {
            return Err(ClaudeError::validation_error(
                "api.top_p",
                "Top-p must be between 0.0 and 1.0"
            ));
        }

        Ok(())
    }
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别
    #[serde(default = "default_log_level")]
    pub level: String,
    /// 日志文件路径
    pub file: Option<PathBuf>,
    /// 是否启用控制台输出
    #[serde(default = "default_console_output")]
    pub console: bool,
    /// 是否启用结构化日志
    #[serde(default = "default_structured")]
    pub structured: bool,
    /// 日志格式
    #[serde(default = "default_log_format")]
    pub format: String,
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 最大并发请求数
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: u32,
    /// 缓存大小（MB）
    #[serde(default = "default_cache_size")]
    pub cache_size_mb: u32,
    /// 是否启用性能监控
    #[serde(default = "default_monitoring")]
    pub enable_monitoring: bool,
    /// 性能指标收集间隔（秒）
    #[serde(default = "default_metrics_interval")]
    pub metrics_interval: u64,
}

/// 用户偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// 默认编辑器
    pub editor: Option<String>,
    /// 默认 shell
    pub shell: Option<String>,
    /// 自动保存间隔（秒）
    #[serde(default = "default_autosave_interval")]
    pub autosave_interval: u64,
    /// 是否启用自动完成
    #[serde(default = "default_autocomplete")]
    pub enable_autocomplete: bool,
    /// 是否启用语法高亮
    #[serde(default = "default_syntax_highlighting")]
    pub enable_syntax_highlighting: bool,
    /// 代码风格偏好
    #[serde(default)]
    pub code_style: CodeStyleConfig,
}

/// 代码风格配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStyleConfig {
    /// 缩进大小
    #[serde(default = "default_indent_size")]
    pub indent_size: u32,
    /// 是否使用制表符
    #[serde(default = "default_use_tabs")]
    pub use_tabs: bool,
    /// 行长度限制
    #[serde(default = "default_line_length")]
    pub max_line_length: u32,
    /// 是否自动格式化
    #[serde(default = "default_auto_format")]
    pub auto_format: bool,
}

// 默认值函数
fn default_api_base_url() -> String {
    "https://api.anthropic.com".to_string()
}

fn default_model() -> String {
    "claude-3-haiku-20240307".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

fn default_top_p() -> f32 {
    0.9
}

fn default_top_k() -> u32 {
    40
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

fn default_stream() -> bool {
    true
}

fn default_api_version() -> String {
    "2023-06-01".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_console_output() -> bool {
    true
}

fn default_structured() -> bool {
    false
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_max_concurrent() -> u32 {
    10
}

fn default_cache_size() -> u32 {
    100
}

fn default_monitoring() -> bool {
    false
}

fn default_metrics_interval() -> u64 {
    60
}

fn default_autosave_interval() -> u64 {
    300
}

fn default_autocomplete() -> bool {
    true
}

fn default_syntax_highlighting() -> bool {
    true
}

fn default_indent_size() -> u32 {
    4
}

fn default_use_tabs() -> bool {
    false
}

fn default_line_length() -> u32 {
    100
}

fn default_auto_format() -> bool {
    true
}

// Default 实现
impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            console: default_console_output(),
            structured: default_structured(),
            format: default_log_format(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: default_max_concurrent(),
            cache_size_mb: default_cache_size(),
            enable_monitoring: default_monitoring(),
            metrics_interval: default_metrics_interval(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            editor: None,
            shell: None,
            autosave_interval: default_autosave_interval(),
            enable_autocomplete: default_autocomplete(),
            enable_syntax_highlighting: default_syntax_highlighting(),
            code_style: CodeStyleConfig::default(),
        }
    }
}

impl Default for CodeStyleConfig {
    fn default() -> Self {
        Self {
            indent_size: default_indent_size(),
            use_tabs: default_use_tabs(),
            max_line_length: default_line_length(),
            auto_format: default_auto_format(),
        }
    }
}
