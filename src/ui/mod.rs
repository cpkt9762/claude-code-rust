//! 基础终端交互模块
//! 
//! 实现基础的终端UI和用户交互功能

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color as CrosstermColor, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout, Write};

use crate::error::{ClaudeError, Result};

/// 终端UI管理器
pub struct TerminalUI {
    /// 是否启用原始模式
    raw_mode_enabled: bool,
    /// 消息历史
    messages: Vec<UIMessage>,
    /// 当前输入
    current_input: String,
    /// 光标位置
    cursor_position: usize,
    /// 是否应该退出
    should_quit: bool,
}

/// UI消息
#[derive(Debug, Clone)]
pub struct UIMessage {
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 消息类型
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    /// 用户输入
    User,
    /// 系统响应
    Assistant,
    /// 系统信息
    System,
    /// 错误信息
    Error,
    /// 警告信息
    Warning,
    /// 调试信息
    Debug,
}

/// 终端颜色主题
#[derive(Debug, Clone)]
pub struct ColorTheme {
    pub user_color: Color,
    pub assistant_color: Color,
    pub system_color: Color,
    pub error_color: Color,
    pub warning_color: Color,
    pub debug_color: Color,
    pub border_color: Color,
    pub background_color: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            user_color: Color::Cyan,
            assistant_color: Color::Green,
            system_color: Color::Blue,
            error_color: Color::Red,
            warning_color: Color::Yellow,
            debug_color: Color::DarkGray,
            border_color: Color::White,
            background_color: Color::Black,
        }
    }
}

impl TerminalUI {
    /// 创建新的终端UI
    pub fn new() -> Self {
        Self {
            raw_mode_enabled: false,
            messages: Vec::new(),
            current_input: String::new(),
            cursor_position: 0,
            should_quit: false,
        }
    }

    /// 启用原始模式
    pub fn enable_raw_mode(&mut self) -> Result<()> {
        if !self.raw_mode_enabled {
            terminal::enable_raw_mode()
                .map_err(|e| ClaudeError::General(format!("Failed to enable raw mode: {}", e)))?;
            self.raw_mode_enabled = true;
        }
        Ok(())
    }

    /// 禁用原始模式
    pub fn disable_raw_mode(&mut self) -> Result<()> {
        if self.raw_mode_enabled {
            terminal::disable_raw_mode()
                .map_err(|e| ClaudeError::General(format!("Failed to disable raw mode: {}", e)))?;
            self.raw_mode_enabled = false;
        }
        Ok(())
    }

    /// 添加消息
    pub fn add_message(&mut self, content: String, message_type: MessageType) {
        let message = UIMessage {
            content,
            message_type,
            timestamp: chrono::Utc::now(),
        };
        self.messages.push(message);
    }

    /// 清除消息历史
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// 简单的交互式输入
    pub async fn read_line(&mut self, prompt: &str) -> Result<String> {
        print!("{}", prompt);
        io::stdout().flush()
            .map_err(|e| ClaudeError::General(format!("Failed to flush stdout: {}", e)))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .map_err(|e| ClaudeError::General(format!("Failed to read input: {}", e)))?;

        Ok(input.trim().to_string())
    }

    /// 显示彩色消息
    pub fn print_colored_message(&self, message: &str, color: Color) -> Result<()> {
        // 转换 ratatui::Color 到 crossterm::Color
        let crossterm_color = match color {
            Color::Black => crossterm::style::Color::Black,
            Color::Red => crossterm::style::Color::Red,
            Color::Green => crossterm::style::Color::Green,
            Color::Yellow => crossterm::style::Color::Yellow,
            Color::Blue => crossterm::style::Color::Blue,
            Color::Magenta => crossterm::style::Color::Magenta,
            Color::Cyan => crossterm::style::Color::Cyan,
            Color::Gray => crossterm::style::Color::Grey,
            Color::DarkGray => crossterm::style::Color::DarkGrey,
            Color::LightRed => crossterm::style::Color::DarkRed,
            Color::LightGreen => crossterm::style::Color::DarkGreen,
            Color::LightYellow => crossterm::style::Color::DarkYellow,
            Color::LightBlue => crossterm::style::Color::DarkBlue,
            Color::LightMagenta => crossterm::style::Color::DarkMagenta,
            Color::LightCyan => crossterm::style::Color::DarkCyan,
            Color::White => crossterm::style::Color::White,
            _ => crossterm::style::Color::White, // 默认颜色
        };

        execute!(
            stdout(),
            SetForegroundColor(crossterm_color),
            Print(message),
            ResetColor
        ).map_err(|e| ClaudeError::General(format!("Failed to print colored message: {}", e)))?;

        Ok(())
    }

