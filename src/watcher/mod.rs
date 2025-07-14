//! 文件监控和自动重载模块
//! 
//! 实现文件变化监控和自动重载功能

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use walkdir::WalkDir;

use crate::error::{ClaudeError, Result};

/// 文件变化事件
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// 文件路径
    pub path: PathBuf,
    /// 事件类型
    pub event_type: FileEventType,
    /// 时间戳
    pub timestamp: Instant,
    /// 文件大小（如果可用）
    pub file_size: Option<u64>,
}

/// 文件事件类型
#[derive(Debug, Clone, PartialEq)]
pub enum FileEventType {
    /// 文件创建
    Created,
    /// 文件修改
    Modified,
    /// 文件删除
    Deleted,
    /// 文件重命名
    Renamed { from: PathBuf, to: PathBuf },
}

/// 监控配置
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// 是否递归监控子目录
    pub recursive: bool,
    /// 忽略的文件模式
    pub ignore_patterns: Vec<String>,
    /// 监控的文件扩展名
    pub watch_extensions: Option<Vec<String>>,
    /// 防抖延迟（毫秒）
    pub debounce_delay: u64,
    /// 最大监控文件数
    pub max_files: Option<usize>,
}

/// 文件监控器
pub struct FileWatcher {
    /// 内部监控器
    watcher: Option<RecommendedWatcher>,
    /// 事件发送器
    event_sender: broadcast::Sender<FileChangeEvent>,
    /// 监控的路径
    watched_paths: Arc<Mutex<HashMap<PathBuf, WatchConfig>>>,
    /// 防抖缓存
    debounce_cache: Arc<Mutex<HashMap<PathBuf, Instant>>>,
    /// 是否正在运行
    is_running: Arc<Mutex<bool>>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            recursive: true,
            ignore_patterns: vec![
                ".git".to_string(),
                ".DS_Store".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "*.tmp".to_string(),
                "*.swp".to_string(),
                "*.log".to_string(),
            ],
            watch_extensions: None,
            debounce_delay: 100,
            max_files: Some(10000),
        }
    }
}

impl FileWatcher {
    /// 创建新的文件监控器
    pub fn new() -> Result<Self> {
        let (event_sender, _) = broadcast::channel(1000);
        
        Ok(Self {
            watcher: None,
            event_sender,
            watched_paths: Arc::new(Mutex::new(HashMap::new())),
            debounce_cache: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
        })
    }

    /// 开始监控指定路径
    pub fn watch_path<P: AsRef<Path>>(&mut self, path: P, config: WatchConfig) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        
        if !path.exists() {
            return Err(ClaudeError::General(format!(
                "Path does not exist: {}", path.display()
            )));
        }

        // 检查文件数量限制
        if let Some(max_files) = config.max_files {
            let file_count = self.count_files(&path, &config)?;
            if file_count > max_files {
                return Err(ClaudeError::General(format!(
                    "Too many files to watch: {} (max: {})", file_count, max_files
                )));
            }
        }

        // 创建监控器（如果还没有）
        if self.watcher.is_none() {
            let (tx, rx) = mpsc::channel();
            let watcher = RecommendedWatcher::new(
                move |res| {
                    if let Ok(event) = res {
                        let _ = tx.send(event);
                    }
                },
                Config::default(),
            ).map_err(|e| ClaudeError::General(format!("Failed to create watcher: {}", e)))?;

            self.watcher = Some(watcher);
            self.start_event_loop(rx)?;
        }

        // 添加路径到监控列表
        if let Some(ref mut watcher) = self.watcher {
            let mode = if config.recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };

