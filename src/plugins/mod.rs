//! 插件和扩展系统模块
//!
//! 实现插件架构，支持第三方扩展和自定义工具

pub mod advanced;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{ClaudeError, Result};

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 插件描述
    pub description: String,
    /// 作者
    pub author: String,
    /// 许可证
    pub license: Option<String>,
    /// 主页URL
    pub homepage: Option<String>,
    /// 依赖的Claude Code版本
    pub claude_version: String,
    /// 插件依赖
    pub dependencies: Vec<String>,
    /// 插件标签
    pub tags: Vec<String>,
    /// 插件入口点
    pub entry_point: String,
}

/// 插件状态
#[derive(Debug, Clone, PartialEq)]
pub enum PluginStatus {
    /// 未加载
    Unloaded,
    /// 已加载
    Loaded,
    /// 已激活
    Active,
    /// 已停用
    Disabled,
    /// 错误状态
    Error(String),
}

/// 插件事件
#[derive(Debug, Clone)]
pub enum PluginEvent {
    /// 文件变化事件
    FileChanged { path: PathBuf },
    /// 命令执行事件
    CommandExecuted { command: String, args: Vec<String> },
    /// 配置变化事件
    ConfigChanged { key: String, value: String },
    /// 自定义事件
    Custom { event_type: String, data: serde_json::Value },
}

/// 插件命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    /// 命令名称
    pub name: String,
    /// 命令描述
    pub description: String,
    /// 命令参数
    pub parameters: Vec<CommandParameter>,
    /// 是否需要确认
    pub requires_confirmation: bool,
}

/// 命令参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParameter {
    /// 参数名称
    pub name: String,
    /// 参数类型
    pub param_type: ParameterType,
    /// 是否必需
    pub required: bool,
    /// 默认值
    pub default_value: Option<String>,
    /// 参数描述
    pub description: String,
}

/// 参数类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Path,
    Choice(Vec<String>),
}

/// 插件特征
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 获取插件元数据
    fn metadata(&self) -> &PluginMetadata;

    /// 初始化插件
    async fn initialize(&mut self) -> Result<()>;

    /// 停用插件
    async fn shutdown(&mut self) -> Result<()>;

    /// 处理事件
    async fn handle_event(&mut self, event: &PluginEvent) -> Result<()>;

    /// 执行命令
    async fn execute_command(&mut self, command: &str, args: &[String]) -> Result<serde_json::Value>;

    /// 获取插件提供的命令
    fn get_commands(&self) -> Vec<PluginCommand>;

    /// 获取插件配置架构
    fn get_config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// 验证插件配置
    fn validate_config(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
}

/// 插件实例
pub struct PluginInstance {
    /// 插件实现
    plugin: Box<dyn Plugin>,
    /// 插件状态
    status: PluginStatus,
    /// 插件配置
    config: Option<serde_json::Value>,
    /// 插件路径
    path: PathBuf,
}

/// 插件管理器
pub struct PluginManager {
    /// 已加载的插件
    plugins: Arc<RwLock<HashMap<String, PluginInstance>>>,
    /// 插件目录
    plugin_directories: Vec<PathBuf>,
    /// 事件监听器
    event_listeners: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 全局配置
    global_config: Arc<RwLock<serde_json::Value>>,
}

impl PluginInstance {
    /// 创建新的插件实例
    pub fn new(plugin: Box<dyn Plugin>, path: PathBuf) -> Self {
        Self {
            plugin,
            status: PluginStatus::Unloaded,
            config: None,
            path,
        }
    }

    /// 获取插件元数据
    pub fn metadata(&self) -> &PluginMetadata {
        self.plugin.metadata()
    }

    /// 获取插件状态
    pub fn status(&self) -> &PluginStatus {
        &self.status
    }

    /// 设置插件状态
    pub fn set_status(&mut self, status: PluginStatus) {
        self.status = status;
    }

    /// 获取插件配置
    pub fn config(&self) -> Option<&serde_json::Value> {
        self.config.as_ref()
    }

    /// 设置插件配置
    pub fn set_config(&mut self, config: serde_json::Value) -> Result<()> {
        self.plugin.validate_config(&config)?;
        self.config = Some(config);
        Ok(())
    }
}

