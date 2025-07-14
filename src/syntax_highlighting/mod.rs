//! 语法高亮模块
//! 
//! 使用 syntect 实现代码语法高亮，替代 Highlight.js

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxSet, SyntaxReference};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use std::collections::HashMap;

use crate::error::{ClaudeError, Result};

/// 语法高亮器
pub struct SyntaxHighlighter {
    /// 语法集合
    syntax_set: SyntaxSet,
    /// 主题集合
    theme_set: ThemeSet,
    /// 当前主题
    current_theme: String,
    /// 语法缓存
    syntax_cache: HashMap<String, String>,
}

/// 高亮配置
#[derive(Debug, Clone)]
pub struct HighlightConfig {
    /// 主题名称
    pub theme: String,
    /// 是否显示行号
    pub show_line_numbers: bool,
    /// 行号宽度
    pub line_number_width: usize,
    /// 是否使用终端颜色
    pub use_terminal_colors: bool,
    /// 背景颜色
    pub background_color: Option<String>,
}

/// 高亮结果
#[derive(Debug, Clone)]
pub struct HighlightResult {
    /// 高亮后的代码
    pub highlighted_code: String,
    /// 检测到的语言
    pub detected_language: Option<String>,
    /// 使用的主题
    pub theme: String,
    /// 行数
    pub line_count: usize,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            theme: "base16-ocean.dark".to_string(),
            show_line_numbers: true,
            line_number_width: 4,
            use_terminal_colors: true,
            background_color: None,
        }
    }
}

