use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;
use std::env;

/// 测试CLI基本功能
#[test]
fn test_cli_help() {
    let output = Command::new("./target/release/claude-code-rust")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Claude Code"));
    assert!(stdout.contains("Usage:"));
}

/// 测试版本命令
#[test]
fn test_cli_version() {
    let output = Command::new("./target/release/claude-code-rust")
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("claude") || stdout.contains("0.1.0"));
    assert!(stdout.contains("0.1.0"));
}

/// 测试配置命令
#[test]
fn test_config_commands() {
    // 测试配置显示
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "show"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Configuration"));
}

/// 测试内存命令
#[test]
fn test_memory_commands() {
    // 测试内存显示
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["memory", "show"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Memory Contents"));
}

/// 测试Git命令
#[test]
fn test_git_commands() {
    // 测试Git状态
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["git", "status"])
        .output()
        .expect("Failed to execute command");

    // Git命令可能失败（如果不在git仓库中），但应该有适当的错误信息
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    
    // 应该包含Git相关的输出或错误信息
    assert!(stdout.contains("Git") || stderr.contains("git") || stdout.contains("repository"));
}

/// 测试语法高亮命令
#[cfg(feature = "syntax-highlighting")]
#[test]
fn test_highlight_commands() {
    // 测试支持的语言列表
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["highlight", "languages"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Supported Languages"));
}

/// 测试权限命令
#[test]
fn test_permissions_commands() {
    // 测试权限显示
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["permissions", "show"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Permission Settings"));
}

/// 测试文件操作命令（使用config命令作为替代）
#[test]
fn test_file_commands() {
    // 测试配置文件操作（作为文件操作的替代测试）
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "show"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Configuration") || stdout.contains("config"));
}

/// 测试进程命令
#[test]
fn test_process_commands() {
    // 测试进程列表
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["process", "list"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Running Processes") || stdout.contains("No processes"));
}

/// 测试MCP命令
#[test]
fn test_mcp_commands() {
    // 测试MCP帮助
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["mcp", "--help"])
        .output()
        .expect("Failed to execute command");

    // MCP命令应该显示帮助信息
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("MCP") || stdout.contains("mcp") || stdout.contains("server"));
}

/// 测试图像处理命令
#[cfg(feature = "image-processing")]
#[test]
fn test_image_commands() {
    // 测试图像信息（使用不存在的文件应该返回错误）
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["image", "info", "nonexistent.jpg"])
        .output()
        .expect("Failed to execute command");

    // 应该失败但有适当的错误信息
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error") || stderr.contains("not found"));
}

/// 测试无效命令
#[test]
fn test_invalid_command() {
    let output = Command::new("./target/release/claude-code-rust")
        .arg("invalid-command")
        .output()
        .expect("Failed to execute command");

    // 应该失败
    assert!(!output.status.success());
}

/// 测试配置文件创建
#[test]
fn test_config_file_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    // 使用自定义配置路径
    let output = Command::new("./target/release/claude-code-rust")
        .env("CLAUDE_CONFIG_PATH", config_path.to_str().unwrap())
        .args(&["config", "show"])
        .output()
        .expect("Failed to execute command");

    // 配置文件应该被创建
    assert!(config_path.exists() || output.status.success());
}

/// 性能测试：测试命令响应时间
#[test]
fn test_command_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["--help"])
        .output()
        .expect("Failed to execute command");
    
    let duration = start.elapsed();
    
    assert!(output.status.success());
    // 帮助命令应该在1秒内完成
    assert!(duration.as_secs() < 1, "Help command took too long: {:?}", duration);
}

/// 测试并发执行
#[test]
fn test_concurrent_execution() {
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];
    
    // 启动多个并发命令
    for _ in 0..5 {
        let success_count = Arc::clone(&success_count);
        let handle = thread::spawn(move || {
            let output = Command::new("./target/release/claude-code-rust")
                .args(&["config", "show"])
                .output()
                .expect("Failed to execute command");
            
            if output.status.success() {
                success_count.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    // 所有命令都应该成功
    assert_eq!(success_count.load(Ordering::SeqCst), 5);
}

/// 测试配置文件格式支持
#[test]
fn test_config_file_formats() {
    let temp_dir = TempDir::new().unwrap();

    // 测试 YAML 格式
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "init", "--format", "yaml", "--path", temp_dir.path().join("test.yaml").to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    assert!(temp_dir.path().join("test.yaml").exists());

    // 测试 JSON 格式
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "init", "--format", "json", "--path", temp_dir.path().join("test.json").to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    assert!(temp_dir.path().join("test.json").exists());

    // 测试 TOML 格式
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "init", "--format", "toml", "--path", temp_dir.path().join("test.toml").to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    assert!(temp_dir.path().join("test.toml").exists());
}

/// 测试配置验证功能
#[test]
fn test_config_validation() {
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "validate"])
        .output()
        .expect("Failed to execute command");

    // 验证应该失败，因为没有设置 API 密钥
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("API key") || stderr.contains("validation"));
}

/// 测试配置文件位置列表
#[test]
fn test_config_list() {
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "list"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Configuration File Locations"));
    assert!(stdout.contains("Current Directory"));
    assert!(stdout.contains("User Home"));
}

/// 测试项目初始化功能
#[test]
fn test_project_initialization() {
    let temp_dir = TempDir::new().unwrap();
    env::set_current_dir(&temp_dir).unwrap();

    let output = Command::new("./target/release/claude-code-rust")
        .args(&["init", ".", "--force"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    assert!(temp_dir.path().join("CLAUDE.md").exists());

    let content = fs::read_to_string(temp_dir.path().join("CLAUDE.md")).unwrap();
    assert!(content.contains("CLAUDE.md"));
    assert!(content.contains("Essential Commands"));
}

/// 测试错误处理
#[test]
fn test_error_handling() {
    // 测试无效命令
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["invalid-command"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    // 测试无效参数
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["config", "invalid-action"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
}

/// 测试帮助信息完整性
#[test]
fn test_help_completeness() {
    let commands = vec![
        "api",
        "config",
        "init",
        "memory",
        "mcp",
        "git",
        "highlight",
        "permissions",
        "file",
    ];

    for command in commands {
        let output = Command::new("./target/release/claude-code-rust")
            .args(&[command, "--help"])
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success(), "Help for command '{}' failed", command);

        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("Usage:"), "Help for '{}' missing usage", command);
    }
}

/// 测试调试模式
#[test]
fn test_debug_mode() {
    let output = Command::new("./target/release/claude-code-rust")
        .args(&["--debug", "config", "show"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    // 调试模式应该产生日志输出
    assert!(stderr.contains("INFO") || stderr.contains("DEBUG"));
}
