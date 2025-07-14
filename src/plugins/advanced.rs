use crate::error::{ClaudeError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// 高级插件管理器
pub struct AdvancedPluginManager {
    /// 已加载的插件
    plugins: Arc<RwLock<HashMap<String, Arc<dyn AdvancedPlugin>>>>,
    /// 插件配置
    plugin_configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
    /// 插件依赖图
    dependency_graph: Arc<RwLock<DependencyGraph>>,
    /// 事件总线
    event_bus: Arc<EventBus>,
    /// 插件目录
    plugin_directory: PathBuf,
    /// 沙箱配置
    sandbox_config: SandboxConfig,
}

/// 高级插件 trait
#[async_trait]
pub trait AdvancedPlugin: Send + Sync {
    /// 插件元数据
    fn metadata(&self) -> &PluginMetadata;
    
    /// 初始化插件
    async fn initialize(&self, context: &PluginContext) -> Result<()>;
    
    /// 启动插件
    async fn start(&self) -> Result<()>;
    
    /// 停止插件
    async fn stop(&self) -> Result<()>;
    
    /// 处理事件
    async fn handle_event(&self, event: &PluginEvent) -> Result<Option<PluginEvent>>;
    
    /// 执行命令
    async fn execute_command(&self, command: &str, args: &[String]) -> Result<CommandResult>;
    
    /// 获取插件状态
    fn get_status(&self) -> PluginStatus;
    
    /// 健康检查
    async fn health_check(&self) -> Result<HealthStatus>;
    
    /// 配置更新
    async fn update_config(&self, config: &serde_json::Value) -> Result<()>;
}

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 描述
    pub description: String,
    /// 作者
    pub author: String,
    /// 许可证
    pub license: String,
    /// 主页
    pub homepage: Option<String>,
    /// 依赖项
    pub dependencies: Vec<PluginDependency>,
    /// 提供的功能
    pub capabilities: Vec<String>,
    /// 所需权限
    pub permissions: Vec<String>,
    /// 最小系统要求
    pub min_system_version: String,
    /// 插件类型
    pub plugin_type: PluginType,
}

/// 插件依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version_requirement: String,
    pub optional: bool,
}

/// 插件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginType {
    /// 核心插件
    Core,
    /// 扩展插件
    Extension,
    /// 主题插件
    Theme,
    /// 语言支持
    Language,
    /// 工具集成
    Tool,
    /// 自定义
    Custom(String),
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// 是否启用
    pub enabled: bool,
    /// 自动启动
    pub auto_start: bool,
    /// 优先级
    pub priority: i32,
    /// 配置数据
    pub config_data: serde_json::Value,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 资源限制
    pub resource_limits: ResourceLimits,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// 最大内存使用（MB）
    pub max_memory_mb: Option<u64>,
    /// 最大 CPU 使用率（百分比）
    pub max_cpu_percent: Option<f64>,
    /// 最大文件描述符数
    pub max_file_descriptors: Option<u64>,
    /// 网络访问权限
    pub network_access: bool,
    /// 文件系统访问权限
    pub filesystem_access: Vec<PathBuf>,
}

/// 插件上下文
#[derive(Clone)]
pub struct PluginContext {
    /// 插件名称
    pub plugin_name: String,
    /// 工作目录
    pub work_dir: PathBuf,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 配置目录
    pub config_dir: PathBuf,
    /// 日志记录器
    pub logger: Arc<dyn PluginLogger>,
    /// 事件发送器
    pub event_sender: Arc<dyn EventSender>,
    /// API 客户端
    pub api_client: Arc<dyn PluginApiClient>,
}

/// 插件事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    /// 事件ID
    pub id: String,
    /// 事件类型
    pub event_type: String,
    /// 源插件
    pub source: String,
    /// 目标插件（可选）
    pub target: Option<String>,
    /// 事件数据
    pub data: serde_json::Value,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 优先级
    pub priority: EventPriority,
}