    /// 显示进度条
    pub fn show_progress(&self, current: usize, total: usize, message: &str) -> Result<()> {
        let percentage = if total > 0 {
            (current * 100) / total
        } else {
            0
        };

        let bar_width = 40;
        let filled = (percentage * bar_width) / 100;
        let empty = bar_width - filled;

        let bar = format!(
            "[{}{}] {}% - {}",
            "=".repeat(filled),
            " ".repeat(empty),
            percentage,
            message
        );

        execute!(
            stdout(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::CurrentLine),
            Print(&bar)
        ).map_err(|e| ClaudeError::General(format!("Failed to show progress: {}", e)))?;

        Ok(())
    }

    /// 显示旋转进度指示器
    pub fn show_spinner(&self, message: &str, step: usize) -> Result<()> {
        let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinners[step % spinners.len()];

        execute!(
            stdout(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::CurrentLine),
            Print(format!("{} {}", spinner, message))
        ).map_err(|e| ClaudeError::General(format!("Failed to show spinner: {}", e)))?;

        Ok(())
    }

    /// 显示多步骤进度
    pub fn show_multi_step_progress(&self, current_step: usize, total_steps: usize, step_name: &str, step_progress: Option<(usize, usize)>) -> Result<()> {
        println!("📋 Step {}/{}: {}", current_step, total_steps, step_name);

        // 显示总体进度
        let overall_percentage = if total_steps > 0 {
            ((current_step - 1) * 100) / total_steps
        } else {
            0
        };

        let bar_width = 30;
        let filled = (overall_percentage * bar_width) / 100;
        let empty = bar_width - filled;
        let overall_bar = "█".repeat(filled) + &"░".repeat(empty);

        println!("Overall: [{}] {}%", overall_bar, overall_percentage);

        // 显示当前步骤进度（如果提供）
        if let Some((current, total)) = step_progress {
            let step_percentage = if total > 0 {
                (current * 100) / total
            } else {
                0
            };

            let step_filled = (step_percentage * bar_width) / 100;
            let step_empty = bar_width - step_filled;
            let step_bar = "█".repeat(step_filled) + &"░".repeat(step_empty);

            println!("Current: [{}] {}% ({}/{})", step_bar, step_percentage, current, total);
        }

        println!();
        Ok(())
    }

    /// 显示加载动画
    pub async fn show_loading_animation(&self, message: &str, duration_ms: u64) -> Result<()> {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_duration = 100; // ms per frame
        let total_frames = duration_ms / frame_duration;

        for i in 0..total_frames {
            let frame = frames[(i % frames.len() as u64) as usize];
            execute!(
                stdout(),
                cursor::MoveToColumn(0),
                terminal::Clear(ClearType::CurrentLine),
                Print(format!("{} {}", frame, message))
            ).map_err(|e| ClaudeError::General(format!("Failed to show loading animation: {}", e)))?;

            tokio::time::sleep(tokio::time::Duration::from_millis(frame_duration)).await;
        }

        // 清除加载动画
        execute!(
            stdout(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::CurrentLine),
            Print("")
        ).map_err(|e| ClaudeError::General(format!("Failed to clear loading animation: {}", e)))?;

        Ok(())
    }

    /// 显示文件处理进度
    pub fn show_file_progress(&self, file_name: &str, current: usize, total: usize, bytes_processed: u64, total_bytes: u64) -> Result<()> {
        let file_percentage = if total > 0 {
            (current * 100) / total
        } else {
            0
        };

        let bar_width = 30;
        let filled = (file_percentage * bar_width) / 100;
        let empty = bar_width - filled;
        let bar = "█".repeat(filled) + &"░".repeat(empty);

        execute!(
            stdout(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::CurrentLine),
            Print(format!("📄 {} [{}] {}% ({}/{}) | {} bytes",
                         file_name, bar, file_percentage, current, total,
                         self.format_bytes(bytes_processed)))
        ).map_err(|e| ClaudeError::General(format!("Failed to show file progress: {}", e)))?;

        Ok(())
    }

    /// 格式化字节数
    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// 显示实时状态更新
    pub fn show_status_update(&self, status: &str, color: Color) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");

