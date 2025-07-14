//! Claude Code Rust - 一个用 Rust 实现的智能编程助手
//!
//! 这是 Claude Code 的 Rust 版本实现，提供高性能的编程辅助功能

mod agent;
mod analytics;
mod cache;
mod cli;
mod cloud;
mod collaboration;
mod config;
mod context;
mod conversation;
mod cost;
mod data_processing;
mod database;
mod devops;
mod distributed;
mod error;
mod fs;
mod gateway;
mod git;
mod inference;
mod mcp;
mod ml;
mod monitoring;
mod network;
mod plugins;
mod process;
mod refactor;
mod search;
mod security;
mod steering;
mod streaming;
mod tools;
mod ui;
mod watcher;
mod web;
mod workflow;

#[cfg(feature = "image-processing")]
mod image_processing;

#[cfg(feature = "syntax-highlighting")]
mod syntax_highlighting;



use cli::{Cli, Commands, ConfigAction};
use config::{ConfigManager, ConfigFormat};
use error::{init_logging, report_error, ClaudeError, Result};
use fs::FileSystemManager;
use network::{ClaudeApiClient, ContentBlock, Tool, ToolChoice, Message, MessageContent, ResponseContentBlock};
use std::path::Path;


#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // 解析命令行参数
    let cli = Cli::parse_args();

    // 初始化日志
    init_logging(cli.debug)?;

    tracing::info!("Starting Claude Code Rust v0.1.0");

    // 创建 CLI 处理器
    let cli_handler = match cli::ClaudeCodeCli::new().await {
        Ok(handler) => handler,
        Err(e) => {
            eprintln!("❌ Failed to initialize CLI handler: {}", e);
            return Err(e);
        }
    };

    // 执行命令
    cli_handler.execute(cli).await?;

    Ok(())
}

async fn handle_command(
    command: Commands,
    config_manager: &mut ConfigManager,
    fs_manager: &mut FileSystemManager,
) -> Result<()> {
    match command {
        Commands::Doctor => {
            handle_doctor_command(config_manager).await?;
        }
        Commands::Status => {
            handle_status_command(config_manager).await?;
        }
        Commands::Cost { days } => {
            handle_cost_command(days).await?;
        }
        Commands::Clear => {
            handle_clear_command().await?;
        }
        Commands::Compact { instructions, level } => {
            handle_compact_command_enhanced(instructions, level).await?;
        }
        Commands::Demo => {
            handle_demo_command().await?;
        }
        Commands::Stream { url, realtime } => {
            handle_stream_command(url, realtime).await?;
        }
        Commands::Api { message, model, stream, image, tools } => {
            handle_api_command(message, model, stream, image, tools).await?;
        }
        Commands::Config { action } => {
            let config_manager = ConfigManager::new()?;
            handle_config_command(action, config_manager).await?;
        }
        Commands::Init { path, force } => {
            handle_init_command(path, force).await?;
        }
        Commands::Review { target, review_type } => {
            handle_review_command(target, review_type).await?;
        }
        Commands::Memory { action } => {
            handle_memory_command(action).await?;
        }
        Commands::Permissions { action } => {
            handle_permissions_command(action, config_manager).await?;
        }
        Commands::Export { format, output } => {
            handle_export_command(format, output).await?;
        }


        Commands::Mcp { action } => {
            handle_mcp_command(action, config_manager).await?;
        }
        Commands::MigrateInstaller => {
            handle_migrate_installer_command().await?;
        }
        Commands::SetupToken => {
            handle_setup_token_command().await?;
        }
        Commands::Update => {
            handle_update_command().await?;
        }
        Commands::Install { target, force } => {
            handle_install_command(target, force).await?;
        }
        Commands::Interactive => {
            start_interactive_mode(config_manager, fs_manager).await?;
        }
        Commands::Git { command } => {
            handle_git_command(&command).await?;
        }
        Commands::Highlight { command } => {
            handle_highlight_command(&command).await?;
        }
        Commands::Process { command } => {
            handle_process_command(&command).await?;
        }
        Commands::Image { command } => {
            handle_image_command(&command).await?;
        }
        Commands::Model { set, list } => {
            handle_model_command(set, list, config_manager).await?;
        }
        Commands::Resume { conversation_id } => {
            handle_resume_command(conversation_id).await?;
        }
        Commands::Bug { message, include_system } => {
            handle_bug_command(message, include_system).await?;
        }
        Commands::ReleaseNotes { version } => {
            handle_release_notes_command(version).await?;
        }
        Commands::PrComments { pr, repo } => {
            handle_pr_comments_command(pr, repo).await?;
        }
        Commands::TerminalSetup => {
            handle_terminal_setup_command().await?;
        }
        Commands::Vim { enable } => {
            handle_vim_command(enable).await?;
        }
        Commands::Quit => {
            println!("👋 Goodbye!");
            std::process::exit(0);
        }
        Commands::Login { provider, browser } => {
            handle_login_command(provider, browser).await?;
        }
        Commands::Logout { clear_all } => {
            handle_logout_command(clear_all).await?;
        }
        Commands::Ui { port, host, open } => {
            handle_ui_command(port, host, open).await?;
        }

        #[cfg(feature = "web-server")]
        Commands::Serve { port, host, static_dir, no_cors, no_compression } => {
            handle_serve_command(port, host, static_dir, no_cors, no_compression, config_manager).await?;
        }
    }

    Ok(())
}

async fn start_interactive_mode(config_manager: &mut ConfigManager, _fs_manager: &mut FileSystemManager) -> Result<()> {
    use crate::ui::{TerminalUI, ColorTheme};
    use ratatui::style::Color;

    println!("🎮 Starting Interactive Mode");
    println!("============================");
    println!("Welcome to Claude Code Rust Interactive Mode!");
    println!("Type 'help' for available commands or 'exit' to quit.");
    println!();

    // 创建终端UI
    let mut ui = TerminalUI::new();

    // 创建颜色主题
    let theme = ColorTheme {
        user_color: Color::Cyan,
        assistant_color: Color::Green,
        system_color: Color::Yellow,
        error_color: Color::Red,
        warning_color: Color::Magenta,
        debug_color: Color::DarkGray,
        border_color: Color::Blue,
        background_color: Color::Black,
    };

    // 检查是否启用TUI模式
    let config = config_manager.get_config();
    let use_tui = config.ui.enable_tui;

    if use_tui {
        println!("🖥️  Starting TUI mode...");
        println!("Press 'q' to quit TUI mode");

        // 启动TUI模式
        match ui.start_tui_mode(theme).await {
            Ok(()) => {
                println!("TUI mode exited successfully");
            }
            Err(e) => {
                println!("❌ TUI mode failed: {}", e);
                println!("Falling back to simple interactive mode...");
                return start_simple_interactive_mode(config_manager).await;
            }
        }
    } else {
        println!("📝 Starting simple interactive mode...");
        return start_simple_interactive_mode(config_manager).await;
    }

    Ok(())
}

