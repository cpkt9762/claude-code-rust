//! 高级终端UI应用
//! 
//! 基于ratatui实现的现代化终端用户界面

use crate::error::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap, Gauge},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use tui_input::{backend::crossterm::EventHandler, Input};
use tracing::{info, warn, error, debug};

/// 应用状态
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// 主界面
    Main,
    /// 聊天模式
    Chat,
    /// 配置模式
    Config,
    /// 帮助模式
    Help,
    /// 退出确认
    ExitConfirm,
}

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// 消息内容
    pub content: String,
    /// 是否为用户消息
    pub is_user: bool,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 终端应用
pub struct TerminalApp {
    /// 当前模式
    mode: AppMode,
    /// 是否应该退出
    should_quit: bool,
    /// 输入框
    input: Input,
    /// 聊天消息历史
    messages: Vec<ChatMessage>,
    /// 当前选中的菜单项
    selected_menu: usize,
    /// 状态消息
    status_message: String,
    /// 加载状态
    is_loading: bool,
    /// 加载进度
    loading_progress: f64,
}

impl Default for TerminalApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalApp {
    /// 创建新的终端应用
    pub fn new() -> Self {
        Self {
            mode: AppMode::Main,
            should_quit: false,
            input: Input::default(),
            messages: Vec::new(),
            selected_menu: 0,
            status_message: "Welcome to Claude Code - Rust Edition".to_string(),
            is_loading: false,
            loading_progress: 0.0,
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

        // 添加欢迎消息
        self.add_system_message("Claude Code - Rust Edition started successfully!");
        self.add_system_message("Type 'help' for available commands or press 'h' for help.");

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

    /// 处理按键事件
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Main => self.handle_main_keys(key).await?,
            AppMode::Chat => self.handle_chat_keys(key).await?,
            AppMode::Config => self.handle_config_keys(key).await?,
            AppMode::Help => self.handle_help_keys(key).await?,
            AppMode::ExitConfirm => self.handle_exit_confirm_keys(key).await?,
        }
        Ok(())
    }

