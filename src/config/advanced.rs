use crate::error::{ClaudeError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn, error};

/// 高级配置管理器
pub struct AdvancedConfigManager {
    /// 配置文件路径
    config_path: PathBuf,
    /// 配置模式（开发、测试、生产）
    environment: Environment,
    /// 配置缓存
    config_cache: Option<AdvancedConfig>,
    /// 配置监听器
    watchers: Vec<Box<dyn ConfigWatcher>>,
    /// 配置验证器
    validators: Vec<Box<dyn ConfigValidator>>,
}

/// 环境类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
}

/// 高级配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// 基础配置
    pub base: BaseConfig,
    /// 环境特定配置
    pub environment: EnvironmentConfig,
    /// 功能开关
    pub features: FeatureFlags,
    /// 性能配置
    pub performance: PerformanceConfig,
    /// 安全配置
    pub security: SecurityConfig,
    /// 监控配置
    pub monitoring: MonitoringConfig,
    /// 实验性功能
    pub experiments: ExperimentConfig,
}

/// 基础配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    /// 应用名称
    pub app_name: String,
    /// 版本
    pub version: String,
    /// 日志级别
    pub log_level: String,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 临时目录
    pub temp_dir: PathBuf,
}

/// 环境特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// API 端点
    pub api_endpoints: HashMap<String, String>,
    /// 数据库配置
    pub database: Option<DatabaseConfig>,
    /// 缓存配置
    pub cache: CacheConfig,
    /// 网络配置
    pub network: NetworkConfig,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub timeout_seconds: u64,
    pub ssl_mode: String,
}

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size_mb: u64,
    pub ttl_seconds: u64,
    pub compression: bool,
}

/// 网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
    pub max_concurrent_requests: u32,
}

/// 功能开关
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Web 界面
    pub web_ui: bool,
    /// 实时协作
    pub real_time_collaboration: bool,
    /// 高级分析
    pub advanced_analytics: bool,
    /// 自动备份
    pub auto_backup: bool,
    /// 插件系统
    pub plugin_system: bool,
    /// 自定义功能
    pub custom_features: HashMap<String, bool>,
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 最大内存使用（MB）
    pub max_memory_mb: u64,
    /// 工作线程数
    pub worker_threads: Option<usize>,
    /// 异步任务队列大小
    pub async_queue_size: usize,
    /// 垃圾回收配置
    pub gc_config: GcConfig,
}

/// 垃圾回收配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub memory_threshold_mb: u64,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 启用 HTTPS
    pub enable_https: bool,
    /// 证书路径
    pub cert_path: Option<PathBuf>,
    /// 私钥路径
    pub key_path: Option<PathBuf>,
    /// 允许的主机
    pub allowed_hosts: Vec<String>,
    /// API 密钥加密
    pub encrypt_api_keys: bool,
    /// 会话超时（秒）
    pub session_timeout_seconds: u64,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 启用指标收集
    pub enable_metrics: bool,
    /// 指标导出端口
    pub metrics_port: Option<u16>,
    /// 健康检查间隔
    pub health_check_interval_seconds: u64,
    /// 日志聚合
    pub log_aggregation: LogAggregationConfig,
}

/// 日志聚合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAggregationConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub batch_size: usize,
    pub flush_interval_seconds: u64,
}

/// 实验性功能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// A/B 测试
    pub ab_tests: HashMap<String, ABTestConfig>,
    /// 功能门控
    pub feature_gates: HashMap<String, FeatureGateConfig>,
}

/// A/B 测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    pub enabled: bool,
    pub variant_a_percentage: f64,
    pub variant_b_percentage: f64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 功能门控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureGateConfig {
    pub enabled: bool,
    pub rollout_percentage: f64,
    pub user_groups: Vec<String>,
}

/// 配置监听器 trait
pub trait ConfigWatcher: Send + Sync {
    fn on_config_changed(&self, old_config: &AdvancedConfig, new_config: &AdvancedConfig);
}

/// 配置验证器 trait
pub trait ConfigValidator: Send + Sync {
    fn validate(&self, config: &AdvancedConfig) -> Result<()>;
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            base: BaseConfig {
                app_name: "claude-code-rust".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                log_level: "info".to_string(),
                data_dir: PathBuf::from("./data"),
                temp_dir: PathBuf::from("./tmp"),
            },
            environment: EnvironmentConfig {
                api_endpoints: HashMap::new(),
                database: None,
                cache: CacheConfig {
                    enabled: true,
                    max_size_mb: 100,
                    ttl_seconds: 3600,
                    compression: true,
                },
                network: NetworkConfig {
                    timeout_seconds: 30,
                    retry_attempts: 3,
                    retry_delay_ms: 1000,
                    max_concurrent_requests: 100,
                },
            },
            features: FeatureFlags {
                web_ui: true,
                real_time_collaboration: false,
                advanced_analytics: false,
                auto_backup: true,
                plugin_system: true,
                custom_features: HashMap::new(),
            },
            performance: PerformanceConfig {
                max_memory_mb: 512,
                worker_threads: None,
                async_queue_size: 1000,
                gc_config: GcConfig {
                    enabled: true,
                    interval_seconds: 300,
                    memory_threshold_mb: 400,
                },
            },
            security: SecurityConfig {
                enable_https: false,
                cert_path: None,
                key_path: None,
                allowed_hosts: vec!["localhost".to_string(), "127.0.0.1".to_string()],
                encrypt_api_keys: true,
                session_timeout_seconds: 3600,
            },
            monitoring: MonitoringConfig {
                enable_metrics: true,
                metrics_port: Some(9090),
                health_check_interval_seconds: 30,
                log_aggregation: LogAggregationConfig {
                    enabled: false,
                    endpoint: None,
                    batch_size: 100,
                    flush_interval_seconds: 60,
                },
            },
            experiments: ExperimentConfig {
                ab_tests: HashMap::new(),
                feature_gates: HashMap::new(),
            },
        }
    }
}

