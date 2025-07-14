//! 文件系统操作模块
//! 
//! 提供文件读写、目录管理、路径处理等核心文件操作功能

use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};

use crate::error::{ClaudeError, Result};

/// 文件编辑操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    /// 文件路径
    pub file_path: String,
    /// 编辑类型
    pub edit_type: EditType,
    /// 编辑内容
    pub content: String,
    /// 行号范围（可选）
    pub line_range: Option<(usize, usize)>,
}

/// 编辑类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditType {
    /// 替换整个文件
    Replace,
    /// 在指定位置插入
    Insert { line: usize },
    /// 删除指定行范围
    Delete { start: usize, end: usize },
    /// 替换指定行范围
    ReplaceRange { start: usize, end: usize },
    /// 在文件末尾追加
    Append,
}

/// 文件管理器（简化版，用于 CLI）
pub struct FileManager {
    /// 基础文件系统管理器
    fs_manager: FileSystemManager,
}

impl FileManager {
    /// 创建新的文件管理器
    pub fn new() -> Self {
        Self {
            fs_manager: FileSystemManager::new(vec![PathBuf::from(".")]),
        }
    }

    /// 应用编辑到文件
    pub async fn apply_edit_to_file(&self, file_path: &str, edit: &Edit) -> Result<()> {
        info!("Applying edit to file: {}", file_path);
        debug!("Edit: {:?}", edit);

        // 创建备份
        let backup_path = self.create_file_backup(file_path).await?;
        info!("Created backup: {}", backup_path);

        // 读取原文件内容
        let original_content = match self.fs_manager.read_file(Path::new(file_path)).await {
            Ok(content) => content,
            Err(_) => {
                // 如果文件不存在，创建新文件
                if matches!(edit.edit_type, EditType::Replace) {
                    String::new()
                } else {
                    return Err(ClaudeError::fs_error(format!("File not found: {}", file_path)));
                }
            }
        };

        // 应用编辑
        let new_content = match &edit.edit_type {
            EditType::Replace => edit.content.clone(),
            EditType::Insert { line } => {
                self.insert_at_line(&original_content, *line, &edit.content)?
            },
            EditType::Delete { start, end } => {
                self.delete_lines(&original_content, *start, *end)?
            },
            EditType::ReplaceRange { start, end } => {
                self.replace_lines(&original_content, *start, *end, &edit.content)?
            },
            EditType::Append => {
                format!("{}\n{}", original_content, edit.content)
            },
        };

        // 写入新内容
        self.fs_manager.write_file(Path::new(file_path), &new_content).await?;

        // 验证语法（如果是代码文件）
        if let Err(e) = self.validate_syntax(file_path).await {
            warn!("Syntax validation failed: {}", e);
            // 恢复备份
            self.restore_from_backup(file_path, &backup_path).await?;
            return Err(crate::error::ClaudeError::fs_error(format!("Syntax error after edit: {}", e)));
        }

        info!("Edit applied successfully to: {}", file_path);
        Ok(())
    }

    /// 创建文件备份
    pub async fn create_file_backup(&self, file_path: &str) -> Result<String> {
        let backup_path = format!("{}.backup.{}", file_path, chrono::Utc::now().timestamp());

        if Path::new(file_path).exists() {
            self.fs_manager.copy_file(Path::new(file_path), Path::new(&backup_path)).await?;
        }

        Ok(backup_path)
    }

    /// 验证文件语法
    pub async fn validate_syntax(&self, file_path: &str) -> Result<bool> {
        let path = Path::new(file_path);
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension {
            "rs" => self.validate_rust_syntax(file_path).await,
            "py" => self.validate_python_syntax(file_path).await,
            "js" | "ts" => self.validate_javascript_syntax(file_path).await,
            "json" => self.validate_json_syntax(file_path).await,
            _ => {
                debug!("No syntax validation available for extension: {}", extension);
                Ok(true) // 默认认为有效
            }
        }
    }

    /// 检查文件权限
    pub async fn check_permissions(&self, path: &str) -> Result<()> {
        let path = Path::new(path);

        if !path.exists() {
            return Err(crate::error::ClaudeError::fs_error(format!("Path does not exist: {}", path.display())));
        }

        // 检查读权限
        if let Err(e) = fs::metadata(path).await {
            return Err(crate::error::ClaudeError::fs_error(format!("Cannot read metadata: {}", e)));
        }

        // 检查写权限（如果是文件）
        if path.is_file() {
            let test_content = "test";
            let temp_path = format!("{}.temp", path.display());

            match fs::write(&temp_path, test_content).await {
                Ok(_) => {
                    let _ = fs::remove_file(&temp_path).await;
                },
                Err(e) => {
                    return Err(crate::error::ClaudeError::fs_error(format!("No write permission: {}", e)));
                }
            }
        }

        Ok(())
    }

    /// 读取图像文件
    pub async fn read_image(&self, path: &str) -> Result<Vec<u8>> {
        self.fs_manager.read_file_bytes(Path::new(path)).await
    }

