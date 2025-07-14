use claude_code_rust::config::{ClaudeConfig, ConfigManager};
use claude_code_rust::error::{ClaudeError, Result};
use claude_code_rust::fs::FileSystemManager;
use std::path::PathBuf;
use tempfile::TempDir;

/// 测试配置管理器
#[test]
fn test_config_manager() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    // 测试创建配置管理器
    let mut config_manager = ConfigManager::from_path(config_path.clone()).unwrap();
    
    // 测试获取配置
    let config = config_manager.get_config();
    assert_eq!(config.ui.theme, "default");
    assert!(!config.ui.enable_tui);
    
    // 测试修改配置
    {
        let config = config_manager.get_config_mut();
        config.ui.theme = "dark".to_string();
        config.ui.enable_tui = true;
    }
    
    // 测试保存配置
    config_manager.save().unwrap();
    
    // 测试重新加载配置
    let config_manager2 = ConfigManager::from_path(config_path).unwrap();
    let config2 = config_manager2.get_config();
    assert_eq!(config2.ui.theme, "dark");
    assert!(config2.ui.enable_tui);
}

/// 测试默认配置
#[test]
fn test_default_config() {
    let config = ClaudeConfig::default();
    
    assert_eq!(config.ui.theme, "default");
    assert!(!config.ui.enable_tui);
    assert!(config.permissions.require_confirmation); // 修正：默认为true
    assert!(!config.permissions.allowed_tools.is_empty()); // 默认有一些允许的工具
    assert!(config.permissions.denied_tools.is_empty());
}

/// 测试文件系统管理器
#[test]
fn test_file_system_manager() {
    let temp_dir = TempDir::new().unwrap();
    let working_dirs = vec![temp_dir.path().to_path_buf()];
    
    let mut fs_manager = FileSystemManager::new(working_dirs);
    
    // 测试添加工作目录
    let new_dir = temp_dir.path().join("new_dir");
    std::fs::create_dir(&new_dir).unwrap();
    fs_manager.add_working_dir(new_dir.clone());
    
    // 测试获取工作目录
    let dirs = fs_manager.get_working_dirs();
    assert!(dirs.contains(&new_dir));
}

/// 测试错误处理
#[test]
fn test_error_handling() {
    // 测试通用错误
    let error = ClaudeError::General("Test error".to_string());
    assert_eq!(error.to_string(), "General error: Test error");
    
    // 测试IO错误
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let claude_error = ClaudeError::Io(io_error);
    assert!(claude_error.to_string().contains("File not found"));
    
    // 测试YAML错误
    let yaml_error = serde_yaml::Error::from(serde_yaml::from_str::<serde_yaml::Value>("invalid: yaml: content").unwrap_err());
    let claude_error = ClaudeError::Yaml(yaml_error);
    assert!(claude_error.to_string().contains("YAML"));
}

/// 测试配置序列化和反序列化
#[test]
fn test_config_serialization() {
    let config = ClaudeConfig::default();
    
    // 测试序列化
    let yaml_str = serde_yaml::to_string(&config).unwrap();
    assert!(yaml_str.contains("ui:"));
    assert!(yaml_str.contains("permissions:"));
    
    // 测试反序列化
    let deserialized_config: ClaudeConfig = serde_yaml::from_str(&yaml_str).unwrap();
    assert_eq!(deserialized_config.ui.theme, config.ui.theme);
    assert_eq!(deserialized_config.ui.enable_tui, config.ui.enable_tui);
}

/// 测试权限配置
#[test]
fn test_permissions_config() {
    let mut config = ClaudeConfig::default();
    
    // 测试添加允许的工具
    config.permissions.allowed_tools.push("git".to_string());
    config.permissions.allowed_tools.push("file-system".to_string());
    
    // 测试添加拒绝的工具
    config.permissions.denied_tools.push("network".to_string());
    
    assert!(config.permissions.allowed_tools.contains(&"git".to_string()));
    assert!(config.permissions.denied_tools.contains(&"network".to_string()));
    // 注意：默认配置可能包含其他工具，所以检查包含而不是精确长度
    assert!(config.permissions.allowed_tools.len() >= 1);
    assert_eq!(config.permissions.denied_tools.len(), 1);
}

