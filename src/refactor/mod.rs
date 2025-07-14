//! 代码编辑和重构工具模块
//! 
//! 实现代码编辑、重构建议和自动修复功能

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{ClaudeError, Result};
use crate::fs::FileSystemManager;

/// 重构建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorSuggestion {
    /// 建议ID
    pub id: String,
    /// 建议类型
    pub suggestion_type: SuggestionType,
    /// 文件路径
    pub file_path: PathBuf,
    /// 行号范围
    pub line_range: (usize, usize),
    /// 原始代码
    pub original_code: String,
    /// 建议的代码
    pub suggested_code: String,
    /// 建议描述
    pub description: String,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
    /// 影响范围
    pub impact: ImpactLevel,
}

/// 建议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    /// 重命名变量/函数
    Rename { from: String, to: String },
    /// 提取函数
    ExtractFunction { function_name: String },
    /// 内联函数
    InlineFunction { function_name: String },
    /// 移除未使用的代码
    RemoveUnused,
    /// 简化表达式
    SimplifyExpression,
    /// 格式化代码
    FormatCode,
    /// 添加文档注释
    AddDocumentation,
    /// 性能优化
    PerformanceOptimization,
    /// 安全修复
    SecurityFix,
}

/// 影响级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    /// 低影响（仅格式化等）
    Low,
    /// 中等影响（重命名等）
    Medium,
    /// 高影响（结构性变更）
    High,
}

/// 代码编辑操作
#[derive(Debug, Clone)]
pub struct EditOperation {
    /// 操作类型
    pub operation_type: EditType,
    /// 文件路径
    pub file_path: PathBuf,
    /// 位置信息
    pub position: EditPosition,
    /// 内容
    pub content: String,
}

/// 编辑类型
#[derive(Debug, Clone)]
pub enum EditType {
    /// 插入
    Insert,
    /// 删除
    Delete,
    /// 替换
    Replace,
    /// 移动
    Move { to_position: EditPosition },
}

/// 编辑位置
#[derive(Debug, Clone)]
pub struct EditPosition {
    /// 行号（从1开始）
    pub line: usize,
    /// 列号（从1开始）
    pub column: usize,
    /// 长度（对于替换和删除操作）
    pub length: Option<usize>,
}

/// 重构引擎
pub struct RefactorEngine {
    /// 文件系统管理器
    fs_manager: FileSystemManager,
    /// 语言特定的规则
    language_rules: HashMap<String, Vec<RefactorRule>>,
    /// 自定义规则
    custom_rules: Vec<RefactorRule>,
}

/// 重构规则
#[derive(Debug, Clone)]
pub struct RefactorRule {
    /// 规则名称
    pub name: String,
    /// 适用的文件扩展名
    pub file_extensions: Vec<String>,
    /// 匹配模式
    pub pattern: Regex,
    /// 替换模板
    pub replacement: String,
    /// 建议描述
    pub description: String,
    /// 置信度
    pub confidence: f32,
    /// 影响级别
    pub impact: ImpactLevel,
}

impl RefactorEngine {
    /// 创建新的重构引擎
    pub fn new() -> Self {
        let mut engine = Self {
            fs_manager: FileSystemManager::new(vec![]),
            language_rules: HashMap::new(),
            custom_rules: Vec::new(),
        };
        
        engine.initialize_default_rules();
        engine
    }

    /// 分析文件并生成重构建议
    pub async fn analyze_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<RefactorSuggestion>> {
        let file_path = file_path.as_ref();
        let content = self.fs_manager.read_file(file_path).await?;
        
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let mut suggestions = Vec::new();

        // 应用语言特定规则
        if let Some(rules) = self.language_rules.get(extension) {
            for rule in rules {
                suggestions.extend(self.apply_rule(&content, file_path, rule)?);
            }
        }

        // 应用自定义规则
        for rule in &self.custom_rules {
            if rule.file_extensions.contains(&extension.to_string()) {
                suggestions.extend(self.apply_rule(&content, file_path, rule)?);
            }
        }

        Ok(suggestions)
    }