    /// 检查文件是否存在
    pub async fn exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    /// 创建目录
    pub async fn create_dir(&self, path: &str) -> Result<()> {
        self.fs_manager.create_dir(Path::new(path)).await
    }

    /// 写入 JSON 文件
    pub async fn write_json<T: Serialize>(&self, path: &str, data: &T) -> Result<()> {
        let json_content = serde_json::to_string_pretty(data)?;
        self.fs_manager.write_file(Path::new(path), &json_content).await
    }

    /// 在指定行插入内容
    fn insert_at_line(&self, content: &str, line: usize, new_content: &str) -> Result<String> {
        let mut lines: Vec<&str> = content.lines().collect();

        if line > lines.len() {
            return Err(crate::error::ClaudeError::fs_error(format!("Line {} is beyond file length {}", line, lines.len())));
        }

        lines.insert(line, new_content);
        Ok(lines.join("\n"))
    }

    /// 删除指定行范围
    fn delete_lines(&self, content: &str, start: usize, end: usize) -> Result<String> {
        let mut lines: Vec<&str> = content.lines().collect();

        if start >= lines.len() || end >= lines.len() || start > end {
            return Err(crate::error::ClaudeError::fs_error("Invalid line range for deletion".to_string()));
        }

        lines.drain(start..=end);
        Ok(lines.join("\n"))
    }

    /// 替换指定行范围
    fn replace_lines(&self, content: &str, start: usize, end: usize, new_content: &str) -> Result<String> {
        let mut lines: Vec<&str> = content.lines().collect();

        if start >= lines.len() || end >= lines.len() || start > end {
            return Err(crate::error::ClaudeError::fs_error("Invalid line range for replacement".to_string()));
        }

        // 删除旧行
        lines.drain(start..=end);

        // 插入新内容
        for (i, line) in new_content.lines().enumerate() {
            lines.insert(start + i, line);
        }

        Ok(lines.join("\n"))
    }

    /// 从备份恢复文件
    async fn restore_from_backup(&self, file_path: &str, backup_path: &str) -> Result<()> {
        self.fs_manager.copy_file(Path::new(backup_path), Path::new(file_path)).await?;
        info!("Restored file from backup: {} -> {}", backup_path, file_path);
        Ok(())
    }

    /// 验证 Rust 语法
    async fn validate_rust_syntax(&self, _file_path: &str) -> Result<bool> {
        // 这里可以集成 rustc 或 rust-analyzer 进行语法检查
        // 暂时返回 true
        Ok(true)
    }

    /// 验证 Python 语法
    async fn validate_python_syntax(&self, _file_path: &str) -> Result<bool> {
        // 这里可以集成 Python AST 解析器
        // 暂时返回 true
        Ok(true)
    }

    /// 验证 JavaScript 语法
    async fn validate_javascript_syntax(&self, _file_path: &str) -> Result<bool> {
        // 这里可以集成 JavaScript 解析器
        // 暂时返回 true
        Ok(true)
    }

    /// 验证 JSON 语法
    async fn validate_json_syntax(&self, file_path: &str) -> Result<bool> {
        let content = self.fs_manager.read_file(Path::new(file_path)).await?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("JSON syntax error: {}", e);
                Ok(false)
            }
        }
    }
}

/// 文件系统管理器
pub struct FileSystemManager {
    /// 工作目录列表
    working_dirs: Vec<PathBuf>,
}

impl FileSystemManager {
    /// 创建新的文件系统管理器
    pub fn new(working_dirs: Vec<PathBuf>) -> Self {
        Self { working_dirs }
    }

    /// 添加工作目录
    pub fn add_working_dir(&mut self, dir: PathBuf) {
        if !self.working_dirs.contains(&dir) {
            self.working_dirs.push(dir);
        }
    }

    /// 获取工作目录列表
    pub fn get_working_dirs(&self) -> &[PathBuf] {
        &self.working_dirs
    }

    /// 读取文件内容
    pub async fn read_file(&self, path: &Path) -> Result<String> {
        let resolved_path = self.resolve_path(path)?;
        
        if !resolved_path.exists() {
            return Err(ClaudeError::fs_error(format!("File not found: {}", path.display())));
        }

        let content = fs::read_to_string(&resolved_path).await?;
        Ok(content)
    }

    /// 读取文件为字节数组
    pub async fn read_file_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let resolved_path = self.resolve_path(path)?;
        
        if !resolved_path.exists() {
            return Err(ClaudeError::fs_error(format!("File not found: {}", path.display())));
        }

