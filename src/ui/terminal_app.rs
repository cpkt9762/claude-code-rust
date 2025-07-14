//! Claude Code - Rust Edition Terminal UI
//!
//! 基于ratatui实现的现代化终端用户界面，模仿原版Claude Code的交互体验

use crate::error::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap, Gauge},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use tui_input::{backend::crossterm::EventHandler, Input};
use tracing::{info, warn, error, debug};

/// 应用状态 - 模仿原版Claude Code的界面模式
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// 聊天模式 - 默认模式，类似原版Claude Code
    Chat,
    /// 帮助模式
    Help,
    /// 退出确认
    ExitConfirm,
}

/// 消息类型
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    /// 用户消息
    User,
    /// AI助手消息
    Assistant,
    /// 系统消息
    System,
    /// 错误消息
    Error,
}

/// 聊天消息 - 重新设计以匹配原版Claude Code的消息格式
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 是否正在输入中（用于流式响应）
    pub is_streaming: bool,
}

/// 终端应用 - 重新设计以匹配原版Claude Code的体验
pub struct TerminalApp {
    /// 当前模式
    mode: AppMode,
    /// 是否应该退出
    should_quit: bool,
    /// 输入框
    input: Input,
    /// 聊天消息历史
    messages: Vec<ChatMessage>,
    /// 状态消息
    status_message: String,
    /// 加载状态
    is_loading: bool,
    /// 加载进度
    loading_progress: f64,
    /// 消息滚动位置
    message_scroll: usize,
    /// 是否显示欢迎信息
    show_welcome: bool,
    /// 输入历史
    input_history: Vec<String>,
    /// 历史索引
    history_index: Option<usize>,
}

impl Default for TerminalApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalApp {
    /// 创建新的终端应用 - 默认进入聊天模式，类似原版Claude Code
    pub fn new() -> Self {
        Self {
            mode: AppMode::Chat,  // 默认进入聊天模式
            should_quit: false,
            input: Input::default(),
            messages: Vec::new(),
            status_message: "Claude Code - Rust Edition | Ready to chat".to_string(),
            is_loading: false,
            loading_progress: 0.0,
            message_scroll: 0,
            show_welcome: true,
            input_history: Vec::new(),
            history_index: None,
        }
    }

    /// 运行应用
    pub async fn run(&mut self) -> Result<()> {
        // 设置终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // 添加欢迎消息 - 模仿原版Claude Code的启动体验
        if self.show_welcome {
            self.add_message(
                "Welcome to Claude Code - Rust Edition! 🦀",
                MessageType::System,
            );
            self.add_message(
                "I'm Claude, your AI assistant. I can help you with coding, writing, analysis, and more.",
                MessageType::Assistant,
            );
            self.add_message(
                "Type your message below and press Enter to start our conversation.",
                MessageType::System,
            );
            self.show_welcome = false;
        }

        let result = self.run_app(&mut terminal).await;

        // 恢复终端
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// 运行应用主循环
    async fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(250);

        loop {
            terminal.draw(|f| self.ui(f))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key).await?;
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// 处理按键事件 - 重新设计以匹配原版Claude Code的快捷键
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // 全局快捷键
        match key.code {
            KeyCode::Esc if key.modifiers.is_empty() => {
                match self.mode {
                    AppMode::Chat => {
                        // 在聊天模式下，ESC两次退出
                        self.mode = AppMode::ExitConfirm;
                    }
                    _ => {
                        self.mode = AppMode::Chat;
                    }
                }
                return Ok(());
            }
            _ => {}
        }

        match self.mode {
            AppMode::Chat => self.handle_chat_keys(key).await?,
            AppMode::Help => self.handle_help_keys(key).await?,
            AppMode::ExitConfirm => self.handle_exit_confirm_keys(key).await?,
        }
        Ok(())
    }