    /// 分析目录并生成重构建议
    pub async fn analyze_directory<P: AsRef<Path>>(&self, dir_path: P) -> Result<Vec<RefactorSuggestion>> {
        let dir_path = dir_path.as_ref();
        let mut all_suggestions = Vec::new();

        let entries = self.fs_manager.list_directory(dir_path).await?;
        
        for entry in entries {
            if entry.is_file() {
                if let Some(ext) = entry.extension().and_then(|e| e.to_str()) {
                    if self.is_supported_language(ext) {
                        let suggestions = self.analyze_file(&entry).await?;
                        all_suggestions.extend(suggestions);
                    }
                }
            } else if entry.is_dir() {
                // 递归分析子目录
                let sub_suggestions = self.analyze_directory(&entry).await?;
                all_suggestions.extend(sub_suggestions);
            }
        }

        Ok(all_suggestions)
    }

    /// 应用重构建议
    pub async fn apply_suggestion(&self, suggestion: &RefactorSuggestion) -> Result<()> {
        let content = self.fs_manager.read_file(&suggestion.file_path).await?;
        let lines: Vec<&str> = content.lines().collect();

        if suggestion.line_range.1 > lines.len() {
            return Err(ClaudeError::General(
                "Line range exceeds file length".to_string()
            ));
        }

        let mut new_lines = lines.clone();
        
        // 替换指定范围的行
        let start_idx = suggestion.line_range.0.saturating_sub(1);
        let end_idx = suggestion.line_range.1.saturating_sub(1);
        
        // 移除原始行
        for _ in start_idx..=end_idx {
            if start_idx < new_lines.len() {
                new_lines.remove(start_idx);
            }
        }

        // 插入新代码
        let suggested_lines: Vec<&str> = suggestion.suggested_code.lines().collect();
        for (i, line) in suggested_lines.iter().enumerate() {
            new_lines.insert(start_idx + i, line);
        }

        let new_content = new_lines.join("\n");
        self.fs_manager.write_file(&suggestion.file_path, &new_content).await?;

        Ok(())
    }

    /// 批量应用重构建议
    pub async fn apply_suggestions(&self, suggestions: &[RefactorSuggestion]) -> Result<Vec<String>> {
        let mut applied = Vec::new();
        let mut errors = Vec::new();

        for suggestion in suggestions {
            match self.apply_suggestion(suggestion).await {
                Ok(()) => applied.push(suggestion.id.clone()),
                Err(e) => errors.push(format!("Failed to apply {}: {}", suggestion.id, e)),
            }
        }

        if !errors.is_empty() {
            return Err(ClaudeError::General(format!(
                "Some suggestions failed to apply: {}", errors.join(", ")
            )));
        }

        Ok(applied)
    }

    /// 执行编辑操作
    pub async fn execute_edit(&self, operation: &EditOperation) -> Result<()> {
        let content = self.fs_manager.read_file(&operation.file_path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let mut new_lines = lines.clone();

        match &operation.operation_type {
            EditType::Insert => {
                let line_idx = operation.position.line.saturating_sub(1);
                if line_idx <= new_lines.len() {
                    new_lines.insert(line_idx, &operation.content);
                }
            }
            EditType::Delete => {
                let line_idx = operation.position.line.saturating_sub(1);
                if line_idx < new_lines.len() {
                    new_lines.remove(line_idx);
                }
            }
            EditType::Replace => {
                let line_idx = operation.position.line.saturating_sub(1);
                if line_idx < new_lines.len() {
                    new_lines[line_idx] = &operation.content;
                }
            }
            EditType::Move { to_position } => {
                let from_idx = operation.position.line.saturating_sub(1);
                let to_idx = to_position.line.saturating_sub(1);
                
                if from_idx < new_lines.len() && to_idx <= new_lines.len() {
                    let line = new_lines.remove(from_idx);
                    new_lines.insert(to_idx, line);
                }
            }
        }

        let new_content = new_lines.join("\n");
        self.fs_manager.write_file(&operation.file_path, &new_content).await?;

        Ok(())
    }

    /// 添加自定义规则
    pub fn add_custom_rule(&mut self, rule: RefactorRule) {
        self.custom_rules.push(rule);
    }

    /// 获取支持的语言列表
    pub fn get_supported_languages(&self) -> Vec<String> {
        self.language_rules.keys().cloned().collect()
    }

    /// 检查是否支持指定语言
    fn is_supported_language(&self, extension: &str) -> bool {
        self.language_rules.contains_key(extension) ||
        self.custom_rules.iter().any(|rule| rule.file_extensions.contains(&extension.to_string()))
    }

    /// 应用单个规则
    fn apply_rule(
        &self,
        content: &str,
        file_path: &Path,
        rule: &RefactorRule,
    ) -> Result<Vec<RefactorSuggestion>> {
        let mut suggestions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            if let Some(captures) = rule.pattern.captures(line) {
                let suggestion = RefactorSuggestion {
                    id: format!("{}_{}", rule.name, line_num + 1),
                    suggestion_type: self.determine_suggestion_type(&rule.name, &captures),
                    file_path: file_path.to_path_buf(),
                    line_range: (line_num + 1, line_num + 1),
                    original_code: line.to_string(),
                    suggested_code: self.apply_replacement(&rule.replacement, &captures),
                    description: rule.description.clone(),
                    confidence: rule.confidence,
                    impact: rule.impact.clone(),
                };
                suggestions.push(suggestion);
            }
        }

        Ok(suggestions)
    }

