//! Git集成模块
//! 
//! 实现Git操作集成，包括提交、分支管理、差异查看等

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;

use crate::error::{ClaudeError, Result};

/// Git仓库状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    /// 当前分支
    pub current_branch: String,
    /// 是否有未提交的更改
    pub has_changes: bool,
    /// 暂存的文件
    pub staged_files: Vec<String>,
    /// 未暂存的文件
    pub unstaged_files: Vec<String>,
    /// 未跟踪的文件
    pub untracked_files: Vec<String>,
    /// 远程分支状态
    pub remote_status: RemoteStatus,
}

/// 远程分支状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStatus {
    /// 领先的提交数
    pub ahead: u32,
    /// 落后的提交数
    pub behind: u32,
    /// 远程分支名
    pub remote_branch: Option<String>,
}

/// Git提交信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    /// 提交哈希
    pub hash: String,
    /// 提交消息
    pub message: String,
    /// 作者
    pub author: String,
    /// 提交时间
    pub timestamp: String,
    /// 文件变更
    pub files_changed: Vec<String>,
}

/// Git分支信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    /// 分支名
    pub name: String,
    /// 是否为当前分支
    pub is_current: bool,
    /// 是否为远程分支
    pub is_remote: bool,
    /// 最后提交哈希
    pub last_commit: Option<String>,
}

/// Git差异信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    /// 文件路径
    pub file_path: String,
    /// 差异内容
    pub diff_content: String,
    /// 添加的行数
    pub lines_added: u32,
    /// 删除的行数
    pub lines_deleted: u32,
}

/// Git管理器
pub struct GitManager {
    /// 工作目录
    working_dir: PathBuf,
}

impl GitManager {
    /// 创建新的Git管理器
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    /// 检查是否为Git仓库
    pub async fn is_git_repository(&self) -> bool {
        let output = AsyncCommand::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(&self.working_dir)
            .output()
            .await;

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// 初始化Git仓库
    pub async fn init_repository(&self) -> Result<()> {
        let output = AsyncCommand::new("git")
            .arg("init")
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to execute git init: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ClaudeError::General(format!("Git init failed: {}", error)));
        }