            watcher.watch(&path, mode)
                .map_err(|e| ClaudeError::General(format!("Failed to watch path: {}", e)))?;
        }

        // 保存配置
        self.watched_paths.lock().unwrap().insert(path, config);

        Ok(())
    }

    /// 停止监控指定路径
    pub fn unwatch_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(&path)
                .map_err(|e| ClaudeError::General(format!("Failed to unwatch path: {}", e)))?;
        }

        self.watched_paths.lock().unwrap().remove(&path);

        Ok(())
    }

    /// 获取事件接收器
    pub fn subscribe(&self) -> broadcast::Receiver<FileChangeEvent> {
        self.event_sender.subscribe()
    }

    /// 停止监控
    pub fn stop(&mut self) {
        *self.is_running.lock().unwrap() = false;
        self.watcher = None;
        self.watched_paths.lock().unwrap().clear();
    }

    /// 获取监控的路径列表
    pub fn get_watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().unwrap().keys().cloned().collect()
    }

    /// 启动事件循环
    fn start_event_loop(&self, rx: Receiver<Event>) -> Result<()> {
        let event_sender = self.event_sender.clone();
        let watched_paths = self.watched_paths.clone();
        let debounce_cache = self.debounce_cache.clone();
        let is_running = self.is_running.clone();

        *is_running.lock().unwrap() = true;

        thread::spawn(move || {
            while *is_running.lock().unwrap() {
                if let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                    if let Some(file_event) = Self::process_notify_event(
                        event,
                        &watched_paths,
                        &debounce_cache,
                    ) {
                        let _ = event_sender.send(file_event);
                    }
                }
            }
        });

        Ok(())
    }

    /// 处理notify事件
    fn process_notify_event(
        event: Event,
        watched_paths: &Arc<Mutex<HashMap<PathBuf, WatchConfig>>>,
        debounce_cache: &Arc<Mutex<HashMap<PathBuf, Instant>>>,
    ) -> Option<FileChangeEvent> {
        let paths = event.paths;
        if paths.is_empty() {
            return None;
        }

        let path = &paths[0];
        let now = Instant::now();

        // 检查是否应该忽略此文件
        let watched_paths_guard = watched_paths.lock().unwrap();
        let config = watched_paths_guard.values().next()?;

        if Self::should_ignore_path(path, config) {
            return None;
        }

        // 防抖处理
        {
            let mut cache = debounce_cache.lock().unwrap();
            if let Some(last_time) = cache.get(path) {
                if now.duration_since(*last_time).as_millis() < config.debounce_delay as u128 {
                    return None;
                }
            }
            cache.insert(path.clone(), now);
        }

        // 转换事件类型
        let event_type = match event.kind {
            EventKind::Create(_) => FileEventType::Created,
            EventKind::Modify(_) => FileEventType::Modified,
            EventKind::Remove(_) => FileEventType::Deleted,
            _ => return None,
        };

        // 获取文件大小
        let file_size = if path.exists() {
            std::fs::metadata(path).ok().map(|m| m.len())
        } else {
            None
        };

        Some(FileChangeEvent {
            path: path.clone(),
            event_type,
            timestamp: now,
            file_size,
        })
    }

    /// 检查是否应该忽略路径
    fn should_ignore_path(path: &Path, config: &WatchConfig) -> bool {
        let path_str = path.to_string_lossy();

        // 检查忽略模式
        for pattern in &config.ignore_patterns {
            if pattern.contains('*') {
                // 简单的通配符匹配
                let pattern_parts: Vec<&str> = pattern.split('*').collect();
                if pattern_parts.len() == 2 {
                    let prefix = pattern_parts[0];
                    let suffix = pattern_parts[1];
                    if path_str.starts_with(prefix) && path_str.ends_with(suffix) {
                        return true;
                    }
                }
            } else if path_str.contains(pattern) {
                return true;
            }
        }

        // 检查文件扩展名
        if let Some(ref extensions) = config.watch_extensions {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !extensions.contains(&ext.to_string()) {
                    return true;
                }
            }
        }

        false
    }

    /// 计算路径下的文件数量
    fn count_files(&self, path: &Path, config: &WatchConfig) -> Result<usize> {
        let mut count = 0;
        
        let walker = if config.recursive {
            WalkDir::new(path)
        } else {
            WalkDir::new(path).max_depth(1)
        };

        for entry in walker {
            let entry = entry.map_err(|e| ClaudeError::General(format!("Walk error: {}", e)))?;
            
            if entry.file_type().is_file() {
                if !Self::should_ignore_path(entry.path(), config) {
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_file_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_config_default() {
        let config = WatchConfig::default();
        assert!(config.recursive);
        assert!(!config.ignore_patterns.is_empty());
        assert_eq!(config.debounce_delay, 100);
    }

    #[tokio::test]
    async fn test_file_watching() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        let config = WatchConfig::default();

        // 开始监控
        let result = watcher.watch_path(temp_dir.path(), config);
        assert!(result.is_ok());

        // 检查监控路径
        let watched = watcher.get_watched_paths();
        assert_eq!(watched.len(), 1);
        assert_eq!(watched[0], temp_dir.path());
    }
}