/// 事件优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// 命令结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// 是否成功
    pub success: bool,
    /// 输出
    pub output: String,
    /// 错误信息
    pub error: Option<String>,
    /// 退出码
    pub exit_code: i32,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
}

/// 插件状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginStatus {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 已初始化
    Initialized,
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 错误状态
    Error(String),
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// 是否健康
    pub healthy: bool,
    /// 状态消息
    pub message: String,
    /// 详细信息
    pub details: HashMap<String, serde_json::Value>,
    /// 最后检查时间
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// 依赖图
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// 节点（插件名称）
    nodes: Vec<String>,
    /// 边（依赖关系）
    edges: Vec<(String, String)>,
}

/// 事件总线
pub struct EventBus {
    /// 事件订阅者
    subscribers: Arc<RwLock<HashMap<String, Vec<Arc<dyn EventSubscriber>>>>>,
    /// 事件队列
    event_queue: Arc<RwLock<Vec<PluginEvent>>>,
}

/// 事件订阅者
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn handle_event(&self, event: &PluginEvent) -> Result<()>;
}

/// 沙箱配置
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 启用沙箱
    pub enabled: bool,
    /// 允许的系统调用
    pub allowed_syscalls: Vec<String>,
    /// 禁止的系统调用
    pub blocked_syscalls: Vec<String>,
    /// 网络隔离
    pub network_isolation: bool,
    /// 文件系统隔离
    pub filesystem_isolation: bool,
}

/// 插件日志记录器
#[async_trait]
pub trait PluginLogger: Send + Sync {
    async fn log(&self, level: LogLevel, message: &str);
    async fn log_with_context(&self, level: LogLevel, message: &str, context: &HashMap<String, String>);
}

/// 日志级别
#[derive(Debug, Clone)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// 事件发送器
#[async_trait]
pub trait EventSender: Send + Sync {
    async fn send_event(&self, event: PluginEvent) -> Result<()>;
    async fn broadcast_event(&self, event: PluginEvent) -> Result<()>;
}

/// 插件 API 客户端
#[async_trait]
pub trait PluginApiClient: Send + Sync {
    async fn get(&self, endpoint: &str) -> Result<serde_json::Value>;
    async fn post(&self, endpoint: &str, data: &serde_json::Value) -> Result<serde_json::Value>;
    async fn put(&self, endpoint: &str, data: &serde_json::Value) -> Result<serde_json::Value>;
    async fn delete(&self, endpoint: &str) -> Result<serde_json::Value>;
}