    /// 确定建议类型
    fn determine_suggestion_type(&self, rule_name: &str, _captures: &regex::Captures) -> SuggestionType {
        match rule_name {
            "remove_unused_imports" => SuggestionType::RemoveUnused,
            "format_code" => SuggestionType::FormatCode,
            "add_documentation" => SuggestionType::AddDocumentation,
            _ => SuggestionType::SimplifyExpression,
        }
    }

    /// 应用替换模板
    fn apply_replacement(&self, template: &str, captures: &regex::Captures) -> String {
        let mut result = template.to_string();
        
        for (i, capture) in captures.iter().enumerate() {
            if let Some(matched) = capture {
                let placeholder = format!("${}", i);
                result = result.replace(&placeholder, matched.as_str());
            }
        }
        
        result
    }

    /// 初始化默认规则
    fn initialize_default_rules(&mut self) {
        // Rust 规则
        let rust_rules = vec![
            RefactorRule {
                name: "remove_unused_imports".to_string(),
                file_extensions: vec!["rs".to_string()],
                pattern: Regex::new(r"^use\s+([^;]+);$").unwrap(),
                replacement: "// Unused import: use $1;".to_string(),
                description: "Remove unused import".to_string(),
                confidence: 0.8,
                impact: ImpactLevel::Low,
            },
            RefactorRule {
                name: "add_documentation".to_string(),
                file_extensions: vec!["rs".to_string()],
                pattern: Regex::new(r"^pub fn\s+(\w+)").unwrap(),
                replacement: "/// TODO: Add documentation\npub fn $1".to_string(),
                description: "Add documentation for public function".to_string(),
                confidence: 0.9,
                impact: ImpactLevel::Low,
            },
        ];

        self.language_rules.insert("rs".to_string(), rust_rules);

        // JavaScript/TypeScript 规则
        let js_rules = vec![
            RefactorRule {
                name: "use_const".to_string(),
                file_extensions: vec!["js".to_string(), "ts".to_string()],
                pattern: Regex::new(r"^let\s+(\w+)\s*=\s*([^;]+);$").unwrap(),
                replacement: "const $1 = $2;".to_string(),
                description: "Use const instead of let for immutable variables".to_string(),
                confidence: 0.7,
                impact: ImpactLevel::Low,
            },
        ];

        self.language_rules.insert("js".to_string(), js_rules.clone());
        self.language_rules.insert("ts".to_string(), js_rules);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactor_engine_creation() {
        let engine = RefactorEngine::new();
        assert!(!engine.get_supported_languages().is_empty());
    }

    #[test]
    fn test_suggestion_type_serialization() {
        let suggestion_type = SuggestionType::Rename {
            from: "old_name".to_string(),
            to: "new_name".to_string(),
        };
        
        let serialized = serde_json::to_string(&suggestion_type).unwrap();
        let deserialized: SuggestionType = serde_json::from_str(&serialized).unwrap();
        
        match deserialized {
            SuggestionType::Rename { from, to } => {
                assert_eq!(from, "old_name");
                assert_eq!(to, "new_name");
            }
            _ => panic!("Unexpected suggestion type"),
        }
    }
}
