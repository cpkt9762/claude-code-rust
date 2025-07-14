//! 终端UI演示程序
//! 
//! 展示Claude Code Rust版本的终端用户界面功能

use claude_rust::ui::terminal_app::TerminalApp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🦀 Claude Code - Rust Edition Terminal UI Demo");
    println!("===============================================");
    println!();
    println!("This demo showcases the modern terminal user interface");
    println!("built with ratatui for the Claude Code Rust edition.");
    println!();
    println!("Features:");
    println!("• 💬 Interactive chat interface");
    println!("• ⚙️  Configuration management");
    println!("• ❓ Built-in help system");
    println!("• 🎨 Modern terminal UI with colors and animations");
    println!("• ⌨️  Keyboard shortcuts and navigation");
    println!();
    println!("Press any key to start the terminal UI...");
    
    // 等待用户按键
    use std::io::{self, Read};
    let mut buffer = [0; 1];
    io::stdin().read_exact(&mut buffer)?;

    // 启动终端UI
    let mut app = TerminalApp::new();
    app.run().await?;

    println!("✅ Demo completed successfully!");
    Ok(())
}