        execute!(
            stdout(),
            SetForegroundColor(CrosstermColor::DarkGrey),
            Print(format!("[{}] ", timestamp)),
            SetForegroundColor(match color {
                Color::Red => CrosstermColor::Red,
                Color::Green => CrosstermColor::Green,
                Color::Yellow => CrosstermColor::Yellow,
                Color::Blue => CrosstermColor::Blue,
                Color::Cyan => CrosstermColor::Cyan,
                Color::Magenta => CrosstermColor::Magenta,
                _ => CrosstermColor::White,
            }),
            Print(status),
            ResetColor,
            Print("\n")
        ).map_err(|e| ClaudeError::General(format!("Failed to show status update: {}", e)))?;

        Ok(())
    }

    /// 显示确认对话框
    pub async fn confirm(&mut self, message: &str) -> Result<bool> {
        let prompt = format!("{} (y/N): ", message);
        let input = self.read_line(&prompt).await?;
        
        Ok(matches!(input.to_lowercase().as_str(), "y" | "yes"))
    }

    /// 显示选择菜单
    pub async fn select_option(&mut self, message: &str, options: &[&str]) -> Result<usize> {
        println!("{}", message);
        for (i, option) in options.iter().enumerate() {
            println!("  {}. {}", i + 1, option);
        }

        loop {
            let input = self.read_line("Select option (number): ").await?;
            
            if let Ok(choice) = input.parse::<usize>() {
                if choice > 0 && choice <= options.len() {
                    return Ok(choice - 1);
                }
            }
            
            println!("Invalid choice. Please enter a number between 1 and {}", options.len());
        }
    }

    /// 启动全屏TUI模式
    pub async fn start_tui_mode(&mut self, theme: ColorTheme) -> Result<()> {
        self.enable_raw_mode()?;
        
        let mut stdout = stdout();
        execute!(stdout, terminal::EnterAlternateScreen)?;
        
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .map_err(|e| ClaudeError::General(format!("Failed to create terminal: {}", e)))?;

        let result = self.run_tui_loop(&mut terminal, theme).await;

        // 清理
        execute!(
            terminal.backend_mut(),
            terminal::LeaveAlternateScreen
        )?;
        
        self.disable_raw_mode()?;
        
        result
    }

    /// TUI主循环
    async fn run_tui_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        theme: ColorTheme,
    ) -> Result<()> {
        let mut last_tick = std::time::Instant::now();
        let tick_rate = std::time::Duration::from_millis(250);

        loop {
            // 绘制UI
            terminal.draw(|f| self.draw_ui(f, &theme))
                .map_err(|e| ClaudeError::General(format!("Failed to draw UI: {}", e)))?;

            // 计算超时时间
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| std::time::Duration::from_secs(0));

            // 处理事件
            if event::poll(timeout)
                .map_err(|e| ClaudeError::General(format!("Failed to poll events: {}", e)))?
            {
                if let Event::Key(key) = event::read()
                    .map_err(|e| ClaudeError::General(format!("Failed to read event: {}", e)))?
                {
                    if self.handle_key_event(key).await? {
                        break;
                    }
                }
            }

            // 定期更新
            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = std::time::Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// 定期更新处理
    fn on_tick(&mut self) {
        // 这里可以添加定期更新的逻辑
        // 例如：更新状态、刷新数据等
    }

    /// 绘制UI
    fn draw_ui(&self, f: &mut Frame, theme: &ColorTheme) {
        // 主布局：顶部状态栏 + 中间内容 + 底部输入
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // 状态栏
                Constraint::Min(0),      // 主内容区域
                Constraint::Length(3),   // 输入区域
            ])
            .split(f.size());

        // 绘制状态栏
        self.draw_status_bar(f, main_chunks[0], theme);

        // 中间内容区域：左侧消息 + 右侧信息面板
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75), // 消息区域
                Constraint::Percentage(25), // 信息面板
            ])
            .split(main_chunks[1]);

        // 绘制消息区域
        self.draw_messages(f, content_chunks[0], theme);

        // 绘制信息面板
        self.draw_info_panel(f, content_chunks[1], theme);

        // 绘制输入区域
        self.draw_input(f, main_chunks[2], theme);
    }

    /// 绘制状态栏
    fn draw_status_bar(&self, f: &mut Frame, area: Rect, theme: &ColorTheme) {
        let status_text = format!(
            " Claude Code Rust v0.1.0 | Messages: {} | Press Ctrl+C or Esc to exit ",
            self.messages.len()
        );

        let status_bar = Paragraph::new(status_text)
            .style(Style::default().fg(theme.system_color).add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_color))
            );

        f.render_widget(status_bar, area);
    }

    /// 绘制信息面板
    fn draw_info_panel(&self, f: &mut Frame, area: Rect, theme: &ColorTheme) {
        // 分割信息面板为多个部分
        let info_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),   // 快捷键帮助
                Constraint::Min(0),      // 系统信息
            ])
            .split(area);

        // 绘制快捷键帮助
        self.draw_help_panel(f, info_chunks[0], theme);

        // 绘制系统信息
        self.draw_system_info(f, info_chunks[1], theme);
    }

    /// 绘制帮助面板
    fn draw_help_panel(&self, f: &mut Frame, area: Rect, theme: &ColorTheme) {
        let help_text = vec![
            Line::from("Keyboard Shortcuts:"),
            Line::from(""),
            Line::from("Enter    - Send message"),
            Line::from("Ctrl+C   - Exit"),
            Line::from("Esc      - Exit"),
            Line::from("Ctrl+L   - Clear screen"),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .border_style(Style::default().fg(theme.border_color))
            )
            .wrap(Wrap { trim: true });

        f.render_widget(help_paragraph, area);
    }

    /// 绘制系统信息
    fn draw_system_info(&self, f: &mut Frame, area: Rect, theme: &ColorTheme) {
        let info_text = vec![
            Line::from("System Information:"),
            Line::from(""),
            Line::from(format!("Memory: {}MB", self.get_memory_usage())),
            Line::from(format!("Uptime: {}s", self.get_uptime())),
            Line::from("Status: Running"),
        ];

        let info_paragraph = Paragraph::new(info_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("System")
                    .border_style(Style::default().fg(theme.border_color))
            )
            .wrap(Wrap { trim: true });

        f.render_widget(info_paragraph, area);
    }

    /// 获取内存使用情况（模拟）
    fn get_memory_usage(&self) -> u64 {
        // 这里可以实现真实的内存使用统计
        10 + (self.messages.len() as u64 / 10)
    }

    /// 获取运行时间（模拟）
    fn get_uptime(&self) -> u64 {
        // 这里可以实现真实的运行时间统计
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() % 3600 // 简化显示
    }

    /// 绘制消息区域
    fn draw_messages(&self, f: &mut Frame, area: Rect, theme: &ColorTheme) {
        let messages: Vec<ListItem> = self.messages
            .iter()
            .map(|msg| {
                let color = match msg.message_type {
                    MessageType::User => theme.user_color,
                    MessageType::Assistant => theme.assistant_color,
                    MessageType::System => theme.system_color,
                    MessageType::Error => theme.error_color,
                    MessageType::Warning => theme.warning_color,
                    MessageType::Debug => theme.debug_color,
                };

                let prefix = match msg.message_type {
                    MessageType::User => "You",
                    MessageType::Assistant => "Claude",
                    MessageType::System => "System",
                    MessageType::Error => "Error",
                    MessageType::Warning => "Warning",
                    MessageType::Debug => "Debug",
                };

                let content = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", prefix),
                        Style::default().fg(color).add_modifier(Modifier::BOLD)
                    ),
                    Span::raw(&msg.content),
                ]);

                ListItem::new(content)
            })
            .collect();

        let messages_list = List::new(messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Conversation")
                    .border_style(Style::default().fg(theme.border_color))
            );

        f.render_widget(messages_list, area);
    }

    /// 绘制输入区域
    fn draw_input(&self, f: &mut Frame, area: Rect, theme: &ColorTheme) {
        let input = Paragraph::new(self.current_input.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Input")
                    .border_style(Style::default().fg(theme.border_color))
            )
            .wrap(Wrap { trim: true });

        f.render_widget(input, area);
    }

    /// 处理键盘事件
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            // 退出快捷键
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Ok(true);
            }
            KeyCode::Esc => {
                self.should_quit = true;
                return Ok(true);
            }

            // 清屏快捷键
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.clear_messages();
                self.add_message(
                    "Screen cleared.".to_string(),
                    MessageType::System,
                );
            }

            // 删除整个单词
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                while self.cursor_position > 0 {
                    let prev_char = self.current_input.chars().nth(self.cursor_position - 1);
                    if prev_char == Some(' ') {
                        break;
                    }
                    self.current_input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }

            // 清空输入
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.current_input.clear();
                self.cursor_position = 0;
            }

            // 发送消息
            KeyCode::Enter => {
                if !self.current_input.trim().is_empty() {
                    let input = self.current_input.clone();
                    self.add_message(input.clone(), MessageType::User);
                    self.current_input.clear();
                    self.cursor_position = 0;

                    // 处理特殊命令
                    if input.starts_with('/') {
                        self.handle_tui_command(&input).await?;
                    } else {
                        // 模拟Claude响应
                        self.add_message(
                            format!("Echo: {}", input),
                            MessageType::Assistant,
                        );
                    }
                }
            }

            // 文本输入
            KeyCode::Char(c) => {
                self.current_input.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }

            // 删除字符
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.current_input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }

            // 光标移动
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.current_input.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.current_input.len();
            }

            _ => {}
        }

        Ok(false)
    }

    /// 处理TUI模式下的命令
    async fn handle_tui_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command[1..].split_whitespace().collect();

        match parts.get(0) {
            Some(&"help") => {
                self.add_message(
                    "Available commands: /help, /clear, /status, /quit".to_string(),
                    MessageType::System,
                );
            }
            Some(&"clear") => {
                self.clear_messages();
                self.add_message(
                    "Messages cleared.".to_string(),
                    MessageType::System,
                );
            }
            Some(&"status") => {
                self.add_message(
                    format!("Status: {} messages, {}MB memory",
                           self.messages.len(),
                           self.get_memory_usage()),
                    MessageType::System,
                );
            }
            Some(&"quit") | Some(&"exit") => {
                self.should_quit = true;
            }
            _ => {
                self.add_message(
                    format!("Unknown command: {}. Type /help for available commands.", command),
                    MessageType::Error,
                );
            }
        }

        Ok(())
    }

    /// 显示帮助信息
    pub fn show_help(&self) -> Result<()> {
        self.show_detailed_help()
    }

    /// 显示详细帮助信息
    pub fn show_detailed_help(&self) -> Result<()> {
        println!("📚 Claude Code Rust - Interactive Help");
        println!("=====================================");
        println!();

        println!("🔧 Basic Commands:");
        println!("  help, ?          - Show this help message");
        println!("  exit, quit       - Exit the program");
        println!("  clear            - Clear the screen");
        println!("  version          - Show version information");
        println!();

        println!("💬 Chat Commands:");
        println!("  <message>        - Send a message to Claude");
        println!("  /retry           - Retry the last message");
        println!("  /undo            - Undo the last message");
        println!("  /new             - Start a new conversation");
        println!();

        println!("📊 System Commands:");
        println!("  /status          - Show system status");
        println!("  /doctor          - Run health check");
        println!("  /cost [days]     - Show cost information");
        println!("  /usage           - Show API usage statistics");
        println!();

        println!("⚙️  Configuration Commands:");
        println!("  /config show     - Show current configuration");
        println!("  /config set <key> <value> - Set configuration value");
        println!("  /config get <key> - Get configuration value");
        println!("  /config reset    - Reset to default configuration");
        println!();

        println!("🔌 MCP Server Commands:");
        println!("  /mcp list        - List MCP servers");
        println!("  /mcp start <name> - Start MCP server");
        println!("  /mcp stop <name> - Stop MCP server");
        println!("  /mcp add <name> <command> [args...] - Add MCP server");
        println!();

        println!("🌿 Git Commands:");
        println!("  /git status      - Show git status");
        println!("  /git add <files> - Add files to staging");
        println!("  /git commit -m \"message\" - Commit changes");
        println!("  /git log         - Show commit history");
        println!("  /git branch      - Show branches");
        println!("  /git diff [file] - Show differences");
        println!();

        println!("🎨 Code Commands:");
        println!("  /highlight <lang> <code> - Highlight code snippet");
        println!("  /file <path>     - Show file with syntax highlighting");
        println!("  /edit <path>     - Edit file (if editor is configured)");
        println!();

        println!("💡 Tips:");
        println!("  - Use Tab for command completion");
        println!("  - Use Ctrl+C to interrupt operations");
        println!("  - Use Ctrl+D to exit");
        println!("  - Commands are case-insensitive");
        println!("  - Use /help <command> for detailed help on specific commands");
        println!();

        Ok(())
    }

    /// 高亮代码并显示
    pub fn highlight_and_display_code(&self, language: &str, code: &str) -> Result<()> {
        #[cfg(feature = "syntax-highlighting")]
        {
            use crate::syntax_highlighting::{SyntaxHighlighter, HighlightConfig};

            let config = HighlightConfig::default();
            let highlighter = SyntaxHighlighter::new()?;
            match highlighter.highlight_code(code, Some(language), &config) {
                Ok(result) => {
                    println!("🎨 Highlighted {} code:", language);
                    println!("```");
                    println!("{}", result.highlighted_code);
                    println!("```");
                }
                Err(e) => {
                    println!("❌ Failed to highlight code: {}", e);
                    println!("Raw code:");
                    println!("```");
                    println!("{}", code);
                    println!("```");
                }
            }
        }
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            println!("🎨 {} code (syntax highlighting disabled):", language);
            println!("```");
            println!("{}", code);
            println!("```");
        }
        Ok(())
    }

    /// 检测代码语言
    pub fn detect_language_from_content(&self, content: &str) -> String {
        // 简单的语言检测逻辑
        if content.contains("fn main()") || content.contains("use std::") {
            "rust".to_string()
        } else if content.contains("function ") || content.contains("const ") || content.contains("let ") {
            "javascript".to_string()
        } else if content.contains("def ") || content.contains("import ") {
            "python".to_string()
        } else if content.contains("#include") || content.contains("int main") {
            "c".to_string()
        } else if content.contains("class ") && content.contains("public ") {
            "java".to_string()
        } else if content.contains("SELECT ") || content.contains("FROM ") {
            "sql".to_string()
        } else if content.contains("<!DOCTYPE") || content.contains("<html") {
            "html".to_string()
        } else if content.contains("{") && content.contains("}") {
            "json".to_string()
        } else {
            "text".to_string()
        }
    }

    /// 在TUI中显示高亮代码
    pub fn render_highlighted_code(&self, frame: &mut Frame, area: Rect, code: &str, language: &str) {
        #[cfg(feature = "syntax-highlighting")]
        {
            use crate::syntax_highlighting::SyntaxHighlighter;

            if let Ok(highlighter) = SyntaxHighlighter::new() {
                use crate::syntax_highlighting::HighlightConfig;
                let config = HighlightConfig::default();
                if let Ok(result) = highlighter.highlight_code(code, Some(language), &config) {
                    let paragraph = Paragraph::new(result.highlighted_code)
                        .block(Block::default()
                            .title(format!("Code ({})", language))
                            .borders(Borders::ALL))
                        .wrap(Wrap { trim: true });
                    frame.render_widget(paragraph, area);
                    return;
                }
            }
        }

        // 回退到普通显示
        let lines: Vec<Line> = code.lines()
            .map(|line| Line::from(Span::raw(line)))
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(Block::default()
                .title(format!("Code ({})", language))
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    /// 显示代码差异（用于Git diff等）
    pub fn display_code_diff(&self, diff_content: &str) -> Result<()> {
        println!("📋 Code Diff:");
        println!("=============");

        for line in diff_content.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                // 添加的行 - 绿色
                execute!(
                    stdout(),
                    SetForegroundColor(CrosstermColor::Green),
                    Print(line),
                    Print("\n"),
                    ResetColor
                )?;
            } else if line.starts_with('-') && !line.starts_with("---") {
                // 删除的行 - 红色
                execute!(
                    stdout(),
                    SetForegroundColor(CrosstermColor::Red),
                    Print(line),
                    Print("\n"),
                    ResetColor
                )?;
            } else if line.starts_with("@@") {
                // 位置信息 - 蓝色
                execute!(
                    stdout(),
                    SetForegroundColor(CrosstermColor::Blue),
                    Print(line),
                    Print("\n"),
                    ResetColor
                )?;
            } else {
                // 普通行
                println!("{}", line);
            }
        }

        Ok(())
    }

    /// 显示特定命令的帮助
    pub fn show_command_help(&self, command: &str) -> Result<()> {
        match command.to_lowercase().as_str() {
            "config" => self.show_config_help(),
            "mcp" => self.show_mcp_help(),
            "git" => self.show_git_help(),
            "highlight" => self.show_highlight_help(),
            "memory" => self.show_memory_help(),
            "permissions" => self.show_permissions_help(),
            "file" | "files" => self.show_file_help(),
            _ => {
                println!("❓ No detailed help available for '{}'", command);
                println!("Use '/help' to see all available commands.");
                Ok(())
            }
        }
    }

    /// 显示配置命令帮助
    fn show_config_help(&self) -> Result<()> {
        println!("⚙️  Configuration Commands Help");
        println!("==============================");
        println!();
        println!("Available configuration commands:");
        println!("  /config show                 - Display current configuration");
        println!("  /config set <key> <value>    - Set a configuration value");
        println!("  /config get <key>            - Get a configuration value");
        println!("  /config reset                - Reset to default configuration");
        println!();
        println!("Common configuration keys:");
        println!("  api.api_key                  - Anthropic API key");
        println!("  api.base_url                 - API base URL");
        println!("  ui.theme                     - UI theme (dark/light)");
        println!("  ui.vim_mode                  - Enable vim-style keybindings");
        println!("  permissions.require_confirmation - Require confirmation for actions");
        println!();
        println!("Examples:");
        println!("  /config set api.api_key sk-ant-...");
        println!("  /config set ui.theme dark");
        println!("  /config get api.base_url");
        println!();
        Ok(())
    }

    /// 显示MCP命令帮助
    fn show_mcp_help(&self) -> Result<()> {
        println!("🔌 MCP Server Commands Help");
        println!("===========================");
        println!();
        println!("MCP (Model Context Protocol) allows Claude to interact with external tools.");
        println!();
        println!("Available MCP commands:");
        println!("  /mcp list                    - List all configured MCP servers");
        println!("  /mcp start <name>            - Start an MCP server");
        println!("  /mcp stop <name>             - Stop an MCP server");
        println!("  /mcp add <name> <cmd> [args] - Add a new MCP server");
        println!("  /mcp remove <name>           - Remove an MCP server");
        println!();
        println!("Examples:");
        println!("  /mcp add filesystem npx @modelcontextprotocol/server-filesystem /path/to/dir");
        println!("  /mcp start filesystem");
        println!("  /mcp list");
        println!("  /mcp stop filesystem");
        println!();
        Ok(())
    }

    /// 显示Git命令帮助
    fn show_git_help(&self) -> Result<()> {
        println!("🌿 Git Commands Help");
        println!("===================");
        println!();
        println!("Git integration allows you to manage version control from within Claude Code.");
        println!();
        println!("Available Git commands:");
        println!("  /git status                  - Show repository status");
        println!("  /git add <files...>          - Add files to staging area");
        println!("  /git commit -m \"message\"     - Commit staged changes");
        println!("  /git log [--limit N]         - Show commit history");
        println!("  /git branch                  - List branches");
        println!("  /git checkout <branch>       - Switch to branch");
        println!("  /git checkout -b <branch>    - Create and switch to new branch");
        println!("  /git diff [file]             - Show differences");
        println!();
        println!("Examples:");
        println!("  /git status");
        println!("  /git add src/main.rs");
        println!("  /git commit -m \"Fix bug in main function\"");
        println!("  /git log --limit 5");
        println!("  /git checkout -b feature/new-ui");
        println!();
        Ok(())
    }

    /// 显示语法高亮命令帮助
    fn show_highlight_help(&self) -> Result<()> {
        println!("🎨 Syntax Highlighting Help");
        println!("===========================");
        println!();
        println!("Syntax highlighting makes code easier to read and understand.");
        println!();
        println!("Available highlighting commands:");
        println!("  /highlight <lang> <code>     - Highlight code snippet");
        println!("  /file <path>                 - Show file with highlighting");
        println!("  /highlight languages         - List supported languages");
        println!();
        println!("Supported languages include:");
        println!("  rust, javascript, typescript, python, c, cpp, java, go,");
        println!("  html, css, json, xml, yaml, toml, sql, bash, markdown");
        println!();
        println!("Examples:");
        println!("  /highlight rust \"fn main() {{ println!(\\\"Hello!\\\"); }}\"");
        println!("  /file src/main.rs");
        println!("  /highlight languages");
        println!();
        Ok(())
    }

    /// 显示内存命令帮助
    fn show_memory_help(&self) -> Result<()> {
        println!("🧠 Memory Commands Help");
        println!("=======================");
        println!();
        println!("Memory allows Claude to remember important information across conversations.");
        println!();
        println!("Available memory commands:");
        println!("  /memory show                 - Display all memory contents");
        println!("  /memory add <content>        - Add information to memory");
        println!("  /memory clear                - Clear all memory");
        println!("  /memory search <query>       - Search memory contents");
        println!();
        println!("Examples:");
        println!("  /memory add \"User prefers TypeScript over JavaScript\"");
        println!("  /memory add \"Project uses React with Vite\"");
        println!("  /memory show");
        println!("  /memory search \"TypeScript\"");
        println!();
        Ok(())
    }

    /// 显示权限命令帮助
    fn show_permissions_help(&self) -> Result<()> {
        println!("🔐 Permissions Commands Help");
        println!("============================");
        println!();
        println!("Permissions control what tools and actions Claude can perform.");
        println!();
        println!("Available permission commands:");
        println!("  /permissions show            - Show current permission settings");
        println!("  /permissions allow <tool>    - Allow a specific tool");
        println!("  /permissions deny <tool>     - Deny a specific tool");
        println!("  /permissions reset           - Reset to default permissions");
        println!();
        println!("Common tools:");
        println!("  file_operations, git_operations, mcp_servers, network_requests,");
        println!("  code_execution, system_commands");
        println!();
        println!("Examples:");
        println!("  /permissions allow git_operations");
        println!("  /permissions deny system_commands");
        println!("  /permissions show");
        println!();
        Ok(())
    }

    /// 显示文件命令帮助
    fn show_file_help(&self) -> Result<()> {
        println!("📁 File Commands Help");
        println!("====================");
        println!();
        println!("File commands help you navigate and manage files and directories.");
        println!();
        println!("Available file commands:");
        println!("  /ls [path]                   - List directory contents");
        println!("  /cd <path>                   - Change working directory");
        println!("  /pwd                         - Show current directory");
        println!("  /find <pattern>              - Search for files matching pattern");
        println!("  /file <path>                 - Display file with syntax highlighting");
        println!("  /edit <path>                 - Edit file (if editor configured)");
        println!();
        println!("Examples:");
        println!("  /ls src/");
        println!("  /cd ../");
        println!("  /pwd");
        println!("  /find \"*.rs\"");
        println!("  /file src/main.rs");
        println!();
        Ok(())
    }

    /// 显示快捷键帮助
    pub fn show_keyboard_shortcuts(&self) -> Result<()> {
        println!("⌨️  Keyboard Shortcuts");
        println!("=====================");
        println!();
        println!("Navigation:");
        println!("  ↑/↓                         - Navigate command history");
        println!("  Ctrl+A                       - Move to beginning of line");
        println!("  Ctrl+E                       - Move to end of line");
        println!("  Ctrl+U                       - Clear line");
        println!("  Ctrl+L                       - Clear screen");
        println!();
        println!("Editing:");
        println!("  Tab                          - Auto-complete commands");
        println!("  Ctrl+W                       - Delete word backwards");
        println!("  Ctrl+K                       - Delete to end of line");
        println!();
        println!("Control:");
        println!("  Ctrl+C                       - Interrupt current operation");
        println!("  Ctrl+D                       - Exit program");
        println!("  Ctrl+Z                       - Suspend program (Unix)");
        println!();
        println!("TUI Mode (when available):");
        println!("  Esc                          - Exit TUI mode");
        println!("  F1                           - Show help");
        println!("  F2                           - Toggle panels");
        println!("  Tab                          - Switch between panels");
        println!();
        Ok(())
    }
}

impl Drop for TerminalUI {
    fn drop(&mut self) {
        if self.raw_mode_enabled {
            let _ = self.disable_raw_mode();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_ui_creation() {
        let ui = TerminalUI::new();
        assert!(!ui.raw_mode_enabled);
        assert!(ui.messages.is_empty());
        assert!(ui.current_input.is_empty());
    }

    #[test]
    fn test_add_message() {
        let mut ui = TerminalUI::new();
        ui.add_message("Test message".to_string(), MessageType::User);
        
        assert_eq!(ui.messages.len(), 1);
        assert_eq!(ui.messages[0].content, "Test message");
        assert_eq!(ui.messages[0].message_type, MessageType::User);
    }

    #[test]
    fn test_clear_messages() {
        let mut ui = TerminalUI::new();
        ui.add_message("Test message".to_string(), MessageType::User);
        ui.clear_messages();
        
        assert!(ui.messages.is_empty());
    }

    #[test]
    fn test_color_theme_default() {
        let theme = ColorTheme::default();
        assert_eq!(theme.user_color, Color::Cyan);
        assert_eq!(theme.assistant_color, Color::Green);
    }
}