impl AdvancedPluginManager {
    /// 创建新的高级插件管理器
    pub fn new(plugin_directory: PathBuf, sandbox_config: SandboxConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_configs: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(DependencyGraph::default())),
            event_bus: Arc::new(EventBus::new()),
            plugin_directory,
            sandbox_config,
        }
    }

    /// 扫描并加载插件
    pub async fn scan_and_load_plugins(&self) -> Result<()> {
        info!("Scanning plugins in directory: {:?}", self.plugin_directory);
        
        if !self.plugin_directory.exists() {
            tokio::fs::create_dir_all(&self.plugin_directory).await
                .map_err(|e| ClaudeError::fs_error(&format!("Failed to create plugin directory: {}", e)))?;
        }

        let mut entries = tokio::fs::read_dir(&self.plugin_directory).await
            .map_err(|e| ClaudeError::fs_error(&format!("Failed to read plugin directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ClaudeError::fs_error(&format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_dir() {
                if let Err(e) = self.load_plugin_from_directory(&path).await {
                    warn!("Failed to load plugin from {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// 从目录加载插件
    async fn load_plugin_from_directory(&self, plugin_dir: &Path) -> Result<()> {
        let manifest_path = plugin_dir.join("plugin.yaml");
        if !manifest_path.exists() {
            return Err(ClaudeError::config_error("Plugin manifest not found"));
        }

        let manifest_content = tokio::fs::read_to_string(&manifest_path).await
            .map_err(|e| ClaudeError::fs_error(&format!("Failed to read plugin manifest: {}", e)))?;

        let metadata: PluginMetadata = serde_yaml::from_str(&manifest_content)
            .map_err(|e| ClaudeError::config_error(&format!("Invalid plugin manifest: {}", e)))?;

        info!("Loading plugin: {} v{}", metadata.name, metadata.version);

        // 这里应该实际加载插件代码
        // 为了演示，我们创建一个模拟插件
        let plugin = Arc::new(MockPlugin::new(metadata));
        
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin.metadata().name.clone(), plugin);

        Ok(())
    }

    /// 启动所有插件
    pub async fn start_all_plugins(&self) -> Result<()> {
        let plugins = self.plugins.read().await;
        
        for (name, plugin) in plugins.iter() {
            info!("Starting plugin: {}", name);
            if let Err(e) = plugin.start().await {
                error!("Failed to start plugin {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// 停止所有插件
    pub async fn stop_all_plugins(&self) -> Result<()> {
        let plugins = self.plugins.read().await;
        
        for (name, plugin) in plugins.iter() {
            info!("Stopping plugin: {}", name);
            if let Err(e) = plugin.stop().await {
                error!("Failed to stop plugin {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// 获取插件状态
    pub async fn get_plugin_status(&self, plugin_name: &str) -> Option<PluginStatus> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_name).map(|plugin| plugin.get_status())
    }

    /// 执行插件命令
    pub async fn execute_plugin_command(&self, plugin_name: &str, command: &str, args: &[String]) -> Result<CommandResult> {
        let plugins = self.plugins.read().await;
        let plugin = plugins.get(plugin_name)
            .ok_or_else(|| ClaudeError::config_error(&format!("Plugin not found: {}", plugin_name)))?;
        
        plugin.execute_command(command, args).await
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            event_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn subscribe(&self, event_type: String, subscriber: Arc<dyn EventSubscriber>) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.entry(event_type).or_insert_with(Vec::new).push(subscriber);
    }

    pub async fn publish(&self, event: PluginEvent) -> Result<()> {
        let subscribers = self.subscribers.read().await;
        
        if let Some(event_subscribers) = subscribers.get(&event.event_type) {
            for subscriber in event_subscribers {
                if let Err(e) = subscriber.handle_event(&event).await {
                    error!("Event subscriber error: {}", e);
                }
            }
        }

        Ok(())
    }
}

/// 模拟插件实现（用于演示）
pub struct MockPlugin {
    metadata: PluginMetadata,
    status: Arc<RwLock<PluginStatus>>,
}

impl MockPlugin {
    pub fn new(metadata: PluginMetadata) -> Self {
        Self {
            metadata,
            status: Arc::new(RwLock::new(PluginStatus::Uninitialized)),
        }
    }
}

#[async_trait]
impl AdvancedPlugin for MockPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&self, _context: &PluginContext) -> Result<()> {
        let mut status = self.status.write().await;
        *status = PluginStatus::Initialized;
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        let mut status = self.status.write().await;
        *status = PluginStatus::Running;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let mut status = self.status.write().await;
        *status = PluginStatus::Stopped;
        Ok(())
    }

    async fn handle_event(&self, _event: &PluginEvent) -> Result<Option<PluginEvent>> {
        Ok(None)
    }

    async fn execute_command(&self, command: &str, _args: &[String]) -> Result<CommandResult> {
        Ok(CommandResult {
            success: true,
            output: format!("Mock plugin executed command: {}", command),
            error: None,
            exit_code: 0,
            execution_time_ms: 10,
        })
    }

    fn get_status(&self) -> PluginStatus {
        // 这里应该返回实际状态，为了演示简化处理
        PluginStatus::Running
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            healthy: true,
            message: "Plugin is healthy".to_string(),
            details: HashMap::new(),
            last_check: chrono::Utc::now(),
        })
    }

    async fn update_config(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
}