    /// 处理聊天模式按键 - 重新设计以匹配原版Claude Code的交互
    async fn handle_chat_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                let message = self.input.value().to_string();
                if !message.trim().is_empty() {
                    // 添加到历史记录
                    self.input_history.push(message.clone());
                    self.history_index = None;

                    self.send_message(message).await?;
                    self.input.reset();
                }
            }
            KeyCode::Up if self.input.value().is_empty() => {
                // 浏览输入历史
                if !self.input_history.is_empty() {
                    let new_index = match self.history_index {
                        None => Some(self.input_history.len() - 1),
                        Some(i) if i > 0 => Some(i - 1),
                        Some(_) => Some(0),
                    };
                    if let Some(index) = new_index {
                        self.history_index = Some(index);
                        let historical_input = self.input_history[index].clone();
                        self.input = Input::new(historical_input);
                    }
                }
            }
            KeyCode::Down if self.history_index.is_some() => {
                // 浏览输入历史
                if let Some(current_index) = self.history_index {
                    if current_index < self.input_history.len() - 1 {
                        let new_index = current_index + 1;
                        self.history_index = Some(new_index);
                        let historical_input = self.input_history[new_index].clone();
                        self.input = Input::new(historical_input);
                    } else {
                        self.history_index = None;
                        self.input.reset();
                    }
                }
            }
            KeyCode::Char('/') if self.input.value().is_empty() => {
                // 输入/字符，让用户继续输入完整命令
                self.input.handle_event(&Event::Key(key));
            }
            KeyCode::Char('?') if self.input.value().is_empty() => {
                // 显示帮助
                self.mode = AppMode::Help;
            }
            _ => {
                // 重置历史索引当用户开始输入
                if self.history_index.is_some() {
                    self.history_index = None;
                }
                self.input.handle_event(&Event::Key(key));
            }
        }
        Ok(())
    }



    /// 处理命令模式按键
    async fn handle_command_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                let command = self.input.value().to_string();
                if !command.trim().is_empty() {
                    self.execute_command(command).await?;
                    self.input.reset();
                    // 执行命令后返回聊天模式
                    self.mode = AppMode::Chat;
                }
            }
            _ => {
                self.input.handle_event(&Event::Key(key));
            }
        }
        Ok(())
    }

    /// 处理帮助模式按键
    async fn handle_help_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                self.mode = AppMode::Chat;
            }
            _ => {}
        }
        Ok(())
    }

    /// 处理退出确认按键
    async fn handle_exit_confirm_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.should_quit = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = AppMode::Chat;
                self.status_message = "Exit cancelled".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    /// 发送消息 - 重新设计以提供更好的用户体验
    async fn send_message(&mut self, message: String) -> Result<()> {
        // 添加用户消息
        self.add_message(&message, MessageType::User);

        // 模拟AI响应
        self.is_loading = true;
        self.status_message = "Claude is thinking...".to_string();

        // 这里可以集成实际的AI API调用
        let response = self.generate_ai_response(&message).await?;

        self.add_message(&response, MessageType::Assistant);
        self.is_loading = false;
        self.status_message = "Ready for your next message".to_string();

        Ok(())
    }

    /// 生成AI响应（模拟）
    async fn generate_ai_response(&mut self, message: &str) -> Result<String> {
        // 模拟处理时间
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        let response = match message.to_lowercase().as_str() {
            msg if msg.contains("hello") || msg.contains("hi") => {
                "Hello! I'm Claude, your AI assistant. How can I help you today?"
            }
            msg if msg.contains("help") => {
                "I can help you with coding, writing, analysis, and many other tasks. What would you like to work on?"
            }
            msg if msg.contains("rust") => {
                "Rust is a great systems programming language! It offers memory safety without garbage collection. What would you like to know about Rust?"
            }
            _ => {
                "I understand your message. This is a demo response from the Claude Code Rust edition terminal UI!"
            }
        };
        
        Ok(response.to_string())
    }

    /// 添加消息 - 统一的消息添加方法
    fn add_message(&mut self, content: &str, message_type: MessageType) {
        self.messages.push(ChatMessage {
            content: content.to_string(),
            message_type,
            timestamp: chrono::Utc::now(),
            is_streaming: false,
        });

        // 自动滚动到最新消息
        if self.messages.len() > 0 {
            self.message_scroll = self.messages.len().saturating_sub(1);
        }
    }

    /// 显示命令列表
    fn show_command_list(&mut self) {
        let command_list = "\
Available commands:

  /add-dir            Add a new working directory
  /bug                Submit feedback about Claude Code
  /clear              Clear conversation history and free up context
  /compact            Clear conversation history but keep a summary in context. Optional: /compact
                      [instructions for summarization]
  /config (theme)     Open config panel
  /cost               Show the total cost and duration of the current session
  /doctor             Diagnose and verify your Claude Code installation and settings
  /exit (quit)        Exit the REPL
  /export             Export the current conversation to a file or clipboard
  /help               Show help and available commands
  /hooks              Manage git hooks for Claude Code
  /ide                Open IDE integration panel
  /init               Initialize Claude Code in a new directory
  /install-github-app Install GitHub app for enhanced integration
  /login              Login to Claude Code services
  /logout             Logout from Claude Code services
  /mcp                Manage Model Context Protocol servers
  /memory             Manage conversation memory and context
  /migrate-installer  Migrate from old installer
  /model              Switch or configure AI models
  /permissions        Manage file and directory permissions
  /pr-comments        Review and manage pull request comments
  /release-notes      Show release notes and updates
  /resume             Resume a previous conversation
  /review             Review code changes and provide feedback
  /status             Show current session status
  /upgrade            Upgrade Claude Code to the latest version
  /vim                Enable vim-style editing mode

Type a command name and press Enter to execute it.
Press ESC to return to chat mode.";

        self.add_message(command_list, MessageType::System);
    }

    /// 执行命令 - 重新设计命令系统
    async fn execute_command(&mut self, command: String) -> Result<()> {
        let cmd = command.trim();

        // 只有以/开头的输入才被当作命令
        if !cmd.starts_with('/') {
            // 不是命令，当作普通消息处理
            self.add_message(cmd, MessageType::User);
            self.add_message("I'm Claude, your AI assistant. How can I help you today?", MessageType::Assistant);
            return Ok(());
        }

        // 去掉/前缀来获取实际命令名
        let cmd_name = &cmd[1..];

        // 添加命令到消息历史
        self.add_message(cmd, MessageType::User);

        let response = match cmd_name.to_lowercase().as_str() {
            "add-dir" => {
                "Add Directory Command\n\n\
                This command would add a new working directory to Claude Code.\n\
                In the full implementation, this would:\n\
                • Browse for a directory\n\
                • Add it to the workspace\n\
                • Index files for context\n\n\
                [Demo mode - command not fully implemented]"
            }
            "bug" => {
                "Bug Report\n\n\
                This command would open a bug report interface.\n\
                In the full implementation, this would:\n\
                • Collect system information\n\
                • Open a feedback form\n\
                • Submit to the development team\n\n\
                [Demo mode - command not fully implemented]"
            }
            "clear" => {
                self.messages.clear();
                self.message_scroll = 0;
                "Conversation cleared! Ready for a fresh start."
            }
            "compact" => {
                "Compact Command\n\n\
                This command would clear conversation history but keep a summary.\n\
                In the full implementation, this would:\n\
                • Analyze conversation context\n\
                • Create a summary\n\
                • Clear detailed history\n\
                • Preserve important context\n\n\
                [Demo mode - command not fully implemented]"
            }
            "config" => {
                "Configuration Panel\n\n\
                This command would open the configuration interface.\n\
                Available settings:\n\
                • Theme selection\n\
                • API keys\n\
                • Model preferences\n\
                • File permissions\n\n\
                [Demo mode - command not fully implemented]"
            }
            "cost" => {
                "Session Cost Information\n\n\
                Current Session:\n\
                • Duration: Demo mode\n\
                • API calls: Demo mode\n\
                • Estimated cost: Demo mode\n\
                • Tokens used: Demo mode\n\n\
                [Demo mode - cost tracking not implemented]"
            }
            "doctor" => {
                "System Diagnostics\n\n\
                ✅ Claude Code - Rust Edition\n\
                ✅ Terminal UI functional\n\
                ✅ Input/Output working\n\
                ✅ Command system active\n\
                ✅ Memory management OK\n\n\
                All systems operational!"
            }
            "help" | "h" => {
                self.show_command_list();
                return Ok(());
            }
            "status" => {
                &format!("System Status:\n\n\
                • Application: Claude Code - Rust Edition\n\
                • Mode: {}\n\
                • Messages: {}\n\
                • Memory: OK\n\
                • Input History: {} entries\n\
                • Uptime: Demo mode",
                match self.mode {
                    AppMode::Chat => "Chat",
                    AppMode::Help => "Help",
                    AppMode::ExitConfirm => "Exit Confirm",
                },
                self.messages.len(),
                self.input_history.len())
            }
            "version" => {
                "Claude Code - Rust Edition v0.1.0\n\n\
                Built with:\n\
                • Rust 🦀\n\
                • ratatui for terminal UI\n\
                • crossterm for cross-platform terminal handling\n\
                • tokio for async runtime\n\n\
                A high-performance reimplementation of Claude Code in Rust."
            }
            "exit" | "quit" => {
                self.mode = AppMode::ExitConfirm;
                return Ok(());
            }
            _ => {
                &format!("Unknown command: '{}'\n\n\
                Type '/help' to see all available commands.\n\
                Press ESC to return to chat mode.", cmd)
            }
        };

        self.add_message(response, MessageType::System);
        self.status_message = format!("Command '{}' executed", cmd);

        Ok(())
    }

    /// 定时器回调
    fn on_tick(&mut self) {
        if self.is_loading {
            self.loading_progress += 0.1;
            if self.loading_progress > 1.0 {
                self.loading_progress = 0.0;
            }
        }
    }

    /// 渲染UI - 重新设计以匹配原版Claude Code的界面风格
    fn ui(&mut self, f: &mut Frame) {
        match self.mode {
            AppMode::Chat => self.render_chat(f),
            AppMode::Help => self.render_help(f),
            AppMode::ExitConfirm => self.render_exit_confirm(f),
        }
    }

    /// 渲染聊天界面 - 重新设计以匹配原版Claude Code的简洁风格
    fn render_chat(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // 消息区域
                Constraint::Length(3),  // 输入框
                Constraint::Length(1),  // 状态栏
            ])
            .split(f.size());

        // 消息区域
        self.render_messages(f, chunks[0]);

        // 输入框
        self.render_input_box(f, chunks[1]);

        // 状态栏
        self.render_status_bar(f, chunks[2]);

        // 加载指示器
        if self.is_loading {
            self.render_loading_popup(f);
        }
    }

    /// 渲染消息区域 - 新的消息显示方式
    fn render_messages(&mut self, f: &mut Frame, area: Rect) {
        if self.messages.is_empty() {
            // 显示欢迎信息
            let welcome_text = vec![
                Line::from(""),
                Line::from("🦀 Welcome to Claude Code - Rust Edition!"),
                Line::from(""),
                Line::from("I'm Claude, your AI assistant. I can help you with:"),
                Line::from("• Writing and editing code"),
                Line::from("• Debugging and troubleshooting"),
                Line::from("• Explaining complex concepts"),
                Line::from("• Planning and architecture"),
                Line::from("• And much more!"),
                Line::from(""),
                Line::from("💡 Tips:"),
                Line::from("• Type '/' to access commands"),
                Line::from("• Press '?' for quick help"),
                Line::from("• Use ↑/↓ to browse input history"),
                Line::from("• Press ESC twice to exit"),
                Line::from(""),
                Line::from("What would you like to work on today?"),
            ];

            let welcome_widget = Paragraph::new(welcome_text)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Claude Code - Rust Edition"))
                .wrap(Wrap { trim: true });
            f.render_widget(welcome_widget, area);
            return;
        }

        // 渲染消息列表
        let messages: Vec<ListItem> = self.messages
            .iter()
            .map(|msg| {
                let timestamp = msg.timestamp.format("%H:%M");
                let (prefix, style) = match msg.message_type {
                    MessageType::User => ("You", Style::default().fg(Color::Blue)),
                    MessageType::Assistant => ("Claude", Style::default().fg(Color::Green)),
                    MessageType::System => ("System", Style::default().fg(Color::Yellow)),
                    MessageType::Error => ("Error", Style::default().fg(Color::Red)),
                };

                // 格式化消息内容，支持多行
                let content = if msg.content.contains('\n') {
                    format!("[{}] {}:\n{}", timestamp, prefix, msg.content)
                } else {
                    format!("[{}] {}: {}", timestamp, prefix, msg.content)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let messages_widget = List::new(messages)
            .block(Block::default().borders(Borders::ALL).title("Conversation"));
        f.render_widget(messages_widget, area);
    }

    /// 渲染输入框 - 新的输入框设计
    fn render_input_box(&mut self, f: &mut Frame, area: Rect) {
        let input_text = self.input.value();

        // 根据当前模式显示不同的提示
        let title = match self.mode {
            AppMode::Chat => "Message (Enter to send, / for commands, ? for help)",
            _ => "Input",
        };

        let input_widget = Paragraph::new(input_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(input_widget, area);

        // 设置光标位置
        f.set_cursor(
            area.x + self.input.visual_cursor() as u16 + 1,
            area.y + 1,
        );
    }

    /// 渲染帮助界面 - 重新设计帮助内容
    fn render_help(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // 帮助内容
                Constraint::Length(1),  // 状态栏
            ])
            .split(f.size());

        // 帮助内容
        let help_text = vec![
            Line::from(""),
            Line::from("🦀 Claude Code - Rust Edition Help"),
            Line::from(""),
            Line::from("💬 Chat Mode (Default):"),
            Line::from("  • Type your message and press Enter to send"),
            Line::from("  • Use ↑/↓ arrows to browse input history"),
            Line::from("  • Type '/' to enter command mode"),
            Line::from("  • Type '?' to show this help"),
            Line::from("  • Press ESC twice to exit"),
            Line::from(""),
            Line::from("⌨️ Available Commands:"),
            Line::from("  • /help, /h - Show this help"),
            Line::from("  • /status - Show system status"),
            Line::from("  • /clear - Clear conversation"),
            Line::from("  • /version - Show version information"),
            Line::from("  • /exit, /quit - Exit application"),
            Line::from(""),
            Line::from("🔧 Keyboard Shortcuts:"),
            Line::from("  • Enter - Send message/Execute command"),
            Line::from("  • ESC - Go back/Cancel (press twice to exit)"),
            Line::from("  • ↑/↓ - Browse input history (when input is empty)"),
            Line::from("  • Ctrl+C - Force quit"),
            Line::from(""),
            Line::from("💡 Tips:"),
            Line::from("  • Claude can help with coding, debugging, explanations, and more"),
            Line::from("  • Be specific in your questions for better responses"),
            Line::from("  • Use the command system for application controls"),
            Line::from(""),
            Line::from("Press any key to return to chat..."),
        ];

        let help_widget = Paragraph::new(help_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help & Commands")
                .border_style(Style::default().fg(Color::Yellow)))
            .wrap(Wrap { trim: true });
        f.render_widget(help_widget, chunks[0]);

        // 状态栏
        self.render_status_bar(f, chunks[1]);
    }

    /// 渲染命令界面 - 显示命令列表
    fn render_command(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // 命令列表区域
                Constraint::Length(3),  // 输入框
                Constraint::Length(1),  // 状态栏
            ])
            .split(f.size());

        // 命令列表区域
        self.render_command_list(f, chunks[0]);

        // 输入框
        self.render_input_box(f, chunks[1]);

        // 状态栏
        self.render_status_bar(f, chunks[2]);
    }

    /// 渲染命令列表
    fn render_command_list(&mut self, f: &mut Frame, area: Rect) {
        let command_items = vec![
            ListItem::new("  /add-dir            Add a new working directory"),
            ListItem::new("  /bug                Submit feedback about Claude Code"),
            ListItem::new("  /clear              Clear conversation history and free up context"),
            ListItem::new("  /compact            Clear conversation history but keep a summary in context. Optional: /compact"),
            ListItem::new("                      [instructions for summarization]"),
            ListItem::new("  /config (theme)     Open config panel"),
            ListItem::new("  /cost               Show the total cost and duration of the current session"),
            ListItem::new("  /doctor             Diagnose and verify your Claude Code installation and settings"),
            ListItem::new("  /exit (quit)        Exit the REPL"),
            ListItem::new("  /export             Export the current conversation to a file or clipboard"),
            ListItem::new("  /help               Show help and available commands"),
            ListItem::new("  /hooks              Manage git hooks for Claude Code"),
            ListItem::new("  /ide                Open IDE integration panel"),
            ListItem::new("  /init               Initialize Claude Code in a new directory"),
            ListItem::new("  /install-github-app Install GitHub app for enhanced integration"),
            ListItem::new("  /login              Login to Claude Code services"),
            ListItem::new("  /logout             Logout from Claude Code services"),
            ListItem::new("  /mcp                Manage Model Context Protocol servers"),
            ListItem::new("  /memory             Manage conversation memory and context"),
            ListItem::new("  /migrate-installer  Migrate from old installer"),
            ListItem::new("  /model              Switch or configure AI models"),
            ListItem::new("  /permissions        Manage file and directory permissions"),
            ListItem::new("  /pr-comments        Review and manage pull request comments"),
            ListItem::new("  /release-notes      Show release notes and updates"),
            ListItem::new("  /resume             Resume a previous conversation"),
            ListItem::new("  /review             Review code changes and provide feedback"),
            ListItem::new("  /status             Show current session status"),
            ListItem::new("  /upgrade            Upgrade Claude Code to the latest version"),
            ListItem::new("  /vim                Enable vim-style editing mode"),
        ];

        let command_list = List::new(command_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Available Commands")
                .border_style(Style::default().fg(Color::Green)))
            .style(Style::default().fg(Color::White));

        f.render_widget(command_list, area);
    }

    /// 渲染退出确认对话框 - 重新设计
    fn render_exit_confirm(&mut self, f: &mut Frame) {
        // 先渲染聊天界面作为背景
        self.render_chat(f);

        // 计算弹窗位置
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 3,
            width: area.width / 2,
            height: 7,
        };

        // 清除弹窗区域
        f.render_widget(Clear, popup_area);

        // 渲染确认对话框
        let confirm_text = vec![
            Line::from(""),
            Line::from("Are you sure you want to exit Claude Code?"),
            Line::from(""),
            Line::from("Press 'Y' to confirm or 'N' to cancel"),
        ];

        let confirm_widget = Paragraph::new(confirm_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Exit Confirmation")
                    .border_style(Style::default().fg(Color::Red))
            );
        f.render_widget(confirm_widget, popup_area);
    }

    /// 渲染状态栏 - 简化的状态栏设计
    fn render_status_bar(&mut self, f: &mut Frame, area: Rect) {
        let status_text = if self.is_loading {
            format!("⏳ {} | Messages: {} | ESC twice to exit",
                self.status_message, self.messages.len())
        } else {
            format!("✅ {} | Messages: {} | ESC twice to exit",
                self.status_message, self.messages.len())
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Left);
        f.render_widget(status, area);
    }

    /// 渲染加载弹窗
    fn render_loading_popup(&mut self, f: &mut Frame) {
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 3,
            y: area.height / 2 - 2,
            width: area.width / 3,
            height: 5,
        };

        // 清除弹窗区域
        f.render_widget(Clear, popup_area);

        // 进度条
        let progress = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("AI Thinking..."))
            .gauge_style(Style::default().fg(Color::Green))
            .percent((self.loading_progress * 100.0) as u16)
            .label(format!("{:.0}%", self.loading_progress * 100.0));
        f.render_widget(progress, popup_area);
    }
}