        let content = fs::read(&resolved_path).await?;
        Ok(content)
    }

    /// 写入文件内容
    pub async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        let resolved_path = self.resolve_path(path)?;
        
        // 确保父目录存在
        if let Some(parent) = resolved_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&resolved_path, content).await?;
        Ok(())
    }

    /// 写入字节数组到文件
    pub async fn write_file_bytes(&self, path: &Path, content: &[u8]) -> Result<()> {
        let resolved_path = self.resolve_path(path)?;
        
        // 确保父目录存在
        if let Some(parent) = resolved_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&resolved_path, content).await?;
        Ok(())
    }

    /// 追加内容到文件
    pub async fn append_file(&self, path: &Path, content: &str) -> Result<()> {
        let resolved_path = self.resolve_path(path)?;
        
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&resolved_path)
            .await?;
        
        file.write_all(content.as_bytes()).await?;
        Ok(())
    }

    /// 检查文件是否存在
    pub async fn file_exists(&self, path: &Path) -> bool {
        if let Ok(resolved_path) = self.resolve_path(path) {
            resolved_path.exists()
        } else {
            false
        }
    }

    /// 检查是否为目录
    pub async fn is_directory(&self, path: &Path) -> bool {
        if let Ok(resolved_path) = self.resolve_path(path) {
            resolved_path.is_dir()
        } else {
            false
        }
    }

    /// 创建目录
    pub async fn create_dir(&self, path: &Path) -> Result<()> {
        let resolved_path = self.resolve_path(path)?;
        fs::create_dir_all(&resolved_path).await?;
        Ok(())
    }

    /// 删除文件
    pub async fn delete_file(&self, path: &Path) -> Result<()> {
        let resolved_path = self.resolve_path(path)?;
        
        if !resolved_path.exists() {
            return Err(ClaudeError::fs_error(format!("File not found: {}", path.display())));
        }

        if resolved_path.is_dir() {
            fs::remove_dir_all(&resolved_path).await?;
        } else {
            fs::remove_file(&resolved_path).await?;
        }
        
        Ok(())
    }

    /// 复制文件
    pub async fn copy_file(&self, from: &Path, to: &Path) -> Result<()> {
        let from_path = self.resolve_path(from)?;
        let to_path = self.resolve_path(to)?;
        
        if !from_path.exists() {
            return Err(ClaudeError::fs_error(format!("Source file not found: {}", from.display())));
        }

        // 确保目标目录存在
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::copy(&from_path, &to_path).await?;
        Ok(())
    }

    /// 移动文件
    pub async fn move_file(&self, from: &Path, to: &Path) -> Result<()> {
        let from_path = self.resolve_path(from)?;
        let to_path = self.resolve_path(to)?;
        
        if !from_path.exists() {
            return Err(ClaudeError::fs_error(format!("Source file not found: {}", from.display())));
        }

        // 确保目标目录存在
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::rename(&from_path, &to_path).await?;
        Ok(())
    }

    /// 列出目录内容
    pub async fn list_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let resolved_path = self.resolve_path(path)?;
        
        if !resolved_path.exists() {
            return Err(ClaudeError::fs_error(format!("Directory not found: {}", path.display())));
        }

        if !resolved_path.is_dir() {
            return Err(ClaudeError::fs_error(format!("Path is not a directory: {}", path.display())));
        }

        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&resolved_path).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry.path());
        }
        
        entries.sort();
        Ok(entries)
    }

    /// 获取文件元数据
    pub async fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let resolved_path = self.resolve_path(path)?;
        
        if !resolved_path.exists() {
            return Err(ClaudeError::fs_error(format!("File not found: {}", path.display())));
        }

        let metadata = fs::metadata(&resolved_path).await?;
        
        Ok(FileMetadata {
            size: metadata.len(),
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
        })
    }

    /// 解析路径（相对于工作目录）
    fn resolve_path(&self, path: &Path) -> Result<PathBuf> {
        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }

        // 尝试在工作目录中查找文件
        for working_dir in &self.working_dirs {
            let full_path = working_dir.join(path);
            if full_path.exists() {
                return Ok(full_path);
            }
        }

        // 如果没有找到，使用第一个工作目录
        if let Some(first_dir) = self.working_dirs.first() {
            Ok(first_dir.join(path))
        } else {
            Ok(std::env::current_dir()?.join(path))
        }
    }

    /// 搜索文件
    pub async fn search_files(&self, pattern: &str, extensions: Option<&[&str]>) -> Result<Vec<PathBuf>> {
        let mut results = Vec::new();
        
        for working_dir in &self.working_dirs {
            let found = self.search_in_directory(working_dir, pattern, extensions).await?;
            results.extend(found);
        }
        
        results.sort();
        results.dedup();
        Ok(results)
    }

    /// 在指定目录中搜索文件
    async fn search_in_directory(&self, dir: &Path, pattern: &str, extensions: Option<&[&str]>) -> Result<Vec<PathBuf>> {
        let mut results = Vec::new();
        
        if !dir.exists() || !dir.is_dir() {
            return Ok(results);
        }

        let mut stack = vec![dir.to_path_buf()];
        
        while let Some(current_dir) = stack.pop() {
            let mut entries = fs::read_dir(&current_dir).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    let file_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    // 检查文件名是否匹配模式
                    if file_name.contains(pattern) {
                        // 检查扩展名
                        if let Some(exts) = extensions {
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                if exts.contains(&ext) {
                                    results.push(path);
                                }
                            }
                        } else {
                            results.push(path);
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
}

/// 文件元数据
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub modified: Option<std::time::SystemTime>,
    pub created: Option<std::time::SystemTime>,
}