impl PluginManager {
    /// 创建新的插件管理器
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_directories: Vec::new(),
            event_listeners: Arc::new(RwLock::new(HashMap::new())),
            global_config: Arc::new(RwLock::new(serde_json::json!({}))),
        }
    }

    /// 添加插件目录
    pub fn add_plugin_directory<P: AsRef<Path>>(&mut self, path: P) {
        self.plugin_directories.push(path.as_ref().to_path_buf());
    }

    /// 扫描并加载插件
    pub async fn scan_and_load_plugins(&self) -> Result<Vec<String>> {
        let mut loaded_plugins = Vec::new();

        for dir in &self.plugin_directories {
            if dir.exists() && dir.is_dir() {
                let entries = std::fs::read_dir(dir)
                    .map_err(|e| ClaudeError::General(format!("Failed to read plugin directory: {}", e)))?;

                for entry in entries {
                    let entry = entry.map_err(|e| ClaudeError::General(format!("Failed to read directory entry: {}", e)))?;
                    let path = entry.path();

                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        match self.load_plugin_from_manifest(&path).await {
                            Ok(plugin_name) => loaded_plugins.push(plugin_name),
                            Err(e) => eprintln!("Failed to load plugin from {}: {}", path.display(), e),
                        }
                    }
                }
            }
        }

        Ok(loaded_plugins)
    }

    /// 从清单文件加载插件
    async fn load_plugin_from_manifest(&self, manifest_path: &Path) -> Result<String> {
        let manifest_content = tokio::fs::read_to_string(manifest_path).await
            .map_err(|e| ClaudeError::General(format!("Failed to read manifest: {}", e)))?;

        let metadata: PluginMetadata = toml::from_str(&manifest_content)
            .map_err(|e| ClaudeError::General(format!("Failed to parse manifest: {}", e)))?;

        // 这里应该动态加载插件，但为了简化，我们创建一个示例插件
        let plugin = Box::new(ExamplePlugin::new(metadata.clone()));
        let instance = PluginInstance::new(plugin, manifest_path.to_path_buf());

        let plugin_name = metadata.name.clone();
        self.plugins.write().await.insert(plugin_name.clone(), instance);

        Ok(plugin_name)
    }

    /// 激活插件
    pub async fn activate_plugin(&self, plugin_name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(instance) = plugins.get_mut(plugin_name) {
            match instance.status {
                PluginStatus::Unloaded | PluginStatus::Loaded | PluginStatus::Disabled => {
                    instance.plugin.initialize().await?;
                    instance.set_status(PluginStatus::Active);
                    Ok(())
                }
                PluginStatus::Active => Ok(()), // 已经激活
                _ => Err(ClaudeError::General(format!(
                    "Cannot activate plugin '{}' in state {:?}", plugin_name, instance.status
                ))),
            }
        } else {
            Err(ClaudeError::General(format!("Plugin '{}' not found", plugin_name)))
        }
    }

    /// 停用插件
    pub async fn deactivate_plugin(&self, plugin_name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(instance) = plugins.get_mut(plugin_name) {
            if instance.status == PluginStatus::Active {
                instance.plugin.shutdown().await?;
                instance.set_status(PluginStatus::Disabled);
            }
            Ok(())
        } else {
            Err(ClaudeError::General(format!("Plugin '{}' not found", plugin_name)))
        }
    }

    /// 获取插件列表
    pub async fn list_plugins(&self) -> Vec<(String, PluginStatus, PluginMetadata)> {
        let plugins = self.plugins.read().await;
        plugins.iter()
            .map(|(name, instance)| {
                (name.clone(), instance.status.clone(), instance.metadata().clone())
            })
            .collect()
    }

    /// 执行插件命令
    pub async fn execute_plugin_command(
        &self,
        plugin_name: &str,
        command: &str,
        args: &[String],
    ) -> Result<serde_json::Value> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(instance) = plugins.get_mut(plugin_name) {
            if instance.status == PluginStatus::Active {
                instance.plugin.execute_command(command, args).await
            } else {
                Err(ClaudeError::General(format!(
                    "Plugin '{}' is not active", plugin_name
                )))
            }
        } else {
            Err(ClaudeError::General(format!("Plugin '{}' not found", plugin_name)))
        }
    }

    /// 广播事件给所有插件
    pub async fn broadcast_event(&self, event: &PluginEvent) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        for (_, instance) in plugins.iter_mut() {
            if instance.status == PluginStatus::Active {
                if let Err(e) = instance.plugin.handle_event(event).await {
                    eprintln!("Plugin '{}' failed to handle event: {}", 
                             instance.metadata().name, e);
                }
            }
        }

        Ok(())
    }

    /// 获取所有插件提供的命令
    pub async fn get_all_commands(&self) -> HashMap<String, Vec<PluginCommand>> {
        let plugins = self.plugins.read().await;
        let mut all_commands = HashMap::new();

        for (name, instance) in plugins.iter() {
            if instance.status == PluginStatus::Active {
                let commands = instance.plugin.get_commands();
                if !commands.is_empty() {
                    all_commands.insert(name.clone(), commands);
                }
            }
        }

        all_commands
    }
}

/// 示例插件实现
pub struct ExamplePlugin {
    metadata: PluginMetadata,
}

impl ExamplePlugin {
    pub fn new(metadata: PluginMetadata) -> Self {
        Self { metadata }
    }
}

#[async_trait]
impl Plugin for ExamplePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> Result<()> {
        println!("Initializing plugin: {}", self.metadata.name);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        println!("Shutting down plugin: {}", self.metadata.name);
        Ok(())
    }

    async fn handle_event(&mut self, event: &PluginEvent) -> Result<()> {
        match event {
            PluginEvent::FileChanged { path } => {
                println!("Plugin {} received file change event: {}", 
                        self.metadata.name, path.display());
            }
            _ => {}
        }
        Ok(())
    }

    async fn execute_command(&mut self, command: &str, args: &[String]) -> Result<serde_json::Value> {
        match command {
            "hello" => Ok(serde_json::json!({
                "message": format!("Hello from plugin {}!", self.metadata.name),
                "args": args
            })),
            _ => Err(ClaudeError::General(format!("Unknown command: {}", command))),
        }
    }

    fn get_commands(&self) -> Vec<PluginCommand> {
        vec![
            PluginCommand {
                name: "hello".to_string(),
                description: "Say hello from the plugin".to_string(),
                parameters: vec![],
                requires_confirmation: false,
            }
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(manager.plugin_directories.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let manager = PluginManager::new();
        
        let metadata = PluginMetadata {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Test Author".to_string(),
            license: Some("MIT".to_string()),
            homepage: None,
            claude_version: "0.1.0".to_string(),
            dependencies: vec![],
            tags: vec!["test".to_string()],
            entry_point: "main".to_string(),
        };

        let plugin = Box::new(ExamplePlugin::new(metadata.clone()));
        let instance = PluginInstance::new(plugin, PathBuf::from("test.toml"));
        
        manager.plugins.write().await.insert("test_plugin".to_string(), instance);

        // 测试激活
        let result = manager.activate_plugin("test_plugin").await;
        assert!(result.is_ok());

        // 测试停用
        let result = manager.deactivate_plugin("test_plugin").await;
        assert!(result.is_ok());
    }
}