        Ok(())
    }

    /// 获取Git状态
    pub async fn get_status(&self) -> Result<GitStatus> {
        if !self.is_git_repository().await {
            return Err(ClaudeError::General("Not a git repository".to_string()));
        }

        let current_branch = self.get_current_branch().await?;
        let (staged_files, unstaged_files, untracked_files) = self.get_file_status().await?;
        let remote_status = self.get_remote_status().await?;

        Ok(GitStatus {
            current_branch,
            has_changes: !staged_files.is_empty() || !unstaged_files.is_empty() || !untracked_files.is_empty(),
            staged_files,
            unstaged_files,
            untracked_files,
            remote_status,
        })
    }

    /// 获取当前分支
    pub async fn get_current_branch(&self) -> Result<String> {
        let output = AsyncCommand::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to get current branch: {}", e)))?;

        if !output.status.success() {
            return Err(ClaudeError::General("Failed to get current branch".to_string()));
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(branch)
    }

    /// 获取文件状态
    async fn get_file_status(&self) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let output = AsyncCommand::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to get file status: {}", e)))?;

        if !output.status.success() {
            return Err(ClaudeError::General("Failed to get file status".to_string()));
        }

        let status_output = String::from_utf8_lossy(&output.stdout);
        let mut staged_files = Vec::new();
        let mut unstaged_files = Vec::new();
        let mut untracked_files = Vec::new();

        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }

            let status_chars = &line[0..2];
            let file_path = line[3..].to_string();

            match status_chars {
                "??" => untracked_files.push(file_path),
                _ => {
                    if status_chars.chars().nth(0).unwrap() != ' ' {
                        staged_files.push(file_path.clone());
                    }
                    if status_chars.chars().nth(1).unwrap() != ' ' {
                        unstaged_files.push(file_path);
                    }
                }
            }
        }

        Ok((staged_files, unstaged_files, untracked_files))
    }

    /// 获取远程状态
    async fn get_remote_status(&self) -> Result<RemoteStatus> {
        // 获取远程分支信息
        let output = AsyncCommand::new("git")
            .arg("status")
            .arg("--porcelain=v1")
            .arg("--branch")
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to get remote status: {}", e)))?;

        if !output.status.success() {
            return Ok(RemoteStatus {
                ahead: 0,
                behind: 0,
                remote_branch: None,
            });
        }

        let status_output = String::from_utf8_lossy(&output.stdout);
        let first_line = status_output.lines().next().unwrap_or("");

        if first_line.starts_with("## ") {
            let branch_info = &first_line[3..];
            if let Some(tracking_info) = branch_info.split("...").nth(1) {
                let remote_branch = tracking_info.split_whitespace().next().map(|s| s.to_string());
                
                // 解析ahead/behind信息
                let mut ahead = 0;
                let mut behind = 0;
                
                if let Some(bracket_start) = tracking_info.find('[') {
                    if let Some(bracket_end) = tracking_info.find(']') {
                        let tracking_part = &tracking_info[bracket_start + 1..bracket_end];
                        for part in tracking_part.split(", ") {
                            if part.starts_with("ahead ") {
                                ahead = part[6..].parse().unwrap_or(0);
                            } else if part.starts_with("behind ") {
                                behind = part[7..].parse().unwrap_or(0);
                            }
                        }
                    }
                }

                return Ok(RemoteStatus {
                    ahead,
                    behind,
                    remote_branch,
                });
            }
        }

        Ok(RemoteStatus {
            ahead: 0,
            behind: 0,
            remote_branch: None,
        })
    }

    /// 添加文件到暂存区
    pub async fn add_files(&self, files: &[String]) -> Result<()> {
        let mut cmd = AsyncCommand::new("git");
        cmd.arg("add").current_dir(&self.working_dir);
        
        for file in files {
            cmd.arg(file);
        }

        let output = cmd.output().await
            .map_err(|e| ClaudeError::General(format!("Failed to add files: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ClaudeError::General(format!("Git add failed: {}", error)));
        }

        Ok(())
    }

    /// 提交更改
    pub async fn commit(&self, message: &str) -> Result<String> {
        let output = AsyncCommand::new("git")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to commit: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ClaudeError::General(format!("Git commit failed: {}", error)));
        }

        // 获取提交哈希
        let hash_output = AsyncCommand::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to get commit hash: {}", e)))?;

        let commit_hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();
        Ok(commit_hash)
    }

    /// 获取提交历史
    pub async fn get_commit_history(&self, limit: Option<u32>) -> Result<Vec<GitCommit>> {
        let mut cmd = AsyncCommand::new("git");
        cmd.arg("log")
            .arg("--pretty=format:%H|%s|%an|%ad")
            .arg("--date=iso")
            .current_dir(&self.working_dir);

        if let Some(limit) = limit {
            cmd.arg(format!("-{}", limit));
        }

        let output = cmd.output().await
            .map_err(|e| ClaudeError::General(format!("Failed to get commit history: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let log_output = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();

        for line in log_output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                commits.push(GitCommit {
                    hash: parts[0].to_string(),
                    message: parts[1].to_string(),
                    author: parts[2].to_string(),
                    timestamp: parts[3].to_string(),
                    files_changed: Vec::new(), // 可以后续扩展
                });
            }
        }

        Ok(commits)
    }

    /// 获取分支列表
    pub async fn get_branches(&self) -> Result<Vec<GitBranch>> {
        let output = AsyncCommand::new("git")
            .arg("branch")
            .arg("-a")
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to get branches: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let branch_output = String::from_utf8_lossy(&output.stdout);
        let mut branches = Vec::new();

        for line in branch_output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let is_current = line.starts_with('*');
            let branch_name = if is_current {
                line[2..].trim()
            } else {
                line
            };

            let is_remote = branch_name.starts_with("remotes/");
            let clean_name = if is_remote {
                branch_name.strip_prefix("remotes/").unwrap_or(branch_name)
            } else {
                branch_name
            };

            branches.push(GitBranch {
                name: clean_name.to_string(),
                is_current,
                is_remote,
                last_commit: None, // 可以后续扩展
            });
        }

        Ok(branches)
    }

    /// 创建新分支
    pub async fn create_branch(&self, branch_name: &str) -> Result<()> {
        let output = AsyncCommand::new("git")
            .arg("checkout")
            .arg("-b")
            .arg(branch_name)
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to create branch: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ClaudeError::General(format!("Git branch creation failed: {}", error)));
        }

        Ok(())
    }

    /// 切换分支
    pub async fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let output = AsyncCommand::new("git")
            .arg("checkout")
            .arg(branch_name)
            .current_dir(&self.working_dir)
            .output()
            .await
            .map_err(|e| ClaudeError::General(format!("Failed to checkout branch: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ClaudeError::General(format!("Git checkout failed: {}", error)));
        }

        Ok(())
    }

    /// 获取文件差异
    pub async fn get_diff(&self, file_path: Option<&str>) -> Result<Vec<GitDiff>> {
        let mut cmd = AsyncCommand::new("git");
        cmd.arg("diff").current_dir(&self.working_dir);

        if let Some(file) = file_path {
            cmd.arg(file);
        }

        let output = cmd.output().await
            .map_err(|e| ClaudeError::General(format!("Failed to get diff: {}", e)))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let diff_output = String::from_utf8_lossy(&output.stdout);
        
        // 简化的差异解析
        let mut diffs = Vec::new();
        let mut current_file = None;
        let mut diff_content = String::new();
        let mut lines_added = 0;
        let mut lines_deleted = 0;

        for line in diff_output.lines() {
            if line.starts_with("diff --git") {
                // 保存前一个文件的差异
                if let Some(file) = current_file.take() {
                    diffs.push(GitDiff {
                        file_path: file,
                        diff_content: diff_content.clone(),
                        lines_added,
                        lines_deleted,
                    });
                    diff_content.clear();
                    lines_added = 0;
                    lines_deleted = 0;
                }

                // 解析新文件
                if let Some(file_part) = line.split_whitespace().nth(3) {
                    current_file = Some(file_part.strip_prefix("b/").unwrap_or(file_part).to_string());
                }
            } else if line.starts_with('+') && !line.starts_with("+++") {
                lines_added += 1;
                diff_content.push_str(line);
                diff_content.push('\n');
            } else if line.starts_with('-') && !line.starts_with("---") {
                lines_deleted += 1;
                diff_content.push_str(line);
                diff_content.push('\n');
            } else {
                diff_content.push_str(line);
                diff_content.push('\n');
            }
        }

        // 保存最后一个文件的差异
        if let Some(file) = current_file {
            diffs.push(GitDiff {
                file_path: file,
                diff_content,
                lines_added,
                lines_deleted,
            });
        }

        Ok(diffs)
    }
}