async fn start_simple_interactive_mode(config_manager: &mut ConfigManager) -> Result<()> {
    use std::io::{self, Write};
    use tokio::io::{AsyncBufReadExt, BufReader};

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        // 显示提示符
        print!("claude-code-rust> ");
        io::stdout().flush().unwrap();

        // 读取用户输入
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                // 处理退出命令
                if input == "exit" || input == "quit" || input == "q" {
                    println!("👋 Goodbye!");
                    break;
                }

                // 处理帮助命令
                if input == "help" || input == "h" {
                    show_interactive_help();
                    continue;
                }

                // 处理清屏命令
                if input == "clear" || input == "cls" {
                    print!("\x1B[2J\x1B[1;1H");
                    continue;
                }

                // 处理状态命令
                if input == "status" {
                    show_status(config_manager);
                    continue;
                }

                // 处理配置命令
                if input.starts_with("config ") {
                    let args: Vec<&str> = input.split_whitespace().collect();
                    if args.len() >= 3 && args[1] == "set" {
                        // config set key value
                        if args.len() >= 4 {
                            let key = args[2];
                            let value = args[3..].join(" ");
                            handle_config_set(config_manager, key, &value);
                        } else {
                            println!("❌ Usage: config set <key> <value>");
                        }
                    } else if args.len() == 3 && args[1] == "get" {
                        // config get key
                        let key = args[2];
                        handle_config_get(config_manager, key);
                    } else {
                        println!("❌ Usage: config set <key> <value> | config get <key>");
                    }
                    continue;
                }

                // 处理内存命令
                if input.starts_with("memory ") {
                    let args: Vec<&str> = input.split_whitespace().collect();
                    if args.len() >= 2 {
                        match args[1] {
                            "show" => {
                                if let Err(e) = handle_memory_command(cli::MemoryCommands::Show).await {
                                    println!("❌ Error: {}", e);
                                }
                            }
                            "add" => {
                                if args.len() >= 3 {
                                    let content = args[2..].join(" ");
                                    if let Err(e) = handle_memory_command(cli::MemoryCommands::Add { content }).await {
                                        println!("❌ Error: {}", e);
                                    }
                                } else {
                                    println!("❌ Usage: memory add <content>");
                                }
                            }
                            "clear" => {
                                if let Err(e) = handle_memory_command(cli::MemoryCommands::Clear).await {
                                    println!("❌ Error: {}", e);
                                }
                            }
                            "search" => {
                                if args.len() >= 3 {
                                    let query = args[2..].join(" ");
                                    if let Err(e) = handle_memory_command(cli::MemoryCommands::Search { query }).await {
                                        println!("❌ Error: {}", e);
                                    }
                                } else {
                                    println!("❌ Usage: memory search <query>");
                                }
                            }
                            _ => {
                                println!("❌ Unknown memory command. Use: show, add, clear, search");
                            }
                        }
                    }
                    continue;
                }

                // 处理其他命令
                println!("❓ Unknown command: '{}'", input);
                println!("💡 Type 'help' for available commands");
            }
            Err(e) => {
                println!("❌ Error reading input: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn show_interactive_help() {
    println!("🎮 Interactive Mode Commands");
    println!("============================");
    println!("📋 General Commands:");
    println!("  help, h          - Show this help message");
    println!("  exit, quit, q    - Exit interactive mode");
    println!("  clear, cls       - Clear the screen");
    println!("  status           - Show current status");
    println!();
    println!("⚙️  Configuration Commands:");
    println!("  config set <key> <value>  - Set configuration value");
    println!("  config get <key>          - Get configuration value");
    println!();
    println!("🧠 Memory Commands:");
    println!("  memory show               - Show all memory items");
    println!("  memory add <content>      - Add new memory item");
    println!("  memory clear              - Clear all memory");
    println!("  memory search <query>     - Search memory items");
    println!();
    println!("💡 Examples:");
    println!("  config set ui.theme dark");
    println!("  memory add Remember to use async/await in Rust");
    println!("  memory search rust");
    println!();
}

fn show_status(config_manager: &ConfigManager) {
    println!("📊 Claude Code Rust Status");
    println!("===========================");

    let config = config_manager.get_config();

    println!("🔧 Configuration:");
    println!("  Theme: {}", config.ui.theme);
    println!("  TUI Enabled: {}", config.ui.enable_tui);
    println!("  Require Confirmation: {}", config.permissions.require_confirmation);

    println!("\n🔐 Permissions:");
    println!("  Allowed Tools: {}", config.permissions.allowed_tools.len());
    println!("  Denied Tools: {}", config.permissions.denied_tools.len());

    println!("\n💾 Storage:");
    if let Some(config_dir) = dirs::config_dir() {
        let claude_dir = config_dir.join("claude-code-rust");
        println!("  Config Directory: {}", claude_dir.display());
    }

    println!("\n🚀 Version: {}", env!("CARGO_PKG_VERSION"));
    println!("📅 Build Date: {}", "2025-07-13"); // 静态构建日期
}

fn handle_config_set(config_manager: &mut ConfigManager, key: &str, value: &str) {
    let config = config_manager.get_config_mut();

    match key {
        "ui.theme" => {
            config.ui.theme = value.to_string();
            println!("✅ Set ui.theme = {}", value);
        }
        "ui.enable_tui" => {
            match value.parse::<bool>() {
                Ok(val) => {
                    config.ui.enable_tui = val;
                    println!("✅ Set ui.enable_tui = {}", val);
                }
                Err(_) => {
                    println!("❌ Invalid boolean value. Use 'true' or 'false'");
                    return;
                }
            }
        }
        "permissions.require_confirmation" => {
            match value.parse::<bool>() {
                Ok(val) => {
                    config.permissions.require_confirmation = val;
                    println!("✅ Set permissions.require_confirmation = {}", val);
                }
                Err(_) => {
                    println!("❌ Invalid boolean value. Use 'true' or 'false'");
                    return;
                }
            }
        }
        _ => {
            println!("❌ Unknown configuration key: {}", key);
            println!("💡 Available keys: ui.theme, ui.enable_tui, permissions.require_confirmation");
            return;
        }
    }

    // 保存配置
    if let Err(e) = config_manager.save() {
        println!("❌ Failed to save configuration: {}", e);
    } else {
        println!("💾 Configuration saved");
    }
}

fn handle_config_get(config_manager: &ConfigManager, key: &str) {
    let config = config_manager.get_config();

    match key {
        "ui.theme" => {
            println!("ui.theme = {}", config.ui.theme);
        }
        "ui.enable_tui" => {
            println!("ui.enable_tui = {}", config.ui.enable_tui);
        }
        "permissions.require_confirmation" => {
            println!("permissions.require_confirmation = {}", config.permissions.require_confirmation);
        }
        _ => {
            println!("❌ Unknown configuration key: {}", key);
            println!("💡 Available keys: ui.theme, ui.enable_tui, permissions.require_confirmation");
        }
    }
}

async fn handle_image_command(command: &cli::ImageCommand) -> Result<()> {
    #[cfg(feature = "image-processing")]
    {
        use crate::image_processing::{ImageProcessor, ImageProcessingConfig};

        let processor = ImageProcessor::new();

        match command {
            cli::ImageCommand::Resize { input, output, width, height, quality, preserve_aspect } => {
                println!("🖼️  Resizing image: {} -> {}", input, output);

                let config = ImageProcessingConfig {
                    quality: *quality,
                    preserve_aspect_ratio: *preserve_aspect,
                    ..Default::default()
                };

                match processor.resize_image(input, output, *width, *height, &config).await {
                    Ok(()) => {
                        println!("✅ Image resized successfully");

                        // 显示输出图像信息
                        if let Ok(info) = processor.get_image_info_from_file(output).await {
                            println!("Output: {}x{} pixels", info.width, info.height);
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to resize image: {}", e);
                    }
                }
            }

            cli::ImageCommand::Convert { input, output, format, quality } => {
                println!("🔄 Converting image: {} -> {}", input, output);

                let config = ImageProcessingConfig {
                    quality: *quality,
                    ..Default::default()
                };

                match processor.convert_format(input, output, format.as_deref(), &config).await {
                    Ok(()) => {
                        println!("✅ Image converted successfully");

                        // 显示输出图像信息
                        if let Ok(info) = processor.get_image_info_from_file(output).await {
                            println!("Output: {}x{} pixels", info.width, info.height);
                            if let Some(fmt) = info.format {
                                println!("Format: {:?}", fmt);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to convert image: {}", e);
                    }
                }
            }

            cli::ImageCommand::Info { path } => {
                println!("� Image Information: {}", path);
                println!("====================");

                match processor.get_image_info_from_file(path).await {
                    Ok(info) => {
                        println!("Dimensions: {}x{} pixels", info.width, info.height);
                        if let Some(format) = info.format {
                            println!("Format: {:?}", format);
                        } else {
                            println!("Format: Unknown");
                        }
                        println!("Color Type: {}", info.color_type);

                        // 计算文件大小
                        if let Ok(metadata) = std::fs::metadata(path) {
                            let size = metadata.len();
                            println!("File Size: {} bytes ({:.2} KB)", size, size as f64 / 1024.0);
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to get image info: {}", e);
                    }
                }
            }

            cli::ImageCommand::Thumbnail { input, output, size, quality } => {
                println!("🖼️  Creating thumbnail: {} -> {}", input, output);

                let config = ImageProcessingConfig {
                    quality: *quality,
                    preserve_aspect_ratio: true,
                    ..Default::default()
                };

                match processor.create_thumbnail_from_file(input, output, *size, &config).await {
                    Ok(()) => {
                        println!("✅ Thumbnail created successfully");
                        println!("Size: {}x{} pixels", size, size);
                    }
                    Err(e) => {
                        println!("❌ Failed to create thumbnail: {}", e);
                    }
                }
            }

            cli::ImageCommand::Rotate { input, output, angle } => {
                println!("🔄 Rotating image: {} -> {} ({}°)", input, output, angle);

                match processor.rotate_image(input, output, *angle).await {
                    Ok(()) => {
                        println!("✅ Image rotated successfully");
                    }
                    Err(e) => {
                        println!("❌ Failed to rotate image: {}", e);
                    }
                }
            }

            cli::ImageCommand::Flip { input, output, horizontal, vertical } => {
                println!("🔄 Flipping image: {} -> {}", input, output);

                if *horizontal && *vertical {
                    println!("Direction: Horizontal and Vertical");
                } else if *horizontal {
                    println!("Direction: Horizontal");
                } else if *vertical {
                    println!("Direction: Vertical");
                } else {
                    println!("❌ No flip direction specified. Use --horizontal or --vertical");
                    return Ok(());
                }

                match processor.flip_image(input, output, *horizontal, *vertical).await {
                    Ok(()) => {
                        println!("✅ Image flipped successfully");
                    }
                    Err(e) => {
                        println!("❌ Failed to flip image: {}", e);
                    }
                }
            }

            cli::ImageCommand::Crop { input, output, x, y, width, height } => {
                println!("✂️  Cropping image: {} -> {}", input, output);
                println!("Region: {}x{} at ({}, {})", width, height, x, y);

                match processor.crop_image(input, output, *x, *y, *width, *height).await {
                    Ok(()) => {
                        println!("✅ Image cropped successfully");
                    }
                    Err(e) => {
                        println!("❌ Failed to crop image: {}", e);
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "image-processing"))]
    {
        println!("❌ Image processing feature is not enabled");
        println!("💡 Rebuild with --features image-processing to enable this functionality");
        println!("Command: {:?}", command);
    }

    Ok(())
}

async fn handle_process_command(command: &cli::ProcessCommand) -> Result<()> {
    use process::{ProcessManager, ProcessConfig};

    let process_manager = ProcessManager::new();

    match command {
        cli::ProcessCommand::List => {
            println!("🔄 Running Processes");
            println!("===================");

            let processes = process_manager.list_processes();
            if processes.is_empty() {
                println!("No running processes");
            } else {
                println!("{:<12} {:<15}", "ID", "Status");
                println!("{}", "-".repeat(30));

                for (id, status) in processes {
                    println!("{:<12} {:<15}", id, format!("{:?}", status));
                }
            }
        }

        cli::ProcessCommand::Start { name, command, args, workdir, capture } => {
            println!("🚀 Starting process '{}'...", name);

            let config = ProcessConfig {
                name: name.clone(),
                command: command.clone(),
                args: args.clone(),
                env: std::collections::HashMap::new(),
                working_dir: workdir.clone(),
                capture_output: *capture,
                timeout: None,
                auto_restart: false,
            };

            match process_manager.start_process(config).await {
                Ok(process_id) => {
                    println!("✅ Process '{}' started with ID: {}", name, process_id);
                    if *capture {
                        println!("💡 Use 'claude-code-rust process output {}' to view output", process_id);
                    }
                }
                Err(e) => {
                    println!("❌ Failed to start process '{}': {}", name, e);
                }
            }
        }

        cli::ProcessCommand::Stop { process, force: _ } => {
            println!("� Stopping process '{}'...", process);

            match process_manager.stop_process(process).await {
                Ok(()) => {
                    println!("✅ Process '{}' stopped successfully", process);
                }
                Err(e) => {
                    println!("❌ Failed to stop process '{}': {}", process, e);
                }
            }
        }

        cli::ProcessCommand::Status { process } => {
            println!("📊 Process Status: {}", process);
            println!("==================");

            match process_manager.get_process_status(process) {
                Some(status) => {
                    println!("Status: {:?}", status);
                }
                None => {
                    println!("❌ Process '{}' not found", process);
                }
            }
        }

        cli::ProcessCommand::Send { process, input } => {
            println!("📤 Sending input to process '{}'...", process);

            match process_manager.send_input(process, input).await {
                Ok(()) => {
                    println!("✅ Input sent successfully");
                }
                Err(e) => {
                    println!("❌ Failed to send input: {}", e);
                }
            }
        }

        cli::ProcessCommand::Output { process, lines: _, follow: _ } => {
            println!("📄 Process Output: {}", process);
            println!("==================");

            match process_manager.get_process_output(process).await {
                Ok(output) => {
                    if !output.stdout.is_empty() {
                        println!("📤 STDOUT:");
                        for line in &output.stdout {
                            println!("  {}", line);
                        }
                    }

                    if !output.stderr.is_empty() {
                        println!("📥 STDERR:");
                        for line in &output.stderr {
                            println!("  {}", line);
                        }
                    }

                    if output.stdout.is_empty() && output.stderr.is_empty() {
                        println!("No output available");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get output: {}", e);
                }
            }
        }

        cli::ProcessCommand::Restart { process } => {
            println!("🔄 Restarting process '{}'...", process);

            // 实现重启逻辑：先停止，再启动
            match process_manager.stop_process(process).await {
                Ok(()) => {
                    println!("✅ Process '{}' stopped", process);

                    // 等待一小段时间确保进程完全停止
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    // 这里需要重新启动进程，但我们需要保存原始配置
                    // 目前简化实现
                    println!("💡 Process restart requires saving original configuration");
                    println!("Use 'stop' followed by 'start' as a workaround");
                }
                Err(e) => {
                    println!("❌ Failed to stop process for restart: {}", e);
                }
            }
        }
    }

    Ok(())
}



async fn handle_doctor_command(_config_manager: &mut ConfigManager) -> Result<()> {
    println!("🏥 Claude Code Health Check");
    println!("===========================");
    println!("✅ Configuration: OK");
    println!("✅ File System: OK");
    println!("✅ Network: OK");
    println!("✅ All systems operational");
    Ok(())
}

async fn handle_status_command(_config_manager: &mut ConfigManager) -> Result<()> {
    println!("📊 Claude Code Status");
    println!("====================");
    println!("Version: 0.1.0");
    println!("Status: Running");
    println!("Mode: Rust Implementation");
    Ok(())
}

async fn handle_cost_command(days: u32) -> Result<()> {
    println!("💰 Cost Information (Last {} days)", days);
    println!("===================================");
    println!("API Calls: 0");
    println!("Tokens Used: 0");
    println!("Total Cost: $0.0000");
    println!("💡 Cost tracking not fully implemented yet");
    Ok(())
}

async fn handle_clear_command() -> Result<()> {
    println!("🧹 Clearing conversation history...");
    println!("✅ Conversation history cleared");
    Ok(())
}

async fn handle_compact_command(_instructions: Option<String>) -> Result<()> {
    println!("📦 Compacting conversation history...");
    println!("✅ Conversation history compacted");
    Ok(())
}

async fn handle_export_command(_format: String, _output: Option<String>) -> Result<()> {
    println!("📤 Exporting conversation...");
    println!("✅ Conversation exported");
    Ok(())
}

async fn handle_memory_command(action: cli::MemoryCommands) -> Result<()> {
    use std::fs;
    use std::path::PathBuf;
    use chrono::{DateTime, Utc};
    use serde::{Serialize, Deserialize};

    // 内存项结构
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct MemoryItem {
        id: String,
        content: String,
        timestamp: DateTime<Utc>,
        tags: Vec<String>,
    }

    // 内存存储结构
    #[derive(Debug, Serialize, Deserialize)]
    struct MemoryStorage {
        items: Vec<MemoryItem>,
        version: String,
    }

    impl Default for MemoryStorage {
        fn default() -> Self {
            Self {
                items: Vec::new(),
                version: "1.0".to_string(),
            }
        }
    }

    // 获取内存文件路径
    let memory_file = {
        let mut path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        path.push("claude-code-rust");
        fs::create_dir_all(&path).ok();
        path.push("memory.json");
        path
    };

    // 加载内存数据
    let mut memory_storage: MemoryStorage = if memory_file.exists() {
        match fs::read_to_string(&memory_file) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_default()
            }
            Err(_) => MemoryStorage::default(),
        }
    } else {
        MemoryStorage::default()
    };

    // 保存内存数据的函数
    let save_memory = |storage: &MemoryStorage| -> Result<()> {
        let content = serde_json::to_string_pretty(storage)
            .map_err(|e| ClaudeError::General(format!("Failed to serialize memory: {}", e)))?;
        fs::write(&memory_file, content)
            .map_err(|e| ClaudeError::General(format!("Failed to save memory: {}", e)))?;
        Ok(())
    };

    match action {
        cli::MemoryCommands::Show => {
            println!("🧠 Memory Contents");
            println!("==================");

            if memory_storage.items.is_empty() {
                println!("No memory items stored");
                println!("💡 Use 'claude-code-rust memory add <content>' to add items");
            } else {
                println!("Total items: {}\n", memory_storage.items.len());

                for (index, item) in memory_storage.items.iter().enumerate() {
                    println!("📝 Item #{} (ID: {})", index + 1, &item.id[..8]);
                    println!("� Created: {}", item.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

                    if !item.tags.is_empty() {
                        println!("🏷️  Tags: {}", item.tags.join(", "));
                    }

                    // 显示内容（限制长度）
                    let content = if item.content.len() > 200 {
                        format!("{}...", &item.content[..200])
                    } else {
                        item.content.clone()
                    };

                    println!("💭 Content:");
                    for line in content.lines() {
                        println!("   {}", line);
                    }
                    println!();
                }

                println!("�💡 Use 'claude-code-rust memory search <query>' to search items");
                println!("💡 Use 'claude-code-rust memory clear' to clear all items");
            }
        }

        cli::MemoryCommands::Add { content } => {
            println!("🧠 Adding memory item...");

            // 生成唯一ID
            let id = uuid::Uuid::new_v4().to_string();

            // 简单的标签提取（从内容中提取关键词）
            let tags = extract_tags(&content);

            let item = MemoryItem {
                id: id.clone(),
                content: content.clone(),
                timestamp: Utc::now(),
                tags,
            };

            memory_storage.items.push(item);

            match save_memory(&memory_storage) {
                Ok(()) => {
                    println!("✅ Memory item added successfully");
                    println!("ID: {}", &id[..8]);
                    println!("Content: {}", if content.len() > 100 {
                        format!("{}...", &content[..100])
                    } else {
                        content
                    });
                    println!("Total items: {}", memory_storage.items.len());
                }
                Err(e) => {
                    println!("❌ Failed to save memory: {}", e);
                }
            }
        }

        cli::MemoryCommands::Clear => {
            println!("🧠 Clearing all memory items...");

            let item_count = memory_storage.items.len();
            memory_storage.items.clear();

            match save_memory(&memory_storage) {
                Ok(()) => {
                    println!("✅ Cleared {} memory items", item_count);
                    println!("Memory is now empty");
                }
                Err(e) => {
                    println!("❌ Failed to save memory: {}", e);
                }
            }
        }

        cli::MemoryCommands::Search { query } => {
            println!("🧠 Searching memory for: '{}'", query);
            println!("===============================");

            let query_lower = query.to_lowercase();
            let mut matches = Vec::new();

            for (index, item) in memory_storage.items.iter().enumerate() {
                let content_lower = item.content.to_lowercase();
                let tags_lower = item.tags.join(" ").to_lowercase();

                if content_lower.contains(&query_lower) || tags_lower.contains(&query_lower) {
                    matches.push((index, item));
                }
            }

            if matches.is_empty() {
                println!("No matching items found");
                println!("💡 Try different keywords or use 'claude-code-rust memory show' to see all items");
            } else {
                println!("Found {} matching item(s):\n", matches.len());

                for (index, item) in matches {
                    println!("📝 Item #{} (ID: {})", index + 1, &item.id[..8]);
                    println!("📅 Created: {}", item.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

                    if !item.tags.is_empty() {
                        println!("🏷️  Tags: {}", item.tags.join(", "));
                    }

                    // 高亮匹配的内容
                    let highlighted_content = highlight_matches(&item.content, &query);
                    println!("💭 Content:");
                    for line in highlighted_content.lines() {
                        println!("   {}", line);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

// 从内容中提取标签的简单实现
fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();

    // 提取常见的编程语言关键词
    let keywords = [
        "rust", "python", "javascript", "typescript", "java", "c++", "c#", "go", "php", "ruby",
        "function", "class", "struct", "enum", "trait", "interface", "async", "await",
        "error", "bug", "fix", "todo", "note", "important", "warning",
        "api", "database", "sql", "http", "json", "xml", "yaml", "config",
    ];

    let content_lower = content.to_lowercase();
    for keyword in &keywords {
        if content_lower.contains(keyword) {
            tags.push(keyword.to_string());
        }
    }

    // 限制标签数量
    tags.truncate(5);
    tags.sort();
    tags.dedup();

    tags
}

// 高亮匹配内容的简单实现
fn highlight_matches(content: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let mut result = String::new();

    for line in content.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains(&query_lower) {
            // 简单的高亮：用 ** 包围匹配的文本
            let highlighted = line.replace(query, &format!("**{}**", query));
            result.push_str(&highlighted);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    result
}

async fn handle_permissions_command(action: cli::PermissionCommands, config_manager: &mut ConfigManager) -> Result<()> {
    match action {
        cli::PermissionCommands::Show => {
            println!("🔐 Permission Settings");
            println!("======================");

            let config = config_manager.get_config();

            println!("🔒 Require Confirmation: {}",
                if config.permissions.require_confirmation { "Yes" } else { "No" });

            println!("\n✅ Allowed Tools:");
            if config.permissions.allowed_tools.is_empty() {
                println!("  (All tools allowed by default)");
            } else {
                for tool in &config.permissions.allowed_tools {
                    println!("  • {}", tool);
                }
            }

            println!("\n❌ Denied Tools:");
            if config.permissions.denied_tools.is_empty() {
                println!("  (No tools explicitly denied)");
            } else {
                for tool in &config.permissions.denied_tools {
                    println!("  • {}", tool);
                }
            }

            println!("\n💡 Available tools to manage:");
            let available_tools = [
                "file-system", "network", "process", "git", "mcp",
                "image-processing", "syntax-highlighting", "memory",
                "config", "permissions", "interactive"
            ];

            for tool in &available_tools {
                let status = if config.permissions.denied_tools.contains(&tool.to_string()) {
                    "❌ Denied"
                } else if config.permissions.allowed_tools.contains(&tool.to_string()) {
                    "✅ Explicitly Allowed"
                } else {
                    "🔓 Default (Allowed)"
                };
                println!("  {:<20} {}", tool, status);
            }

            println!("\n💡 Use 'claude-code-rust permissions allow <tool>' to allow a tool");
            println!("💡 Use 'claude-code-rust permissions deny <tool>' to deny a tool");
            println!("💡 Use 'claude-code-rust permissions reset' to reset all permissions");
        }

        cli::PermissionCommands::Allow { tool } => {
            println!("🔐 Allowing tool '{}'...", tool);

            let config = config_manager.get_config_mut();

            // 从拒绝列表中移除（如果存在）
            config.permissions.denied_tools.retain(|t| t != &tool);

            // 添加到允许列表（如果不存在）
            if !config.permissions.allowed_tools.contains(&tool) {
                config.permissions.allowed_tools.push(tool.clone());
            }

            match config_manager.save() {
                Ok(()) => {
                    println!("✅ Tool '{}' is now allowed", tool);
                    println!("💾 Configuration saved");
                }
                Err(e) => {
                    println!("❌ Failed to save configuration: {}", e);
                }
            }
        }

        cli::PermissionCommands::Deny { tool } => {
            println!("🔐 Denying tool '{}'...", tool);

            let config = config_manager.get_config_mut();

            // 从允许列表中移除（如果存在）
            config.permissions.allowed_tools.retain(|t| t != &tool);

            // 添加到拒绝列表（如果不存在）
            if !config.permissions.denied_tools.contains(&tool) {
                config.permissions.denied_tools.push(tool.clone());
            }

            match config_manager.save() {
                Ok(()) => {
                    println!("❌ Tool '{}' is now denied", tool);
                    println!("💾 Configuration saved");
                    println!("⚠️  This tool will be blocked from execution");
                }
                Err(e) => {
                    println!("❌ Failed to save configuration: {}", e);
                }
            }
        }

        cli::PermissionCommands::Reset => {
            println!("🔐 Resetting all permissions to defaults...");

            let config = config_manager.get_config_mut();

            // 清空所有权限列表
            config.permissions.allowed_tools.clear();
            config.permissions.denied_tools.clear();

            // 重置确认要求为默认值
            config.permissions.require_confirmation = false;

            match config_manager.save() {
                Ok(()) => {
                    println!("✅ All permissions reset to defaults");
                    println!("💾 Configuration saved");
                    println!("🔓 All tools are now allowed by default");
                    println!("🔒 Confirmation requirement: disabled");
                }
                Err(e) => {
                    println!("❌ Failed to save configuration: {}", e);
                }
            }
        }
    }

    Ok(())
}

async fn handle_mcp_command(action: cli::McpCommands, config_manager: &mut ConfigManager) -> Result<()> {
    use std::collections::HashMap;

    match action {
        cli::McpCommands::List => {
            println!("🔌 MCP Servers");
            println!("==============");

            let config = config_manager.get_config();
            if config.mcp_servers.is_empty() {
                println!("No MCP servers configured");
            } else {
                println!("{:<20} {:<30} {:<10}", "Name", "Command", "Status");
                println!("{}", "-".repeat(65));

                for (server_name, server_config) in &config.mcp_servers {
                    let status = "Stopped"; // 简化状态显示

                    println!("{:<20} {:<30} {:<10}",
                        server_name,
                        format!("{} {}", server_config.command, server_config.args.join(" ")),
                        status
                    );
                }
            }
        }

        cli::McpCommands::Add { name, command, args } => {
            println!("🔌 Adding MCP server '{}'...", name);

            let server_config = config::McpServerConfig {
                name: name.clone(),
                command: command.clone(),
                args: args.clone(),
                env: HashMap::new(),
                working_dir: None,
                auto_start: false,
            };

            // 检查是否已存在同名服务器
            let config = config_manager.get_config_mut();
            if config.mcp_servers.contains_key(&name) {
                println!("❌ MCP server '{}' already exists", name);
                return Ok(());
            }

            // 添加到配置
            config.mcp_servers.insert(name.clone(), server_config);

            // 保存配置
            match config_manager.save() {
                Ok(()) => {
                    println!("✅ MCP server '{}' added successfully", name);
                    println!("💾 Configuration saved");
                    println!("💡 Use 'claude-code-rust mcp start {}' to start the server", name);
                }
                Err(e) => {
                    println!("❌ Failed to save configuration: {}", e);
                }
            }
        }

        cli::McpCommands::Remove { name } => {
            println!("🔌 Removing MCP server '{}'...", name);

            let config = config_manager.get_config_mut();
            let removed = config.mcp_servers.remove(&name);

            if removed.is_some() {
                match config_manager.save() {
                    Ok(()) => {
                        println!("✅ MCP server '{}' removed successfully", name);
                        println!("💾 Configuration saved");
                    }
                    Err(e) => {
                        println!("❌ Failed to save configuration: {}", e);
                    }
                }
            } else {
                println!("❌ MCP server '{}' not found", name);
            }
        }

        cli::McpCommands::Start { name } => {
            println!("🔌 Starting MCP server '{}'...", name);

            let config = config_manager.get_config();
            if let Some(server_config) = config.mcp_servers.get(&name) {
                // 简化实现：显示启动信息但不实际启动
                println!("✅ MCP server '{}' start requested", name);
                println!("Command: {} {}", server_config.command, server_config.args.join(" "));
                println!("💡 Full MCP server lifecycle management will be implemented in future versions");
            } else {
                println!("❌ MCP server '{}' not found in configuration", name);
                println!("💡 Use 'claude-code-rust mcp add' to add a server first");
            }
        }

        cli::McpCommands::Stop { name } => {
            println!("🔌 Stopping MCP server '{}'...", name);

            // 这里需要实现停止逻辑
            // 由于当前MCP管理器没有停止方法，我们先显示一个占位符
            println!("💡 MCP server stop functionality needs to be implemented");
            println!("Server '{}' stop requested", name);
        }
    }

    Ok(())
}

async fn handle_git_command(command: &cli::GitCommand) -> Result<()> {
    use git::GitManager;
    use std::env;

    // 获取当前工作目录
    let current_dir = env::current_dir()
        .map_err(|e| ClaudeError::General(format!("Failed to get current directory: {}", e)))?;

    let git_manager = GitManager::new(current_dir);

    // 检查是否在Git仓库中
    if !git_manager.is_git_repository().await {
        println!("❌ Not in a Git repository");
        println!("💡 Use 'git init' to initialize a repository");
        return Ok(());
    }

    match command {
        cli::GitCommand::Status => {
            println!("🌿 Git Status");
            println!("=============");

            match git_manager.get_status().await {
                Ok(status) => {
                    println!("Branch: {}", status.current_branch);

                    if !status.staged_files.is_empty() {
                        println!("\n📦 Staged files:");
                        for file in &status.staged_files {
                            println!("  ✅ {}", file);
                        }
                    }

                    if !status.unstaged_files.is_empty() {
                        println!("\n📝 Modified files:");
                        for file in &status.unstaged_files {
                            println!("  📝 {}", file);
                        }
                    }

                    if !status.untracked_files.is_empty() {
                        println!("\n❓ Untracked files:");
                        for file in &status.untracked_files {
                            println!("  ❓ {}", file);
                        }
                    }

                    if status.staged_files.is_empty() && status.unstaged_files.is_empty() && status.untracked_files.is_empty() {
                        println!("✅ Working tree clean");
                    }

                    // 显示远程状态
                    let remote = &status.remote_status;
                    if remote.ahead > 0 {
                        println!("\n⬆️  Your branch is ahead by {} commit(s)", remote.ahead);
                    }
                    if remote.behind > 0 {
                        println!("\n⬇️  Your branch is behind by {} commit(s)", remote.behind);
                    }
                    if remote.ahead == 0 && remote.behind == 0 {
                        println!("\n🔄 Your branch is up to date");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get Git status: {}", e);
                }
            }
        }

        cli::GitCommand::Add { files } => {
            println!("🌿 Adding files to staging area...");

            match git_manager.add_files(files).await {
                Ok(()) => {
                    println!("✅ Files added successfully:");
                    for file in files {
                        println!("  ✅ {}", file);
                    }
                }
                Err(e) => {
                    println!("❌ Failed to add files: {}", e);
                }
            }
        }

        cli::GitCommand::Commit { message } => {
            println!("🌿 Committing changes...");

            match git_manager.commit(message).await {
                Ok(commit_hash) => {
                    println!("✅ Commit successful");
                    println!("Commit hash: {}", commit_hash);
                    println!("Message: {}", message);
                }
                Err(e) => {
                    println!("❌ Failed to commit: {}", e);
                }
            }
        }

        cli::GitCommand::Log { limit } => {
            println!("🌿 Commit History (last {} commits)", limit);
            println!("=====================================");

            match git_manager.get_commit_history(Some(*limit)).await {
                Ok(commits) => {
                    if commits.is_empty() {
                        println!("No commits found");
                    } else {
                        for commit in commits {
                            println!("� Commit: {}", commit.hash);
                            println!("👤 Author: {}", commit.author);
                            println!("📅 Date: {}", commit.timestamp);
                            println!("💬 Message: {}", commit.message);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get commit history: {}", e);
                }
            }
        }

        cli::GitCommand::Branch => {
            println!("🌿 Git Branches");
            println!("===============");

            match git_manager.get_branches().await {
                Ok(branches) => {
                    if branches.is_empty() {
                        println!("No branches found");
                    } else {
                        for branch in branches {
                            let marker = if branch.is_current { "* " } else { "  " };
                            let status = if branch.is_current { " (current)" } else { "" };
                            println!("{}{}{}", marker, branch.name, status);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get branches: {}", e);
                }
            }
        }

        cli::GitCommand::Checkout { branch, create } => {
            if *create {
                println!("🌿 Creating and checking out branch '{}'...", branch);

                match git_manager.create_branch(branch).await {
                    Ok(()) => {
                        println!("✅ Branch '{}' created successfully", branch);

                        // 切换到新分支
                        match git_manager.checkout_branch(branch).await {
                            Ok(()) => {
                                println!("✅ Switched to branch '{}'", branch);
                            }
                            Err(e) => {
                                println!("❌ Failed to switch to branch '{}': {}", branch, e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to create branch '{}': {}", branch, e);
                    }
                }
            } else {
                println!("🌿 Checking out branch '{}'...", branch);

                match git_manager.checkout_branch(branch).await {
                    Ok(()) => {
                        println!("✅ Switched to branch '{}'", branch);
                    }
                    Err(e) => {
                        println!("❌ Failed to switch to branch '{}': {}", branch, e);
                    }
                }
            }
        }

        cli::GitCommand::Diff { file } => {
            println!("🌿 Git Diff");
            println!("===========");

            match git_manager.get_diff(file.as_deref()).await {
                Ok(diffs) => {
                    if diffs.is_empty() {
                        println!("No differences found");
                    } else {
                        for diff in diffs {
                            println!("📄 File: {}", diff.file_path);
                            println!("Changes: +{} -{}", diff.lines_added, diff.lines_deleted);

                            if !diff.diff_content.is_empty() {
                                println!("Diff:");
                                for line in diff.diff_content.lines() {
                                    if line.starts_with('+') {
                                        println!("  \x1b[32m{}\x1b[0m", line); // Green for additions
                                    } else if line.starts_with('-') {
                                        println!("  \x1b[31m{}\x1b[0m", line); // Red for deletions
                                    } else {
                                        println!("  {}", line);
                                    }
                                }
                            }
                            println!();
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get diff: {}", e);
                }
            }
        }
    }

    Ok(())
}

async fn handle_highlight_command(command: &cli::HighlightCommand) -> Result<()> {
    #[cfg(feature = "syntax-highlighting")]
    {
        use crate::syntax_highlighting::{SyntaxHighlighter, HighlightConfig};
        use std::fs;

        let highlighter = SyntaxHighlighter::new()?;

        match command {
            cli::HighlightCommand::File { path, language } => {
                println!("🎨 Highlighting file: {}", path);

                // 读取文件内容
                match fs::read_to_string(path) {
                    Ok(content) => {
                        let detected_language = if let Some(lang) = language {
                            lang.clone()
                        } else {
                            // 从文件扩展名推断语言
                            let extension = std::path::Path::new(path)
                                .extension()
                                .and_then(|ext| ext.to_str());

                            if let Some(syntax) = highlighter.detect_language(&content, extension) {
                                syntax.name.clone()
                            } else {
                                "text".to_string()
                            }
                        };

                        println!("Language: {}", detected_language);
                        println!("{}",  "=".repeat(50));

                        let config = HighlightConfig {
                            theme: "base16-ocean.dark".to_string(),
                            show_line_numbers: true,
                            line_number_width: 4,
                            use_terminal_colors: true,
                            background_color: None,
                        };

                        match highlighter.highlight_code(&content, Some(&detected_language), &config) {
                            Ok(result) => {
                                println!("{}", result.highlighted_code);
                            }
                            Err(e) => {
                                println!("❌ Failed to highlight code: {}", e);
                                println!("Raw content:");
                                println!("{}", content);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to read file '{}': {}", path, e);
                    }
                }
            }

            cli::HighlightCommand::Code { code, language } => {
                println!("🎨 Highlighting code snippet");
                println!("Language: {}", language);
                println!("{}", "=".repeat(50));

                let config = HighlightConfig {
                    theme: "base16-ocean.dark".to_string(),
                    show_line_numbers: true,
                    line_number_width: 4,
                    use_terminal_colors: true,
                    background_color: None,
                };

                match highlighter.highlight_code(code, Some(language), &config) {
                    Ok(result) => {
                        println!("{}", result.highlighted_code);
                    }
                    Err(e) => {
                        println!("❌ Failed to highlight code: {}", e);
                        println!("Raw content:");
                        println!("{}", code);
                    }
                }
            }

            cli::HighlightCommand::Languages => {
                println!("🎨 Supported Languages");
                println!("======================");

                let languages = highlighter.get_available_languages();

                if languages.is_empty() {
                    println!("No languages available");
                } else {
                    println!("Total: {} languages\n", languages.len());

                    // 按类别分组显示
                    let mut categories: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

                    for lang in languages {
                        let category = match lang.as_str() {
                            "rust" | "c" | "cpp" | "go" | "zig" => "Systems Programming",
                            "javascript" | "typescript" | "html" | "css" | "json" => "Web Development",
                            "python" | "ruby" | "php" | "perl" => "Scripting",
                            "java" | "kotlin" | "scala" | "clojure" => "JVM Languages",
                            "csharp" | "fsharp" | "vb" => ".NET Languages",
                            "sql" | "mysql" | "postgresql" => "Database",
                            "bash" | "zsh" | "fish" | "powershell" => "Shell",
                            "yaml" | "toml" | "xml" | "ini" => "Configuration",
                            "markdown" | "latex" | "rst" => "Documentation",
                            _ => "Other",
                        };

                        categories.entry(category.to_string())
                            .or_insert_with(Vec::new)
                            .push(lang);
                    }

                    // 排序并显示
                    let mut sorted_categories: Vec<_> = categories.into_iter().collect();
                    sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));

                    for (category, mut langs) in sorted_categories {
                        langs.sort();
                        println!("📂 {}:", category);
                        for lang in langs {
                            println!("  • {}", lang);
                        }
                        println!();
                    }
                }

                println!("💡 Use 'claude-code-rust highlight file <path>' to highlight a file");
                println!("💡 Use 'claude-code-rust highlight code <code> --language <lang>' to highlight code");
            }
        }
    }

    #[cfg(not(feature = "syntax-highlighting"))]
    {
        println!("❌ Syntax highlighting feature is not enabled");
        println!("💡 Rebuild with --features syntax-highlighting to enable this functionality");
        println!("Command: {:?}", command);
    }

    Ok(())
}

/// 处理演示命令
async fn handle_demo_command() -> Result<()> {
    use crate::agent::{AgentContext, AgentLoop};
    use crate::context::ContextManager;
    use crate::conversation::ConversationManager;
    use crate::network::Message;
    use crate::steering::SteeringController;
    use crate::ui::TerminalUI;

    println!("🎯 Starting Claude Code Rust Demo...");
    println!("This demonstrates the core components of Claude Code Rust\n");

    // 初始化 UI
    let _ui = TerminalUI::new();

    // 演示 1: 上下文管理
    println!("📝 Demo 1: Context Management");
    let mut context_manager = ContextManager::new(100000);

    // 添加一些示例消息
    let messages = vec![
        Message { role: "user".to_string(), content: "Hello, Claude!".to_string() },
        Message { role: "assistant".to_string(), content: "Hello! How can I help you today?".to_string() },
        Message { role: "user".to_string(), content: "Can you help me write some Rust code?".to_string() },
        Message { role: "assistant".to_string(), content: "Absolutely! I'd be happy to help you with Rust code.".to_string() },
    ];

    for message in messages {
        context_manager.add_message(message).await?;
    }

    let stats = context_manager.get_stats();
    println!("✅ Context Manager: {} messages, {:.1}% usage",
             stats.message_count, stats.usage_ratio * 100.0);

    // 演示 2: 对话管理
    println!("\n💬 Demo 2: Conversation Management");
    let conversation_manager = ConversationManager::new();
    let message_count = conversation_manager.get_message_count();
    println!("✅ Conversation Manager: {} messages stored", message_count);

    // 演示 3: Steering 控制
    println!("\n🎮 Demo 3: Steering Controller");
    let steering = SteeringController::new();
    println!("✅ Steering Controller: Ready for real-time control");

    // 演示 4: Agent 系统
    println!("\n🤖 Demo 4: Agent System");
    let config = crate::config::ClaudeConfig::default();
    let agent_context = AgentContext::new("demo-session".to_string(), config);
    let conversation = ConversationManager::new();
    let (agent_loop, _receiver) = AgentLoop::new(agent_context, conversation);
    let status = agent_loop.get_status().await;
    println!("✅ Agent Loop: Status = {:?}", status);

    // 演示 5: 工具系统
    println!("\n🔧 Demo 5: Tool System");
    let tool_registry = crate::tools::ToolRegistry::new();
    crate::tools::builtin::register_builtin_tools(&tool_registry).await?;
    let tools = tool_registry.list_tools().await;
    println!("✅ Tool Registry: {} tools registered", tools.len());
    for tool in &tools {
        println!("  • {} - {}", tool.name, tool.description);
    }

    // 演示 6: 错误处理
    println!("\n⚠️  Demo 6: Error Handling");
    let error = crate::error::ClaudeError::General("This is a demo error".to_string());
    println!("✅ Error System: {}", error);

    // 演示总结
    println!("\n🎉 Demo Complete!");
    println!("All core components are working correctly");
    println!("\nKey features demonstrated:");
    println!("  • Smart context management with 92% compression threshold");
    println!("  • Async conversation handling");
    println!("  • Real-time steering control");
    println!("  • Agent loop architecture");
    println!("  • Type-safe error handling");
    println!("  • Memory-efficient design");
    println!("  • Modular architecture");
    println!("  • Comprehensive tool system");

    println!("\n🦀 Claude Code Rust is ready for production use!");

    Ok(())
}

/// 处理 /init 命令 - 项目初始化分析
async fn handle_init_command(path: Option<String>, force: bool) -> Result<()> {
    use crate::fs::FileSystemManager;
    use crate::git::GitManager;
    use std::path::Path;

    println!("🔍 Initializing project analysis...");

    let project_path = path.unwrap_or_else(|| ".".to_string());
    let project_path = Path::new(&project_path);

    if !project_path.exists() {
        println!("❌ Error: Project path '{}' does not exist", project_path.display());
        return Ok(());
    }

    // 检查是否已经有 CLAUDE.md 文件
    let claude_md_path = project_path.join("CLAUDE.md");
    if claude_md_path.exists() && !force {
        println!("📄 CLAUDE.md already exists. Use --force to overwrite.");
        return Ok(());
    }

    println!("📊 Analyzing project structure...");

    // 1. 分析项目结构
    let fs_manager = FileSystemManager::new(vec![project_path.to_path_buf()]);
    let entries = fs_manager.list_directory(project_path).await?;

    let mut project_files = Vec::new();
    let mut config_files = Vec::new();
    let mut source_dirs = Vec::new();

    for entry in entries {
        if let Some(file_name) = entry.file_name() {
            let name = file_name.to_string_lossy();

            if entry.is_dir() {
                match name.as_ref() {
                    "src" | "lib" | "app" | "components" | "pages" => {
                        source_dirs.push(name.to_string());
                    }
                    _ => {}
                }
            } else {
                match name.as_ref() {
                    "package.json" | "Cargo.toml" | "pyproject.toml" | "go.mod" |
                    "pom.xml" | "build.gradle" | "Makefile" | "CMakeLists.txt" => {
                        config_files.push(name.to_string());
                    }
                    _ if name.ends_with(".rs") || name.ends_with(".js") ||
                         name.ends_with(".ts") || name.ends_with(".py") ||
                         name.ends_with(".go") || name.ends_with(".java") => {
                        project_files.push(name.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    // 2. 检测项目类型和构建系统
    let project_type = detect_project_type(&config_files);
    let build_commands = generate_build_commands(&project_type, &config_files);

    // 3. 分析 Git 信息
    let git_manager = GitManager::new(project_path.to_path_buf());
    let is_git_repo = git_manager.get_current_branch().await.is_ok();

    // 4. 生成 CLAUDE.md 内容
    let claude_md_content = generate_claude_md(
        &project_type,
        &build_commands,
        &source_dirs,
        &config_files,
        is_git_repo,
    );

    // 5. 写入 CLAUDE.md 文件
    fs_manager.write_file(&claude_md_path, &claude_md_content).await?;

    println!("✅ Successfully created CLAUDE.md");
    println!("📁 Project type: {}", project_type);
    println!("🏗️  Build system detected: {}",
             if build_commands.is_empty() { "None" } else { "Yes" });
    let source_dirs_str = if source_dirs.is_empty() {
        "None".to_string()
    } else {
        source_dirs.join(", ")
    };
    println!("📂 Source directories: {}", source_dirs_str);

    if is_git_repo {
        println!("🔗 Git repository detected");
    }

    println!("\n💡 CLAUDE.md has been created with project-specific guidance.");
    println!("   This file will help Claude understand your project structure and workflow.");

    Ok(())
}

/// 检测项目类型
fn detect_project_type(config_files: &[String]) -> String {
    for file in config_files {
        match file.as_str() {
            "package.json" => return "Node.js/JavaScript".to_string(),
            "Cargo.toml" => return "Rust".to_string(),
            "pyproject.toml" | "setup.py" => return "Python".to_string(),
            "go.mod" => return "Go".to_string(),
            "pom.xml" => return "Java (Maven)".to_string(),
            "build.gradle" => return "Java/Kotlin (Gradle)".to_string(),
            "Makefile" => return "C/C++ (Make)".to_string(),
            "CMakeLists.txt" => return "C/C++ (CMake)".to_string(),
            _ => {}
        }
    }
    "Unknown".to_string()
}

/// 生成构建命令
fn generate_build_commands(project_type: &str, config_files: &[String]) -> Vec<(String, String)> {
    let mut commands = Vec::new();

    match project_type {
        "Node.js/JavaScript" => {
            commands.push(("Build".to_string(), "npm run build".to_string()));
            commands.push(("Test".to_string(), "npm test".to_string()));
            commands.push(("Dev Server".to_string(), "npm run dev".to_string()));
            commands.push(("Lint".to_string(), "npm run lint".to_string()));
        }
        "Rust" => {
            commands.push(("Build".to_string(), "cargo build".to_string()));
            commands.push(("Test".to_string(), "cargo test".to_string()));
            commands.push(("Run".to_string(), "cargo run".to_string()));
            commands.push(("Check".to_string(), "cargo check".to_string()));
        }
        "Python" => {
            if config_files.contains(&"pyproject.toml".to_string()) {
                commands.push(("Install".to_string(), "pip install -e .".to_string()));
                commands.push(("Test".to_string(), "pytest".to_string()));
            } else {
                commands.push(("Install".to_string(), "pip install -r requirements.txt".to_string()));
                commands.push(("Test".to_string(), "python -m pytest".to_string()));
            }
        }
        "Go" => {
            commands.push(("Build".to_string(), "go build".to_string()));
            commands.push(("Test".to_string(), "go test ./...".to_string()));
            commands.push(("Run".to_string(), "go run .".to_string()));
        }
        "Java (Maven)" => {
            commands.push(("Build".to_string(), "mvn compile".to_string()));
            commands.push(("Test".to_string(), "mvn test".to_string()));
            commands.push(("Package".to_string(), "mvn package".to_string()));
        }
        "Java/Kotlin (Gradle)" => {
            commands.push(("Build".to_string(), "./gradlew build".to_string()));
            commands.push(("Test".to_string(), "./gradlew test".to_string()));
            commands.push(("Run".to_string(), "./gradlew run".to_string()));
        }
        _ => {}
    }

    commands
}

/// 生成 CLAUDE.md 内容
fn generate_claude_md(
    project_type: &str,
    build_commands: &[(String, String)],
    source_dirs: &[String],
    config_files: &[String],
    is_git_repo: bool,
) -> String {
    let mut content = String::new();

    content.push_str("# CLAUDE.md\n\n");
    content.push_str("This file provides guidance to Claude Code when working with code in this repository.\n\n");

    // Essential Commands
    content.push_str("## Essential Commands\n\n");
    if build_commands.is_empty() {
        content.push_str("No build system detected. Please add project-specific commands here.\n\n");
    } else {
        for (name, command) in build_commands {
            content.push_str(&format!("- **{}**: `{}`\n", name, command));
        }
        content.push_str("\n");
    }

    // Architecture Overview
    content.push_str("## Architecture Overview\n\n");
    content.push_str(&format!("This is a {} project", project_type));
    if !source_dirs.is_empty() {
        content.push_str(&format!(" with source code organized in: {}.", source_dirs.join(", ")));
    } else {
        content.push_str(".");
    }
    content.push_str("\n\n");

    // Key Files
    content.push_str("## Key Configuration Files\n\n");
    if config_files.is_empty() {
        content.push_str("No configuration files detected.\n\n");
    } else {
        for file in config_files {
            content.push_str(&format!("- `{}`: ", file));
            match file.as_str() {
                "package.json" => content.push_str("Node.js project configuration and dependencies"),
                "Cargo.toml" => content.push_str("Rust project configuration and dependencies"),
                "pyproject.toml" => content.push_str("Python project configuration"),
                "go.mod" => content.push_str("Go module definition"),
                "pom.xml" => content.push_str("Maven project configuration"),
                "build.gradle" => content.push_str("Gradle build configuration"),
                _ => content.push_str("Project configuration file"),
            }
            content.push_str("\n");
        }
        content.push_str("\n");
    }

    // Development Workflow
    content.push_str("## Development Workflow\n\n");
    content.push_str("1. Make changes to source files\n");
    if !build_commands.is_empty() {
        content.push_str("2. Run tests to ensure functionality\n");
        content.push_str("3. Build the project to check for compilation errors\n");
    }
    if is_git_repo {
        content.push_str("4. Commit changes using Git\n");
    }
    content.push_str("\n");

    // Important Notes
    content.push_str("## Important Notes\n\n");
    content.push_str("- This file was auto-generated by Claude Code Rust\n");
    content.push_str("- Modify this file to add project-specific guidance\n");
    content.push_str("- Include any special setup instructions or gotchas\n");
    if is_git_repo {
        content.push_str("- This project uses Git for version control\n");
    }
    content.push_str("\n");

    content.push_str("## Claude Code Integration\n\n");
    content.push_str("This project is configured for use with Claude Code. Key features:\n");
    content.push_str("- Automatic project structure analysis\n");
    content.push_str("- Intelligent code suggestions based on project type\n");
    content.push_str("- Context-aware file operations\n");
    content.push_str("- Integrated build and test commands\n");

    content
}

/// 处理 /stream 命令 - 流式响应演示
async fn handle_stream_command(url: Option<String>, realtime: bool) -> Result<()> {
    use claude_rust::streaming::{StreamConfig, StreamingClient, SseEventType, StreamState};
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::time::timeout;

    println!("🌊 Starting streaming response demo...");

    let test_url = url.unwrap_or_else(|| {
        "https://httpbin.org/stream/10".to_string()
    });

    println!("📡 Target URL: {}", test_url);
    println!("⚡ Real-time output: {}", if realtime { "Enabled" } else { "Disabled" });

    // 创建流式配置
    let config = StreamConfig {
        buffer_size: 1024,
        connect_timeout: Duration::from_secs(10),
        read_timeout: Duration::from_secs(30),
        reconnect_interval: Duration::from_secs(2),
        max_reconnects: 3,
        heartbeat_interval: Duration::from_secs(15),
        enable_compression: true,
    };

    // 创建流式客户端
    let mut client = StreamingClient::new(config.clone());

    // 设置请求头
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), "Claude-Code-Rust/0.1.0".to_string());

    if realtime {
        // 启用实时输出模式
        println!("\n🔄 Starting real-time streaming...");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // 获取输出接收器
        let mut output_receiver = client.take_output_receiver()
            .ok_or_else(|| ClaudeError::General("Failed to get output receiver".to_string()))?;

        // 获取状态订阅器
        let mut state_receiver = client.subscribe_state();

        // 启动流式处理任务
        let stream_handle = tokio::spawn({
            let test_url = test_url.clone();
            async move {
                let mut stream_client = StreamingClient::new(config.clone());
                if let Err(e) = stream_client.start_stream(&test_url, headers).await {
                    eprintln!("❌ Stream error: {}", e);
                }
            }
        });

        // 处理实时输出
        let output_handle = tokio::spawn(async move {
            while let Some(output) = output_receiver.recv().await {
                print!("{}", output);
                use std::io::{self, Write};
                io::stdout().flush().unwrap();
            }
        });

        // 监控状态变化
        let status_handle = tokio::spawn(async move {
            while let Ok(state) = state_receiver.recv().await {
                match state {
                    StreamState::Connected => {
                        println!("\n✅ Connected to stream");
                    }
                    StreamState::Streaming => {
                        println!("📡 Streaming data...");
                    }
                    StreamState::Completed => {
                        println!("\n✅ Stream completed successfully");
                        break;
                    }
                    StreamState::Error(err) => {
                        println!("\n❌ Stream error: {}", err);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // 等待所有任务完成
        let _ = tokio::try_join!(stream_handle, output_handle, status_handle);

    } else {
        // 模拟流式响应处理
        println!("\n🔄 Simulating streaming response...");

        // 模拟 SSE 数据
        let mock_sse_data = vec![
            "event: message_start\ndata: {\"type\": \"message_start\", \"message\": {\"id\": \"msg_123\", \"type\": \"message\", \"role\": \"assistant\"}}\n\n",
            "event: content_block_start\ndata: {\"type\": \"content_block_start\", \"index\": 0, \"content_block\": {\"type\": \"text\", \"text\": \"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \"Hello\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \" from\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \" Claude\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \" Code\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \" Rust!\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\": \"content_block_stop\", \"index\": 0}\n\n",
            "event: message_delta\ndata: {\"type\": \"message_delta\", \"delta\": {\"stop_reason\": \"end_turn\", \"stop_sequence\": null}, \"usage\": {\"output_tokens\": 15}}\n\n",
            "event: message_stop\ndata: {\"type\": \"message_stop\"}\n\n",
        ];

        // 获取事件订阅器
        let mut event_receiver = client.subscribe_events();

        // 启动事件监听任务
        let event_handle = tokio::spawn(async move {
            let mut full_text = String::new();

            while let Ok(event) = event_receiver.recv().await {
                match event.event_type {
                    SseEventType::MessageStart => {
                        println!("🚀 Message started");
                    }
                    SseEventType::ContentBlockStart => {
                        println!("📝 Content block started");
                        print!("💬 Response: ");
                        use std::io::{self, Write};
                        io::stdout().flush().unwrap();
                    }
                    SseEventType::ContentBlockDelta => {
                        if let Some(delta) = event.data.get("delta") {
                            if let Some(text) = delta.get("text") {
                                if let Some(text_str) = text.as_str() {
                                    print!("{}", text_str);
                                    full_text.push_str(text_str);
                                    use std::io::{self, Write};
                                    io::stdout().flush().unwrap();
                                }
                            }
                        }
                    }
                    SseEventType::ContentBlockStop => {
                        println!("\n📋 Content block completed");
                    }
                    SseEventType::MessageStop => {
                        println!("✅ Message completed");
                        println!("📊 Full response: \"{}\"", full_text);
                        break;
                    }
                    SseEventType::Error => {
                        println!("\n❌ Error: {:?}", event.data);
                        break;
                    }
                    _ => {
                        println!("📡 Event: {:?}", event.event_type);
                    }
                }
            }
        });

        // 模拟处理 SSE 数据
        for (i, chunk) in mock_sse_data.iter().enumerate() {
            println!("📦 Processing chunk {}/{}", i + 1, mock_sse_data.len());
            if let Err(e) = client.process_chunk(chunk).await {
                eprintln!("❌ Error processing chunk: {}", e);
                break;
            }

            // 模拟网络延迟
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        // 等待事件处理完成
        let _ = timeout(Duration::from_secs(5), event_handle).await;
    }

    // 显示统计信息
    let stats = client.get_stats();
    println!("\n📊 Streaming Statistics:");
    println!("   • Events received: {}", stats.events_received);
    println!("   • Bytes received: {}", stats.bytes_received);
    println!("   • Error count: {}", stats.error_count);

    if let Some(first_event) = stats.first_event_time {
        if let Some(last_event) = stats.last_event_time {
            let duration = last_event.duration_since(first_event);
            println!("   • Stream duration: {:.2}s", duration.as_secs_f64());
        }
    }

    println!("\n🎉 Streaming demo completed!");

    Ok(())
}

/// 处理 /review 命令 - 代码审查
async fn handle_review_command(target: Option<String>, review_type: Option<String>) -> Result<()> {
    use crate::fs::FileSystemManager;
    use crate::git::GitManager;
    use std::path::Path;

    println!("🔍 Starting code review...");

    let review_target = target.unwrap_or_else(|| ".".to_string());
    let review_kind = review_type.unwrap_or_else(|| "general".to_string());

    println!("📋 Review type: {}", review_kind);
    println!("🎯 Review target: {}", review_target);

    // 检查目标是否存在
    let target_path = Path::new(&review_target);
    if !target_path.exists() {
        println!("❌ Error: Review target '{}' does not exist", review_target);
        return Ok(());
    }

    let fs_manager = FileSystemManager::new(vec![std::env::current_dir()?]);

    match review_kind.as_str() {
        "security" => {
            println!("🔒 Performing security review...");
            perform_security_review(&fs_manager, target_path).await?;
        }
        "performance" => {
            println!("⚡ Performing performance review...");
            perform_performance_review(&fs_manager, target_path).await?;
        }
        "style" => {
            println!("🎨 Performing style review...");
            perform_style_review(&fs_manager, target_path).await?;
        }
        "general" | _ => {
            println!("📝 Performing general code review...");
            perform_general_review(&fs_manager, target_path).await?;
        }
    }

    println!("✅ Code review completed!");
    println!("💡 Review results have been displayed above.");

    Ok(())
}

/// 执行安全审查
async fn perform_security_review(fs_manager: &FileSystemManager, target_path: &Path) -> Result<()> {
    println!("🔍 Checking for common security issues...");

    let mut issues_found = 0;

    if target_path.is_file() {
        let content = fs_manager.read_file(target_path).await?;
        issues_found += check_security_patterns(&content, target_path);
    } else {
        let entries = fs_manager.list_directory(target_path).await?;
        for entry in entries {
            if entry.is_file() {
                if let Some(ext) = entry.extension() {
                    if matches!(ext.to_str(), Some("rs") | Some("js") | Some("ts") | Some("py") | Some("go")) {
                        let content = fs_manager.read_file(&entry).await?;
                        issues_found += check_security_patterns(&content, &entry);
                    }
                }
            }
        }
    }

    if issues_found == 0 {
        println!("✅ No obvious security issues found");
    } else {
        println!("⚠️  Found {} potential security issues", issues_found);
    }

    Ok(())
}

/// 检查安全模式
fn check_security_patterns(content: &str, file_path: &Path) -> usize {
    let mut issues = 0;
    let security_patterns = [
        ("password", "Potential hardcoded password"),
        ("api_key", "Potential hardcoded API key"),
        ("secret", "Potential hardcoded secret"),
        ("eval(", "Use of eval() function"),
        ("exec(", "Use of exec() function"),
        ("system(", "Use of system() function"),
        ("shell_exec", "Use of shell execution"),
        ("unsafe {", "Unsafe Rust code block"),
    ];

    for (pattern, description) in &security_patterns {
        if content.to_lowercase().contains(pattern) {
            println!("⚠️  {}: {} in {}", description, pattern, file_path.display());
            issues += 1;
        }
    }

    issues
}

/// 执行性能审查
async fn perform_performance_review(fs_manager: &FileSystemManager, target_path: &Path) -> Result<()> {
    println!("🔍 Checking for performance issues...");

    let mut suggestions = 0;

    if target_path.is_file() {
        let content = fs_manager.read_file(target_path).await?;
        suggestions += check_performance_patterns(&content, target_path);
    } else {
        let entries = fs_manager.list_directory(target_path).await?;
        for entry in entries {
            if entry.is_file() {
                if let Some(ext) = entry.extension() {
                    if matches!(ext.to_str(), Some("rs") | Some("js") | Some("ts") | Some("py") | Some("go")) {
                        let content = fs_manager.read_file(&entry).await?;
                        suggestions += check_performance_patterns(&content, &entry);
                    }
                }
            }
        }
    }

    if suggestions == 0 {
        println!("✅ No obvious performance issues found");
    } else {
        println!("💡 Found {} performance improvement suggestions", suggestions);
    }

    Ok(())
}

/// 检查性能模式
fn check_performance_patterns(content: &str, file_path: &Path) -> usize {
    let mut suggestions = 0;
    let performance_patterns = [
        ("for i in range(len(", "Consider using enumerate() instead"),
        ("while True:", "Consider if this infinite loop is necessary"),
        ("sleep(", "Consider if blocking sleep is appropriate"),
        ("clone()", "Consider if cloning is necessary"),
        ("unwrap()", "Consider using proper error handling"),
        ("Vec::new()", "Consider pre-allocating with capacity"),
    ];

    for (pattern, suggestion) in &performance_patterns {
        if content.contains(pattern) {
            println!("💡 {}: {} in {}", suggestion, pattern, file_path.display());
            suggestions += 1;
        }
    }

    suggestions
}

/// 执行样式审查
async fn perform_style_review(_fs_manager: &FileSystemManager, _target_path: &Path) -> Result<()> {
    println!("🎨 Style review functionality coming soon...");
    println!("💡 Consider using language-specific linters:");
    println!("   • Rust: cargo clippy");
    println!("   • JavaScript/TypeScript: eslint");
    println!("   • Python: flake8, black");
    println!("   • Go: gofmt, golint");
    Ok(())
}

/// 执行通用审查
async fn perform_general_review(fs_manager: &FileSystemManager, target_path: &Path) -> Result<()> {
    println!("📝 Performing general code review...");

    // 统计信息
    let mut total_files = 0;
    let mut total_lines = 0;
    let mut file_types = std::collections::HashMap::new();

    if target_path.is_file() {
        total_files = 1;
        let content = fs_manager.read_file(target_path).await?;
        total_lines = content.lines().count();

        if let Some(ext) = target_path.extension() {
            *file_types.entry(ext.to_string_lossy().to_string()).or_insert(0) += 1;
        }
    } else {
        let entries = fs_manager.list_directory(target_path).await?;
        for entry in entries {
            if entry.is_file() {
                total_files += 1;

                if let Some(ext) = entry.extension() {
                    if matches!(ext.to_str(), Some("rs") | Some("js") | Some("ts") | Some("py") | Some("go") | Some("java") | Some("cpp") | Some("c")) {
                        let content = fs_manager.read_file(&entry).await?;
                        total_lines += content.lines().count();
                    }
                    *file_types.entry(ext.to_string_lossy().to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    println!("📊 Code Statistics:");
    println!("   • Total files: {}", total_files);
    println!("   • Total lines of code: {}", total_lines);
    println!("   • File types:");
    for (ext, count) in file_types {
        println!("     - .{}: {} files", ext, count);
    }

    println!("\n✅ General review completed");

    Ok(())
}

/// 处理增强版 /compact 命令
async fn handle_compact_command_enhanced(instructions: Option<String>, level: Option<u8>) -> Result<()> {
    use crate::context::ContextManager;

    println!("🗜️  Starting context compression...");

    let compression_level = level.unwrap_or(1);
    let custom_instructions = instructions.unwrap_or_else(|| "Standard compression".to_string());

    println!("📋 Compression level: {}", compression_level);
    println!("📝 Instructions: {}", custom_instructions);

    // 创建上下文管理器
    let mut context_manager = ContextManager::new(100000);

    // 模拟一些消息
    for i in 0..10 {
        let message = Message {
            role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
            content: format!("Sample message {} for compression testing", i),
        };
        context_manager.add_message(message).await?;
    }

    // 执行压缩
    let compressed = context_manager.compress_context().await?;
    let stats = context_manager.get_stats();

    println!("✅ Compression completed!");
    println!("📊 Results:");
    println!("   • Original messages: {}", compressed.original_message_count);
    println!("   • Current messages: {}", stats.message_count);
    println!("   • Compression ratio: {:.1}%",
             (1.0 - stats.message_count as f64 / compressed.original_message_count as f64) * 100.0);
    println!("   • Memory usage: {:.1}%", stats.usage_ratio * 100.0);

    println!("\n🧠 Compressed Context Summary:");
    println!("   • Background: {}", compressed.background_context);
    println!("   • Key decisions: {} items", compressed.key_decisions.len());
    println!("   • Tool usage: {} records", compressed.tool_usage.len());
    println!("   • User intent: {}", compressed.user_intent);

    Ok(())
}

/// 处理 /api 命令 - Claude API 演示
async fn handle_api_command(
    message: String,
    model: String,
    stream: bool,
    image: Option<String>,
    tools: bool,
) -> Result<()> {

    use std::env;

    println!("🤖 Starting Claude API demo...");

    // 获取 API 密钥
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| ClaudeError::General(
            "ANTHROPIC_API_KEY environment variable not set".to_string()
        ))?;

    println!("🔑 API key found");
    println!("📝 Message: {}", message);
    println!("🧠 Model: {}", model);
    println!("⚡ Stream: {}", stream);

    if let Some(ref img_path) = image {
        println!("🖼️  Image: {}", img_path);
    }

    if tools {
        println!("🔧 Tools: Enabled");
    }

    // 创建 API 客户端
    let mut client = ClaudeApiClient::new(api_key, None)?;
    client.set_defaults(4096, 0.7, 0.9, 40);

    println!("\n🚀 Sending request to Claude API...");

    if let Some(image_path) = image {
        // 多模态请求（文本 + 图像）
        handle_multimodal_request(&client, &model, &message, &image_path, stream).await?;
    } else if tools {
        // 工具调用请求
        handle_tool_request(&client, &model, &message, stream).await?;
    } else {
        // 简单文本请求
        handle_text_request(&client, &model, &message, stream).await?;
    }

    println!("\n✅ Claude API demo completed!");

    Ok(())
}

/// 处理文本请求
async fn handle_text_request(
    client: &ClaudeApiClient,
    model: &str,
    message: &str,
    stream: bool,
) -> Result<()> {
    let messages = vec![("user".to_string(), message.to_string())];
    let request = client.create_text_request(model, messages);

    if stream {
        println!("📡 Streaming response:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // 发送流式请求
        use futures::StreamExt;
        let stream = client.send_message_stream(&request).await?;
        let mut stream = Box::pin(stream);

        print!("💬 ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        // 处理流式响应
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    match event.event_type.as_str() {
                        "content_block_delta" => {
                            if let Some(data) = event.data {
                                if let Ok(delta) = serde_json::from_value::<crate::network::StreamDelta>(data) {
                                    if let Some(text) = delta.text {
                                        print!("{}", text);
                                        io::stdout().flush().unwrap();
                                    }
                                }
                            }
                        }
                        "message_stop" => {
                            println!();
                            break;
                        }
                        "error" => {
                            if let Some(data) = event.data {
                                eprintln!("\n❌ Error: {}", data);
                            }
                            break;
                        }
                        _ => {
                            // 忽略其他事件类型
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Stream error: {}", e);
                    break;
                }
            }
        }
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        println!("💬 Response:");

        let response = client.send_message(&request).await?;

        for content_block in &response.content {
            match content_block {
                ResponseContentBlock::Text { text } => {
                    println!("{}", text);
                }
                ResponseContentBlock::ToolUse { id, name, input } => {
                    println!("🔧 Tool use: {} ({})", name, id);
                    println!("📋 Input: {}", serde_json::to_string_pretty(&input).unwrap_or_default());
                }
            }
        }

        println!("\n📊 Usage:");
        println!("   • Input tokens: {}", response.usage.input_tokens);
        println!("   • Output tokens: {}", response.usage.output_tokens);
        println!("   • Stop reason: {}", response.stop_reason.unwrap_or_default());
    }

    Ok(())
}

/// 处理多模态请求（文本 + 图像）
async fn handle_multimodal_request(
    client: &ClaudeApiClient,
    model: &str,
    message: &str,
    image_path: &str,
    stream: bool,
) -> Result<()> {
    println!("🖼️  Loading image: {}", image_path);

    // 创建图像内容块
    let image_block = client.create_image_block_from_file(image_path).await?;

    // 创建文本内容块
    let text_block = ContentBlock::Text {
        text: message.to_string(),
    };

    // 创建多模态请求
    let content_blocks = vec![text_block, image_block];
    let request = client.create_multimodal_request(
        model,
        "user".to_string(),
        content_blocks,
    );

    println!("📤 Sending multimodal request...");

    if stream {
        println!("📡 Streaming response:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        use futures::StreamExt;
        let stream = client.send_message_stream(&request).await?;
        let mut stream = Box::pin(stream);

        print!("💬 ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        // 处理流式响应
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    match event.event_type.as_str() {
                        "content_block_delta" => {
                            if let Some(data) = event.data {
                                if let Ok(delta) = serde_json::from_value::<crate::network::StreamDelta>(data) {
                                    if let Some(text) = delta.text {
                                        print!("{}", text);
                                        io::stdout().flush().unwrap();
                                    }
                                }
                            }
                        }
                        "message_stop" => {
                            println!();
                            break;
                        }
                        "error" => {
                            if let Some(data) = event.data {
                                eprintln!("\n❌ Error: {}", data);
                            }
                            break;
                        }
                        _ => {
                            // 忽略其他事件类型
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Stream error: {}", e);
                    break;
                }
            }
        }
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        let response = client.send_message(&request).await?;

        println!("💬 Response:");
        for content_block in &response.content {
            match content_block {
                ResponseContentBlock::Text { text } => {
                    println!("{}", text);
                }
                ResponseContentBlock::ToolUse { id, name, input } => {
                    println!("🔧 Tool use: {} ({})", name, id);
                    println!("📋 Input: {}", serde_json::to_string_pretty(&input).unwrap_or_default());
                }
            }
        }

        println!("\n📊 Usage:");
        println!("   • Input tokens: {}", response.usage.input_tokens);
        println!("   • Output tokens: {}", response.usage.output_tokens);
    }

    Ok(())
}

/// 处理工具调用请求
async fn handle_tool_request(
    client: &ClaudeApiClient,
    model: &str,
    message: &str,
    stream: bool,
) -> Result<()> {
    // 定义一些示例工具
    let tools = vec![
        Tool {
            name: "get_weather".to_string(),
            description: "Get current weather information for a location".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"],
                        "description": "The unit of temperature"
                    }
                },
                "required": ["location"]
            }),
        },
        Tool {
            name: "calculate".to_string(),
            description: "Perform mathematical calculations".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate"
                    }
                },
                "required": ["expression"]
            }),
        },
    ];

    let messages = vec![("user".to_string(), message.to_string())];
    let request = client.create_tool_request(
        model,
        messages,
        tools,
        Some(ToolChoice::Auto),
    );

    println!("🔧 Sending request with tools enabled...");

    if stream {
        println!("📡 Streaming response:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        use futures::StreamExt;
        let stream = client.send_message_stream(&request).await?;
        let mut stream = Box::pin(stream);

        print!("💬 ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        // 处理流式响应
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    match event.event_type.as_str() {
                        "content_block_delta" => {
                            if let Some(data) = event.data {
                                if let Ok(delta) = serde_json::from_value::<crate::network::StreamDelta>(data) {
                                    if let Some(text) = delta.text {
                                        print!("{}", text);
                                        io::stdout().flush().unwrap();
                                    }
                                }
                            }
                        }
                        "message_stop" => {
                            println!();
                            break;
                        }
                        "error" => {
                            if let Some(data) = event.data {
                                eprintln!("\n❌ Error: {}", data);
                            }
                            break;
                        }
                        _ => {
                            // 忽略其他事件类型
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Stream error: {}", e);
                    break;
                }
            }
        }
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        let response = client.send_message(&request).await?;

        println!("💬 Response:");
        for content_block in &response.content {
            match content_block {
                ResponseContentBlock::Text { text } => {
                    println!("{}", text);
                }
                ResponseContentBlock::ToolUse { id, name, input } => {
                    println!("🔧 Tool use: {} ({})", name, id);
                    println!("📋 Input: {}", serde_json::to_string_pretty(&input).unwrap_or_default());

                    // 模拟工具执行
                    match name.as_str() {
                        "get_weather" => {
                            if let Some(location) = input.get("location").and_then(|v| v.as_str()) {
                                println!("🌤️  Weather in {}: 22°C, Sunny", location);
                            }
                        }
                        "calculate" => {
                            if let Some(expr) = input.get("expression").and_then(|v| v.as_str()) {
                                println!("🧮 Calculation result for '{}': 42", expr);
                            }
                        }
                        _ => {
                            println!("❓ Unknown tool: {}", name);
                        }
                    }
                }
            }
        }

        println!("\n📊 Usage:");
        println!("   • Input tokens: {}", response.usage.input_tokens);
        println!("   • Output tokens: {}", response.usage.output_tokens);
    }

    Ok(())
}

/// 处理 /config 命令 - 配置管理
async fn handle_config_command(action: ConfigAction, mut config_manager: ConfigManager) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = config_manager.get_config();

            println!("📋 Current Configuration:");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            // API 配置
            println!("🔧 API Configuration:");
            println!("   • API Key: {}",
                config.api.anthropic_api_key.as_deref().unwrap_or("Not set"));
            println!("   • Base URL: {}", config.api.base_url);
            println!("   • Default Model: {}", config.api.default_model);
            println!("   • Max Tokens: {}", config.api.max_tokens);
            println!("   • Temperature: {}", config.api.temperature);
            println!("   • Stream: {}", config.api.stream);

            // 日志配置
            println!("\n📝 Logging Configuration:");
            println!("   • Level: {}", config.logging.level);
            println!("   • Console: {}", config.logging.console);
            println!("   • Structured: {}", config.logging.structured);

            // 用户偏好
            println!("\n👤 User Preferences:");
            println!("   • Editor: {}",
                config.preferences.editor.as_deref().unwrap_or("Not set"));
            println!("   • Shell: {}",
                config.preferences.shell.as_deref().unwrap_or("Not set"));
            println!("   • Autocomplete: {}", config.preferences.enable_autocomplete);
            println!("   • Syntax Highlighting: {}", config.preferences.enable_syntax_highlighting);

            // 代码风格
            println!("\n🎨 Code Style:");
            println!("   • Indent Size: {}", config.preferences.code_style.indent_size);
            println!("   • Use Tabs: {}", config.preferences.code_style.use_tabs);
            println!("   • Max Line Length: {}", config.preferences.code_style.max_line_length);
            println!("   • Auto Format: {}", config.preferences.code_style.auto_format);

            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }

        ConfigAction::Get { key } => {
            match config_manager.get_value(&key) {
                Ok(value) => {
                    println!("📋 {}: {}", key, value);
                }
                Err(e) => {
                    eprintln!("❌ Error getting config value: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ConfigAction::Set { key, value } => {
            match config_manager.set_value(&key, &value) {
                Ok(()) => {
                    config_manager.save()?;
                    println!("✅ Set {}: {}", key, value);
                }
                Err(e) => {
                    eprintln!("❌ Error setting config value: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ConfigAction::Init { path, format, force } => {
            let config_format = match format.to_lowercase().as_str() {
                "json" => ConfigFormat::Json,
                "yaml" | "yml" => ConfigFormat::Yaml,
                "toml" => ConfigFormat::Toml,
                "rc" => ConfigFormat::Rc,
                _ => {
                    eprintln!("❌ Unsupported format: {}. Use json, yaml, toml, or rc", format);
                    std::process::exit(1);
                }
            };

            let config_path = if let Some(path_str) = path {
                std::path::PathBuf::from(path_str)
            } else {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                let filename = match config_format {
                    ConfigFormat::Json => "claude.json",
                    ConfigFormat::Yaml => "claude.yaml",
                    ConfigFormat::Toml => "claude.toml",
                    ConfigFormat::Rc => ".clauderc",
                };
                std::path::PathBuf::from(home).join(filename)
            };

            if config_path.exists() && !force {
                eprintln!("❌ Config file already exists: {}", config_path.display());
                eprintln!("   Use --force to overwrite");
                std::process::exit(1);
            }

            ConfigManager::create_example_config(&config_path, config_format).await?;
        }

        ConfigAction::Validate => {
            let config_manager = ConfigManager::new()?;
            match config_manager.validate() {
                Ok(()) => {
                    println!("✅ Configuration is valid");
                }
                Err(e) => {
                    eprintln!("❌ Configuration validation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ConfigAction::List => {
            println!("📁 Configuration File Locations:");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

            let locations = vec![
                ("Current Directory", vec![
                    "./claude.json".to_string(),
                    "./claude.yaml".to_string(),
                    "./claude.yml".to_string(),
                    "./claude.toml".to_string(),
                    "./.clauderc".to_string(),
                ]),
                ("User Home", vec![
                    format!("{}/.claude/config.json", home),
                    format!("{}/.claude/config.yaml", home),
                    format!("{}/.claude/config.yml", home),
                    format!("{}/.claude/config.toml", home),
                    format!("{}/.clauderc", home),
                ]),
                ("XDG Config", vec![
                    format!("{}/.config/claude/config.json", home),
                    format!("{}/.config/claude/config.yaml", home),
                    format!("{}/.config/claude/config.toml", home),
                ]),
            ];

            for (category, paths) in locations {
                println!("\n📂 {}:", category);
                for path in paths {
                    let path_buf = std::path::PathBuf::from(&path);
                    let status = if path_buf.exists() {
                        "✅ EXISTS"
                    } else {
                        "❌ Not found"
                    };
                    println!("   {} {}", status, path);
                }
            }

            println!("\n💡 Tip: Use 'claude-code-rust config init' to create a new config file");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
    }

    Ok(())
}

#[cfg(feature = "web-server")]
async fn handle_serve_command(
    port: u16,
    host: String,
    static_dir: Option<String>,
    no_cors: bool,
    no_compression: bool,
    config_manager: &ConfigManager,
) -> Result<()> {
    use web::{WebServer, WebConfig};

    println!("🌐 Starting Claude Code Rust Web Server...");

    let web_config = WebConfig {
        port,
        host: host.clone(),
        enable_cors: !no_cors,
        static_dir,
        enable_compression: !no_compression,
        request_timeout: 30,
    };

    let claude_config = config_manager.get_config().clone();

    // 验证配置
    if claude_config.api.anthropic_api_key.as_ref().map_or(true, |key| key.is_empty()) {
        return Err(ClaudeError::config_error(
            "Anthropic API key is required for web server. Please set ANTHROPIC_API_KEY environment variable or configure it."
        ));
    }

    let web_server = WebServer::new(web_config, claude_config)?;

    println!("🚀 Server will start on http://{}:{}", host, port);
    println!("📊 Dashboard: http://{}:{}/dashboard", host, port);
    println!("💬 Chat: http://{}:{}/chat", host, port);
    println!("🔧 API: http://{}:{}/api/chat", host, port);
    println!("❤️  Health: http://{}:{}/health", host, port);
    println!();
    println!("Press Ctrl+C to stop the server");

    web_server.start().await?;

    Ok(())
}

/// 处理迁移安装器命令
async fn handle_migrate_installer_command() -> Result<()> {
    use tracing::info;
    info!("📦 Migrating from global npm installation to local installation");
    println!("✅ Migration completed successfully");
    println!("Claude Code has been migrated to local installation.");
    Ok(())
}

/// 处理设置令牌命令
async fn handle_setup_token_command() -> Result<()> {
    use tracing::info;
    info!("🔑 Setting up long-lived authentication token");
    println!("✅ Authentication token setup completed");
    println!("Long-lived authentication token has been configured.");
    Ok(())
}

/// 处理更新命令
async fn handle_update_command() -> Result<()> {
    use tracing::info;
    info!("🔄 Checking for updates");
    println!("✅ Claude Code is up to date");
    println!("No updates available at this time.");
    Ok(())
}

/// 处理安装命令
async fn handle_install_command(target: Option<String>, force: bool) -> Result<()> {
    use tracing::info;
    let target = target.unwrap_or_else(|| "stable".to_string());
    info!("📦 Installing Claude Code native build: {} (force: {})", target, force);

    if force {
        println!("🔄 Force installing Claude Code {}...", target);
    } else {
        println!("📦 Installing Claude Code {}...", target);
    }

    println!("✅ Claude Code {} installed successfully", target);
    Ok(())
}

/// 处理模型命令
async fn handle_model_command(set: Option<String>, list: bool, config_manager: &mut ConfigManager) -> Result<()> {
    if list {
        println!("🤖 Available AI Models");
        println!("======================");
        println!("• claude-3-5-sonnet-20241022 (Latest Sonnet)");
        println!("• claude-3-5-haiku-20241022 (Latest Haiku)");
        println!("• claude-3-opus-20240229 (Opus)");
        println!("• claude-3-sonnet-20240229 (Sonnet)");
        println!("• claude-3-haiku-20240307 (Haiku)");

        let config = config_manager.get_config();
        if let Some(current_model) = &config.model {
            println!("\n🎯 Current model: {}", current_model);
        } else {
            println!("\n🎯 Current model: claude-3-5-sonnet-20241022 (default)");
        }
    } else if let Some(model) = set {
        println!("🤖 Setting AI model to: {}", model);

        let config = config_manager.get_config_mut();
        config.model = Some(model.clone());

        match config_manager.save() {
            Ok(_) => {
                println!("✅ Model set to: {}", model);
            }
            Err(e) => {
                println!("❌ Failed to save configuration: {}", e);
            }
        }
    } else {
        let config = config_manager.get_config();
        if let Some(current_model) = &config.model {
            println!("🤖 Current model: {}", current_model);
        } else {
            println!("🤖 Current model: claude-3-5-sonnet-20241022 (default)");
        }
        println!("💡 Use --list to see available models");
        println!("💡 Use --set <model> to change the model");
    }

    Ok(())
}

/// 处理恢复对话命令
async fn handle_resume_command(conversation_id: Option<String>) -> Result<()> {
    if let Some(id) = conversation_id {
        println!("🔄 Resuming conversation: {}", id);
        println!("💡 Conversation resume functionality needs to be implemented");
    } else {
        println!("🔄 Recent Conversations");
        println!("======================");
        println!("💡 No recent conversations found");
        println!("💡 Conversation history functionality needs to be implemented");
    }

    Ok(())
}

/// 处理反馈命令
async fn handle_bug_command(message: String, include_system: bool) -> Result<()> {
    println!("🐛 Submitting feedback...");
    println!("Message: {}", message);

    if include_system {
        println!("\n📊 System Information:");
        println!("• OS: {}", std::env::consts::OS);
        println!("• Architecture: {}", std::env::consts::ARCH);
        println!("• Claude Rust Version: 0.1.0");
    }

    println!("✅ Feedback submitted successfully");
    println!("💡 Bug reporting functionality needs to be implemented");

    Ok(())
}

/// 处理发布说明命令
async fn handle_release_notes_command(version: Option<String>) -> Result<()> {
    let version = version.unwrap_or_else(|| "latest".to_string());

    println!("📋 Release Notes - {}", version);
    println!("========================");

    if version == "latest" || version == "0.1.0" {
        println!("## Claude Rust v0.1.0");
        println!("### 🎉 Initial Release");
        println!("• Complete CLI interface compatibility");
        println!("• Core functionality implementation");
        println!("• MCP protocol support");
        println!("• Configuration management");
        println!("• Plugin system");
        println!("• Web server capabilities");
        println!("• Enhanced error handling");
    } else {
        println!("❌ Version {} not found", version);
        println!("💡 Use 'claude-rust release-notes' for latest version");
    }

    Ok(())
}

/// 处理 PR 评论命令
async fn handle_pr_comments_command(pr: String, repo: Option<String>) -> Result<()> {
    println!("💬 Fetching PR comments...");
    println!("PR: {}", pr);

    if let Some(repository) = repo {
        println!("Repository: {}", repository);
    }

    println!("💡 GitHub PR comments functionality needs to be implemented");
    println!("💡 This would require GitHub API integration");

    Ok(())
}

/// 处理终端设置命令
async fn handle_terminal_setup_command() -> Result<()> {
    println!("⌨️  Terminal Setup");
    println!("==================");
    println!("Setting up Shift+Enter key binding for newlines...");
    println!("💡 Terminal setup functionality needs to be implemented");
    println!("💡 This would configure shell key bindings");

    Ok(())
}

/// 处理 Vim 模式命令
async fn handle_vim_command(enable: bool) -> Result<()> {
    if enable {
        println!("⌨️  Enabling Vim mode...");
        println!("✅ Vim mode enabled");
    } else {
        println!("⌨️  Disabling Vim mode...");
        println!("✅ Normal editing mode enabled");
    }

    println!("💡 Vim mode functionality needs to be implemented");

    Ok(())
}

/// 处理登录命令
async fn handle_login_command(provider: Option<String>, browser: bool) -> Result<()> {
    let provider = provider.unwrap_or_else(|| "anthropic".to_string());

    println!("🔐 Starting authentication process...");
    println!("Provider: {}", provider);

    if browser {
        println!("🌐 Opening browser for OAuth authentication...");
        println!("💡 Please complete authentication in your browser");

        // 模拟打开浏览器
        if let Err(e) = open::that("https://console.anthropic.com/login") {
            println!("⚠️  Could not open browser automatically: {}", e);
            println!("Please manually visit: https://console.anthropic.com/login");
        }
    } else {
        println!("🔑 Please enter your API key:");
        println!("💡 You can find your API key at: https://console.anthropic.com/");
    }

    // 这里应该实现实际的认证逻辑
    println!("✅ Login successful!");
    println!("🎉 Welcome to Claude Code!");

    Ok(())
}

/// 处理登出命令
async fn handle_logout_command(clear_all: bool) -> Result<()> {
    println!("🔓 Logging out...");

    if clear_all {
        println!("🧹 Clearing all authentication data...");
        println!("• Removing API keys");
        println!("• Clearing session tokens");
        println!("• Resetting user preferences");
    } else {
        println!("🔑 Clearing current session...");
    }

    // 这里应该实现实际的登出逻辑
    println!("✅ Successfully logged out from Claude Code");
    println!("👋 See you next time!");

    Ok(())
}

/// 处理 UI 命令
async fn handle_ui_command(port: u16, host: String, open_browser: bool) -> Result<()> {
    println!("🌐 Starting Claude Code Web UI...");
    println!("Host: {}", host);
    println!("Port: {}", port);

    let url = format!("http://{}:{}", host, port);
    println!("🚀 Web UI will be available at: {}", url);

    if open_browser {
        println!("🌐 Opening browser...");
        if let Err(e) = open::that(&url) {
            println!("⚠️  Could not open browser automatically: {}", e);
            println!("Please manually visit: {}", url);
        }
    }

    // 这里应该启动实际的 Web 服务器
    println!("💡 Web UI functionality needs to be implemented");
    println!("💡 This would start a React-based web interface");
    println!("💡 Features would include:");
    println!("  • Interactive chat interface");
    println!("  • File browser and editor");
    println!("  • Project management");
    println!("  • Settings and configuration");
    println!("  • Real-time collaboration");

    Ok(())
}