impl AdvancedConfigManager {
    /// 创建新的高级配置管理器
    pub fn new(config_path: PathBuf, environment: Environment) -> Self {
        Self {
            config_path,
            environment,
            config_cache: None,
            watchers: Vec::new(),
            validators: Vec::new(),
        }
    }

    /// 加载配置
    pub async fn load_config(&mut self) -> Result<&AdvancedConfig> {
        let config = if self.config_path.exists() {
            self.load_from_file().await?
        } else {
            info!("Config file not found, using defaults");
            AdvancedConfig::default()
        };

        // 验证配置
        self.validate_config(&config)?;

        self.config_cache = Some(config);
        Ok(self.config_cache.as_ref().unwrap())
    }

    /// 从文件加载配置
    async fn load_from_file(&self) -> Result<AdvancedConfig> {
        let content = fs::read_to_string(&self.config_path).await
            .map_err(|e| ClaudeError::io_error(&format!("Failed to read config file: {}", e)))?;

        let config: AdvancedConfig = match self.config_path.extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content)
                    .map_err(|e| ClaudeError::config_error(&format!("Invalid YAML config: {}", e)))?
            }
            Some("json") => {
                serde_json::from_str(&content)
                    .map_err(|e| ClaudeError::config_error(&format!("Invalid JSON config: {}", e)))?
            }
            Some("toml") => {
                toml::from_str(&content)
                    .map_err(|e| ClaudeError::config_error(&format!("Invalid TOML config: {}", e)))?
            }
            _ => {
                return Err(ClaudeError::config_error("Unsupported config file format"));
            }
        };

        Ok(config)
    }

    /// 保存配置
    pub async fn save_config(&self, config: &AdvancedConfig) -> Result<()> {
        // 验证配置
        self.validate_config(config)?;

        let content = match self.config_path.extension().and_then(|s| s.to_str()) {
            Some("yaml") | Some("yml") => {
                serde_yaml::to_string(config)
                    .map_err(|e| ClaudeError::config_error(&format!("Failed to serialize YAML: {}", e)))?
            }
            Some("json") => {
                serde_json::to_string_pretty(config)
                    .map_err(|e| ClaudeError::config_error(&format!("Failed to serialize JSON: {}", e)))?
            }
            Some("toml") => {
                toml::to_string(config)
                    .map_err(|e| ClaudeError::config_error(&format!("Failed to serialize TOML: {}", e)))?
            }
            _ => {
                return Err(ClaudeError::config_error("Unsupported config file format"));
            }
        };

        // 确保目录存在
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| ClaudeError::io_error(&format!("Failed to create config directory: {}", e)))?;
        }

        fs::write(&self.config_path, content).await
            .map_err(|e| ClaudeError::io_error(&format!("Failed to write config file: {}", e)))?;

        info!("Configuration saved to {:?}", self.config_path);
        Ok(())
    }

    /// 验证配置
    fn validate_config(&self, config: &AdvancedConfig) -> Result<()> {
        for validator in &self.validators {
            validator.validate(config)?;
        }
        Ok(())
    }

    /// 添加配置监听器
    pub fn add_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) {
        self.watchers.push(watcher);
    }

    /// 添加配置验证器
    pub fn add_validator(&mut self, validator: Box<dyn ConfigValidator>) {
        self.validators.push(validator);
    }

    /// 获取当前配置
    pub fn get_config(&self) -> Option<&AdvancedConfig> {
        self.config_cache.as_ref()
    }

    /// 更新配置
    pub async fn update_config<F>(&mut self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut AdvancedConfig),
    {
        if let Some(mut config) = self.config_cache.clone() {
            let old_config = config.clone();
            updater(&mut config);
            
            // 验证新配置
            self.validate_config(&config)?;
            
            // 保存配置
            self.save_config(&config).await?;
            
            // 通知监听器
            for watcher in &self.watchers {
                watcher.on_config_changed(&old_config, &config);
            }
            
            self.config_cache = Some(config);
        }
        
        Ok(())
    }

    /// 获取环境特定配置路径
    pub fn get_environment_config_path(&self) -> PathBuf {
        let mut path = self.config_path.clone();
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let extension = path.extension().unwrap_or_default().to_string_lossy();
        
        let env_name = match self.environment {
            Environment::Development => "development",
            Environment::Testing => "testing",
            Environment::Staging => "staging",
            Environment::Production => "production",
        };
        
        path.set_file_name(format!("{}.{}.{}", stem, env_name, extension));
        path
    }
}