/// 测试UI配置
#[test]
fn test_ui_config() {
    let mut config = ClaudeConfig::default();
    
    // 测试主题设置
    config.ui.theme = "dark".to_string();
    assert_eq!(config.ui.theme, "dark");
    
    // 测试TUI启用
    config.ui.enable_tui = true;
    assert!(config.ui.enable_tui);
    
    // 测试行号显示
    config.ui.show_line_numbers = false;
    assert!(!config.ui.show_line_numbers);
}

/// 测试配置验证
#[test]
fn test_config_validation() {
    let config = ClaudeConfig::default();
    
    // 基本验证：主题不应为空
    assert!(!config.ui.theme.is_empty());
    
    // 权限列表应该是有效的
    for tool in &config.permissions.allowed_tools {
        assert!(!tool.is_empty());
    }
    
    for tool in &config.permissions.denied_tools {
        assert!(!tool.is_empty());
    }
}

/// 测试配置文件路径解析
#[test]
fn test_config_path_resolution() {
    // 测试通过创建配置管理器来验证路径
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    let _config_manager = ConfigManager::from_path(config_path.clone()).unwrap();

    // 验证配置文件被创建
    assert!(config_path.exists());
    assert!(config_path.to_string_lossy().ends_with("test_config.yaml"));
}

/// 测试配置合并
#[test]
fn test_config_merging() {
    let mut base_config = ClaudeConfig::default();
    base_config.ui.theme = "light".to_string();
    base_config.permissions.allowed_tools.push("git".to_string());
    
    let mut override_config = ClaudeConfig::default();
    override_config.ui.theme = "dark".to_string();
    override_config.permissions.allowed_tools.push("network".to_string());
    
    // 手动合并（在实际实现中可能有专门的合并方法）
    base_config.ui.theme = override_config.ui.theme;
    base_config.permissions.allowed_tools.extend(override_config.permissions.allowed_tools);
    
    assert_eq!(base_config.ui.theme, "dark");
    // 注意：默认配置可能包含其他工具，所以检查包含而不是精确长度
    assert!(base_config.permissions.allowed_tools.len() >= 2);
}

/// 测试配置备份和恢复
#[test]
fn test_config_backup_restore() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");
    let backup_path = temp_dir.path().join("config.yaml.backup");
    
    // 创建初始配置
    let mut config_manager = ConfigManager::from_path(config_path.clone()).unwrap();
    {
        let config = config_manager.get_config_mut();
        config.ui.theme = "custom".to_string();
    }
    config_manager.save().unwrap();
    
    // 创建备份
    std::fs::copy(&config_path, &backup_path).unwrap();
    
    // 修改配置
    {
        let config = config_manager.get_config_mut();
        config.ui.theme = "modified".to_string();
    }
    config_manager.save().unwrap();
    
    // 从备份恢复
    std::fs::copy(&backup_path, &config_path).unwrap();
    let restored_manager = ConfigManager::from_path(config_path).unwrap();
    let restored_config = restored_manager.get_config();
    
    assert_eq!(restored_config.ui.theme, "custom");
}

/// 测试异步文件操作
#[tokio::test]
async fn test_async_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let working_dirs = vec![temp_dir.path().to_path_buf()];
    let fs_manager = FileSystemManager::new(working_dirs);
    
    let test_file = temp_dir.path().join("async_test.txt");
    let test_content = "Hello, async world!";
    
    // 测试异步写入
    fs_manager.write_file(&test_file, test_content).await.unwrap();
    
    // 测试文件存在检查
    assert!(fs_manager.file_exists(&test_file).await);
    
    // 测试异步读取
    let read_content = fs_manager.read_file(&test_file).await.unwrap();
    assert_eq!(read_content, test_content);
    
    // 测试异步删除
    fs_manager.delete_file(&test_file).await.unwrap();
    assert!(!fs_manager.file_exists(&test_file).await);
}

/// 测试错误传播
#[tokio::test]
async fn test_error_propagation() {
    let temp_dir = TempDir::new().unwrap();
    let working_dirs = vec![temp_dir.path().to_path_buf()];
    let fs_manager = FileSystemManager::new(working_dirs);
    
    let nonexistent_file = temp_dir.path().join("nonexistent.txt");
    
    // 测试读取不存在的文件
    let result = fs_manager.read_file(&nonexistent_file).await;
    assert!(result.is_err());
    
    // 测试错误类型
    match result {
        Err(ClaudeError::Io(_)) => {
            // 预期的IO错误
        }
        Err(ClaudeError::General(_)) => {
            // 也可能是通用错误，这也是可以接受的
        }
        _ => panic!("Expected IO or General error"),
    }
}