    /// 处理主界面按键
    async fn handle_main_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.mode = AppMode::ExitConfirm;
            }
            KeyCode::Char('c') => {
                self.mode = AppMode::Chat;
                self.status_message = "Chat mode - Type your message and press Enter".to_string();
            }
            KeyCode::Char('h') => {
                self.mode = AppMode::Help;
            }
            KeyCode::Char('s') => {
                self.mode = AppMode::Config;
            }
            KeyCode::Up => {
                if self.selected_menu > 0 {
                    self.selected_menu -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_menu < 3 {
                    self.selected_menu += 1;
                }
            }
            KeyCode::Enter => {
                match self.selected_menu {
                    0 => self.mode = AppMode::Chat,
                    1 => self.mode = AppMode::Config,
                    2 => self.mode = AppMode::Help,
                    3 => self.mode = AppMode::ExitConfirm,
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// 处理聊天模式按键
    async fn handle_chat_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Main;
                self.status_message = "Returned to main menu".to_string();
            }
            KeyCode::Enter => {
                let message = self.input.value().to_string();
                if !message.trim().is_empty() {
                    self.send_message(message).await?;
                    self.input.reset();
                }
            }
            _ => {
                self.input.handle_event(&Event::Key(key));
            }
        }
        Ok(())
    }

    /// 处理配置模式按键
    async fn handle_config_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Main;
                self.status_message = "Configuration saved".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    /// 处理帮助模式按键
    async fn handle_help_keys(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::Main;
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
                self.mode = AppMode::Main;
                self.status_message = "Exit cancelled".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    /// 发送消息
    async fn send_message(&mut self, message: String) -> Result<()> {
        // 添加用户消息
        self.add_user_message(&message);
        
        // 模拟AI响应
        self.is_loading = true;
        self.status_message = "AI is thinking...".to_string();
        
        // 这里可以集成实际的AI API调用
        let response = self.generate_ai_response(&message).await?;
        
        self.add_ai_message(&response);
        self.is_loading = false;
        self.status_message = "Message sent successfully".to_string();
        
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

    /// 添加用户消息
    fn add_user_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            content: content.to_string(),
            is_user: true,
            timestamp: chrono::Utc::now(),
        });
    }

    /// 添加AI消息
    fn add_ai_message(&mut self, content: &str) {
        self.messages.push(ChatMessage {
            content: content.to_string(),
            is_user: false,
            timestamp: chrono::Utc::now(),
        });
    }

    /// 添加系统消息
    fn add_system_message(&mut self, content: &str) {
        self.status_message = content.to_string();
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

    /// 渲染UI
    fn ui(&mut self, f: &mut Frame) {
        match self.mode {
            AppMode::Main => self.render_main(f),
            AppMode::Chat => self.render_chat(f),
            AppMode::Config => self.render_config(f),
            AppMode::Help => self.render_help(f),
            AppMode::ExitConfirm => self.render_exit_confirm(f),
        }
    }

    /// 渲染主界面
    fn render_main(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 菜单
                Constraint::Length(3),  // 状态栏
            ])
            .split(f.size());

        // 标题
        let title = Paragraph::new("🦀 Claude Code - Rust Edition")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Welcome"));
        f.render_widget(title, chunks[0]);

        // 菜单
        let menu_items = vec![
            "💬 Chat Mode",
            "⚙️  Settings",
            "❓ Help",
            "🚪 Exit",
        ];

        let items: Vec<ListItem> = menu_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected_menu {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(*item).style(style)
            })
            .collect();

        let menu = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Main Menu"))
            .highlight_style(Style::default().bg(Color::DarkGray));
        f.render_widget(menu, chunks[1]);

        // 状态栏
        self.render_status_bar(f, chunks[2]);
    }

    /// 渲染聊天界面
    fn render_chat(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 消息区域
                Constraint::Length(3),  // 输入框
                Constraint::Length(3),  // 状态栏
            ])
            .split(f.size());

        // 标题
        let title = Paragraph::new("💬 Chat with Claude")
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Chat Mode"));
        f.render_widget(title, chunks[0]);

        // 消息区域
        let messages: Vec<ListItem> = self.messages
            .iter()
            .map(|msg| {
                let timestamp = msg.timestamp.format("%H:%M:%S");
                let prefix = if msg.is_user { "👤 You" } else { "🤖 Claude" };
                let style = if msg.is_user {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::Green)
                };

                let content = format!("[{}] {}: {}", timestamp, prefix, msg.content);
                ListItem::new(content).style(style)
            })
            .collect();

        let messages_widget = List::new(messages)
            .block(Block::default().borders(Borders::ALL).title("Conversation"));
        f.render_widget(messages_widget, chunks[1]);

        // 输入框
        let input_text = self.input.value();
        let input_widget = Paragraph::new(input_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Type your message (ESC to go back)"));
        f.render_widget(input_widget, chunks[2]);

        // 设置光标位置
        f.set_cursor(
            chunks[2].x + self.input.visual_cursor() as u16 + 1,
            chunks[2].y + 1,
        );

        // 状态栏
        self.render_status_bar(f, chunks[3]);

        // 加载指示器
        if self.is_loading {
            self.render_loading_popup(f);
        }
    }

    /// 渲染配置界面
    fn render_config(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 配置选项
                Constraint::Length(3),  // 状态栏
            ])
            .split(f.size());

        // 标题
        let title = Paragraph::new("⚙️ Configuration")
            .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Settings"));
        f.render_widget(title, chunks[0]);

        // 配置选项
        let config_text = vec![
            Line::from("🔧 Claude Code Configuration"),
            Line::from(""),
            Line::from("• API Provider: Anthropic"),
            Line::from("• Model: Claude-3-Sonnet"),
            Line::from("• Max Tokens: 4096"),
            Line::from("• Temperature: 0.7"),
            Line::from(""),
            Line::from("Press ESC to return to main menu"),
        ];

        let config_widget = Paragraph::new(config_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Current Settings"))
            .wrap(Wrap { trim: true });
        f.render_widget(config_widget, chunks[1]);

        // 状态栏
        self.render_status_bar(f, chunks[2]);
    }

    /// 渲染帮助界面
    fn render_help(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 帮助内容
                Constraint::Length(3),  // 状态栏
            ])
            .split(f.size());

        // 标题
        let title = Paragraph::new("❓ Help & Commands")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(title, chunks[0]);

        // 帮助内容
        let help_text = vec![
            Line::from("🦀 Claude Code - Rust Edition Help"),
            Line::from(""),
            Line::from("📋 Main Menu:"),
            Line::from("  • ↑/↓ - Navigate menu"),
            Line::from("  • Enter - Select option"),
            Line::from("  • c - Chat mode"),
            Line::from("  • s - Settings"),
            Line::from("  • h - Help"),
            Line::from("  • q/ESC - Exit"),
            Line::from(""),
            Line::from("💬 Chat Mode:"),
            Line::from("  • Type message and press Enter"),
            Line::from("  • ESC - Return to main menu"),
            Line::from(""),
            Line::from("🔧 General:"),
            Line::from("  • ESC - Go back/Cancel"),
            Line::from("  • Ctrl+C - Force quit"),
            Line::from(""),
            Line::from("Press ESC or 'q' to return to main menu"),
        ];

        let help_widget = Paragraph::new(help_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Commands & Shortcuts"))
            .wrap(Wrap { trim: true });
        f.render_widget(help_widget, chunks[1]);

        // 状态栏
        self.render_status_bar(f, chunks[2]);
    }

    /// 渲染退出确认对话框
    fn render_exit_confirm(&mut self, f: &mut Frame) {
        // 先渲染主界面作为背景
        self.render_main(f);

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
            Line::from("Are you sure you want to exit?"),
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
                    .style(Style::default().fg(Color::Red))
            );
        f.render_widget(confirm_widget, popup_area);
    }

    /// 渲染状态栏
    fn render_status_bar(&mut self, f: &mut Frame, area: Rect) {
        let status_text = if self.is_loading {
            format!("⏳ {} | Messages: {}", self.status_message, self.messages.len())
        } else {
            format!("✅ {} | Messages: {}", self.status_message, self.messages.len())
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Status"));
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