impl SyntaxHighlighter {
    /// 创建新的语法高亮器
    pub fn new() -> Result<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        
        Ok(Self {
            syntax_set,
            theme_set,
            current_theme: "base16-ocean.dark".to_string(),
            syntax_cache: HashMap::new(),
        })
    }

    /// 设置主题
    pub fn set_theme(&mut self, theme_name: &str) -> Result<()> {
        if !self.theme_set.themes.contains_key(theme_name) {
            return Err(ClaudeError::General(format!(
                "Theme '{}' not found. Available themes: {:?}",
                theme_name,
                self.get_available_themes()
            )));
        }
        
        self.current_theme = theme_name.to_string();
        Ok(())
    }

    /// 获取可用主题列表
    pub fn get_available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    /// 获取可用语言列表
    pub fn get_available_languages(&self) -> Vec<String> {
        self.syntax_set.syntaxes()
            .iter()
            .map(|syntax| syntax.name.clone())
            .collect()
    }

    /// 检测代码语言
    pub fn detect_language(&self, code: &str, file_extension: Option<&str>) -> Option<&SyntaxReference> {
        // 首先尝试通过文件扩展名检测
        if let Some(ext) = file_extension {
            if let Some(syntax) = self.syntax_set.find_syntax_by_extension(ext) {
                return Some(syntax);
            }
        }

        // 尝试通过第一行检测（如 shebang）
        if let Some(first_line) = code.lines().next() {
            if let Some(syntax) = self.syntax_set.find_syntax_by_first_line(first_line) {
                return Some(syntax);
            }
        }

        // 默认使用纯文本
        Some(self.syntax_set.find_syntax_plain_text())
    }

    /// 高亮代码
    pub fn highlight_code(
        &self,
        code: &str,
        language: Option<&str>,
        config: &HighlightConfig,
    ) -> Result<HighlightResult> {
        // 获取语法定义
        let syntax = if let Some(lang) = language {
            self.syntax_set.find_syntax_by_name(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            self.detect_language(code, None)
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        };

        // 获取主题
        let theme = self.theme_set.themes.get(&config.theme)
            .or_else(|| self.theme_set.themes.get(&self.current_theme))
            .ok_or_else(|| ClaudeError::General("No valid theme found".to_string()))?;

        // 创建高亮器
        let mut highlighter = HighlightLines::new(syntax, theme);
        
        let mut highlighted_lines = Vec::new();
        let lines: Vec<&str> = LinesWithEndings::from(code).collect();
        let line_count = lines.len();

        for (line_num, line) in lines.iter().enumerate() {
            let ranges = highlighter.highlight_line(line, &self.syntax_set)
                .map_err(|e| ClaudeError::General(format!("Highlighting error: {}", e)))?;

            let highlighted_line = if config.use_terminal_colors {
                as_24_bit_terminal_escaped(&ranges[..], false)
            } else {
                self.ranges_to_html(&ranges)
            };

            let final_line = if config.show_line_numbers {
                format!(
                    "{:width$} | {}",
                    line_num + 1,
                    highlighted_line,
                    width = config.line_number_width
                )
            } else {
                highlighted_line
            };

            highlighted_lines.push(final_line);
        }

        Ok(HighlightResult {
            highlighted_code: highlighted_lines.join(""),
            detected_language: Some(syntax.name.clone()),
            theme: config.theme.clone(),
            line_count,
        })
    }

    /// 高亮单行代码
    pub fn highlight_line(
        &self,
        line: &str,
        language: &str,
        config: &HighlightConfig,
    ) -> Result<String> {
        let result = self.highlight_code(line, Some(language), config)?;
        Ok(result.highlighted_code)
    }

    /// 高亮代码片段（带上下文）
    pub fn highlight_snippet(
        &self,
        code: &str,
        language: Option<&str>,
        start_line: usize,
        end_line: usize,
        config: &HighlightConfig,
    ) -> Result<HighlightResult> {
        let lines: Vec<&str> = code.lines().collect();
        
        if start_line == 0 || end_line > lines.len() || start_line > end_line {
            return Err(ClaudeError::General(
                "Invalid line range for snippet highlighting".to_string()
            ));
        }

        let snippet_lines = &lines[start_line - 1..end_line];
        let snippet_code = snippet_lines.join("\n");

        let mut result = self.highlight_code(&snippet_code, language, config)?;
        
        // 调整行号
        if config.show_line_numbers {
            let highlighted_lines: Vec<String> = result.highlighted_code
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    let actual_line_num = start_line + i;
                    // 替换行号
                    let parts: Vec<&str> = line.splitn(2, " | ").collect();
                    if parts.len() == 2 {
                        format!(
                            "{:width$} | {}",
                            actual_line_num,
                            parts[1],
                            width = config.line_number_width
                        )
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            
            result.highlighted_code = highlighted_lines.join("\n");
        }

        Ok(result)
    }

    /// 批量高亮多个文件
    pub fn highlight_files(
        &self,
        files: Vec<(String, String, Option<String>)>, // (content, filename, language)
        config: &HighlightConfig,
    ) -> Result<Vec<(String, HighlightResult)>> {
        let mut results = Vec::new();

        for (content, filename, language) in files {
            // 尝试从文件名推断语言
            let detected_language = language.as_deref().or_else(|| {
                std::path::Path::new(&filename)
                    .extension()
                    .and_then(|ext| ext.to_str())
            });

            let result = self.highlight_code(&content, detected_language, config)?;
            results.push((filename, result));
        }

        Ok(results)
    }

    /// 获取语言的文件扩展名
    pub fn get_language_extensions(&self, language: &str) -> Vec<String> {
        if let Some(syntax) = self.syntax_set.find_syntax_by_name(language) {
            syntax.file_extensions.clone()
        } else {
            Vec::new()
        }
    }

    /// 验证语言是否支持
    pub fn is_language_supported(&self, language: &str) -> bool {
        self.syntax_set.find_syntax_by_name(language).is_some() ||
        self.syntax_set.find_syntax_by_extension(language).is_some()
    }

    /// 获取主题预览
    pub fn get_theme_preview(&self, theme_name: &str, sample_code: &str) -> Result<String> {
        let mut config = HighlightConfig::default();
        config.theme = theme_name.to_string();
        config.show_line_numbers = false;
        
        let result = self.highlight_code(sample_code, Some("rust"), &config)?;
        Ok(result.highlighted_code)
    }

    /// 将样式范围转换为 HTML
    fn ranges_to_html(&self, ranges: &[(Style, &str)]) -> String {
        let mut html = String::new();
        
        for (style, text) in ranges {
            let fg = style.foreground;
            let bg = style.background;
            
            html.push_str(&format!(
                "<span style=\"color: #{:02x}{:02x}{:02x}; background-color: #{:02x}{:02x}{:02x};\">",
                fg.r, fg.g, fg.b, bg.r, bg.g, bg.b
            ));
            
            // 转义 HTML 特殊字符
            let escaped_text = text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#39;");
            
            html.push_str(&escaped_text);
            html.push_str("</span>");
        }
        
        html
    }

    /// 创建自定义主题
    pub fn create_custom_theme(&mut self, name: String, base_theme: &str) -> Result<()> {
        if let Some(base) = self.theme_set.themes.get(base_theme).cloned() {
            self.theme_set.themes.insert(name, base);
            Ok(())
        } else {
            Err(ClaudeError::General(format!(
                "Base theme '{}' not found", base_theme
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_available_themes() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let themes = highlighter.get_available_themes();
        assert!(!themes.is_empty());
        assert!(themes.contains(&"base16-ocean.dark".to_string()));
    }

    #[test]
    fn test_available_languages() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let languages = highlighter.get_available_languages();
        assert!(!languages.is_empty());
        assert!(languages.iter().any(|lang| lang.contains("Rust")));
    }

    #[test]
    fn test_language_support() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        assert!(highlighter.is_language_supported("rust"));
        assert!(highlighter.is_language_supported("rs"));
        assert!(!highlighter.is_language_supported("nonexistent"));
    }

    #[tokio::test]
    async fn test_code_highlighting() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let config = HighlightConfig::default();
        
        let code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        
        let result = highlighter.highlight_code(code, Some("rust"), &config);
        assert!(result.is_ok());
        
        let highlight_result = result.unwrap();
        assert!(highlight_result.detected_language.is_some());
        assert!(!highlight_result.highlighted_code.is_empty());
    }
}
