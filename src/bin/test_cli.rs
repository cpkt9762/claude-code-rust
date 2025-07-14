//! 简化的 CLI 测试程序
//! 
//! 用于测试核心 CLI 功能是否正常工作

use claude_rust::cli::{Cli, ClaudeCodeCli};
use claude_rust::error::Result;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🦀 Claude Code Rust - CLI Test");
    println!("==============================");

    // 测试 CLI 处理器创建
    match ClaudeCodeCli::new().await {
        Ok(cli_handler) => {
            println!("✅ CLI handler created successfully");
            
            // 测试状态命令
            println!("\n📊 Testing status command...");
            if let Err(e) = cli_handler.handle_status_command().await {
                println!("❌ Status command failed: {}", e);
            }

            // 测试医生检查命令
            println!("\n🏥 Testing doctor command...");
            if let Err(e) = cli_handler.handle_doctor_command().await {
                println!("❌ Doctor command failed: {}", e);
            }

            // 测试成本命令
            println!("\n💰 Testing cost command...");
            if let Err(e) = cli_handler.handle_cost_command(7).await {
                println!("❌ Cost command failed: {}", e);
            }

            println!("\n🎉 All basic tests completed!");
        },
        Err(e) => {
            println!("❌ Failed to create CLI handler: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
