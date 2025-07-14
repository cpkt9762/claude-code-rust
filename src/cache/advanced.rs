use crate::error::{ClaudeError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, warn, error};

/// 高级缓存管理器
pub struct AdvancedCacheManager {
    /// 内存缓存
    memory_cache: Arc<MemoryCache>,
    /// 持久化缓存
    persistent_cache: Option<Arc<dyn PersistentCache>>,
    /// 分布式缓存
    distributed_cache: Option<Arc<dyn DistributedCache>>,
    /// 缓存策略
    strategy: CacheStrategy,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// 值
    pub value: T,
    /// 创建时间
    pub created_at: Instant,
    /// 最后访问时间
    pub last_accessed: Instant,
    /// 访问次数
    pub access_count: u64,
    /// 过期时间
    pub expires_at: Option<Instant>,
    /// 优先级
    pub priority: CachePriority,
    /// 标签
    pub tags: Vec<String>,
    /// 大小（字节）
    pub size: usize,
}

/// 缓存优先级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CachePriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// 缓存策略
#[derive(Debug, Clone)]
pub struct CacheStrategy {
    /// 最大内存使用（字节）
    pub max_memory_bytes: usize,
    /// 默认 TTL
    pub default_ttl: Duration,
    /// 淘汰策略
    pub eviction_policy: EvictionPolicy,
    /// 压缩阈值
    pub compression_threshold: usize,
    /// 预热策略
    pub warmup_strategy: WarmupStrategy,
}

/// 淘汰策略
#[derive(Debug, Clone)]
pub enum EvictionPolicy {
    /// 最近最少使用
    LRU,
    /// 最不经常使用
    LFU,
    /// 先进先出
    FIFO,
    /// 基于 TTL
    TTL,
    /// 自定义策略
    Custom(String),
}

/// 预热策略
#[derive(Debug, Clone)]
pub struct WarmupStrategy {
    /// 是否启用预热
    pub enabled: bool,
    /// 预热数据源
    pub data_sources: Vec<String>,
    /// 预热优先级
    pub priority_keys: Vec<String>,
}

/// 缓存统计
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 写入次数
    pub writes: u64,
    /// 删除次数
    pub deletes: u64,
    /// 淘汰次数
    pub evictions: u64,
    /// 总内存使用
    pub memory_usage_bytes: usize,
    /// 条目数量
    pub entry_count: usize,
    /// 平均访问时间（微秒）
    pub avg_access_time_us: f64,
}

/// 内存缓存
pub struct MemoryCache {
    /// 缓存数据
    data: Arc<RwLock<HashMap<String, CacheEntry<Vec<u8>>>>>,
    /// 访问顺序（用于 LRU）
    access_order: Arc<Mutex<Vec<String>>>,
    /// 当前内存使用
    memory_usage: Arc<RwLock<usize>>,
    /// 配置
    config: CacheStrategy,
}

/// 持久化缓存 trait
#[async_trait]
pub trait PersistentCache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn clear(&self) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn get_stats(&self) -> Result<CacheStats>;
}

/// 分布式缓存 trait
#[async_trait]
pub trait DistributedCache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn invalidate_pattern(&self, pattern: &str) -> Result<u64>;
    async fn get_cluster_stats(&self) -> Result<HashMap<String, CacheStats>>;
}

/// 文件系统持久化缓存
pub struct FileSystemCache {
    /// 缓存目录
    cache_dir: std::path::PathBuf,
    /// 最大文件大小
    max_file_size: usize,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

/// Redis 分布式缓存
pub struct RedisCache {
    /// Redis 客户端
    client: Arc<Mutex<Option<String>>>, // 简化为连接字符串
    /// 连接池
    pool_size: usize,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

impl AdvancedCacheManager {
    /// 创建新的高级缓存管理器
    pub fn new(strategy: CacheStrategy) -> Self {
        let memory_cache = Arc::new(MemoryCache::new(strategy.clone()));
        
        Self {
            memory_cache,
            persistent_cache: None,
            distributed_cache: None,
            strategy,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// 设置持久化缓存
    pub fn with_persistent_cache(mut self, cache: Arc<dyn PersistentCache>) -> Self {
        self.persistent_cache = Some(cache);
        self
    }

    /// 设置分布式缓存
    pub fn with_distributed_cache(mut self, cache: Arc<dyn DistributedCache>) -> Self {
        self.distributed_cache = Some(cache);
        self
    }

    /// 获取缓存值
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let start_time = Instant::now();
        
        // 首先尝试内存缓存
        if let Some(value) = self.memory_cache.get(key).await? {
            self.update_stats(true, start_time).await;
            let deserialized: T = bincode::deserialize(&value)
                .map_err(|e| ClaudeError::config_error(&format!("Deserialization failed: {}", e)))?;
            return Ok(Some(deserialized));
        }

        // 尝试持久化缓存
        if let Some(persistent) = &self.persistent_cache {
            if let Some(value) = persistent.get(key).await? {
                // 回填到内存缓存
                self.memory_cache.set(key, &value, None).await?;
                self.update_stats(true, start_time).await;
                let deserialized: T = bincode::deserialize(&value)
                    .map_err(|e| ClaudeError::validation_error("cache", &format!("Deserialization failed: {}", e)))?;
                return Ok(Some(deserialized));
            }
        }

        // 尝试分布式缓存
        if let Some(distributed) = &self.distributed_cache {
            if let Some(value) = distributed.get(key).await? {
                // 回填到内存和持久化缓存
                self.memory_cache.set(key, &value, None).await?;
                if let Some(persistent) = &self.persistent_cache {
                    let _ = persistent.set(key, &value, Some(self.strategy.default_ttl)).await;
                }
                self.update_stats(true, start_time).await;
                let deserialized: T = bincode::deserialize(&value)
                    .map_err(|e| ClaudeError::validation_error("cache", &format!("Deserialization failed: {}", e)))?;
                return Ok(Some(deserialized));
            }
        }

        self.update_stats(false, start_time).await;
        Ok(None)
    }

    /// 设置缓存值
    pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = bincode::serialize(value)
            .map_err(|e| ClaudeError::validation_error("cache", &format!("Serialization failed: {}", e)))?;

        // 设置到所有缓存层
        self.memory_cache.set(key, &serialized, ttl).await?;

        if let Some(persistent) = &self.persistent_cache {
            let _ = persistent.set(key, &serialized, ttl.or(Some(self.strategy.default_ttl))).await;
        }

        if let Some(distributed) = &self.distributed_cache {
            let _ = distributed.set(key, &serialized, ttl.or(Some(self.strategy.default_ttl))).await;
        }

        // 更新统计
        let mut stats = self.stats.write().await;
        stats.writes += 1;

        Ok(())
    }

    /// 删除缓存值
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut deleted = false;

        // 从所有缓存层删除
        if self.memory_cache.delete(key).await? {
            deleted = true;
        }

        if let Some(persistent) = &self.persistent_cache {
            if persistent.delete(key).await? {
                deleted = true;
            }
        }

        if let Some(distributed) = &self.distributed_cache {
            if distributed.delete(key).await? {
                deleted = true;
            }
        }

        if deleted {
            let mut stats = self.stats.write().await;
            stats.deletes += 1;
        }

        Ok(deleted)
    }

    /// 清空缓存
    pub async fn clear(&self) -> Result<()> {
        self.memory_cache.clear().await?;

        if let Some(persistent) = &self.persistent_cache {
            let _ = persistent.clear().await;
        }

        // 注意：通常不清空分布式缓存，因为可能被其他实例使用

        Ok(())
    }

    /// 获取缓存统计
    pub async fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.read().await.clone();
        
        // 合并内存缓存统计
        let memory_stats = self.memory_cache.get_stats().await;
        stats.memory_usage_bytes = memory_stats.memory_usage_bytes;
        stats.entry_count = memory_stats.entry_count;

        stats
    }

    /// 预热缓存
    pub async fn warmup(&self) -> Result<()> {
        if !self.strategy.warmup_strategy.enabled {
            return Ok(());
        }

        info!("Starting cache warmup...");

        // 预热优先级键
        for key in &self.strategy.warmup_strategy.priority_keys {
            // 这里应该从数据源加载数据
            debug!("Warming up key: {}", key);
        }

        info!("Cache warmup completed");
        Ok(())
    }

    /// 更新统计信息
    async fn update_stats(&self, hit: bool, start_time: Instant) {
        let mut stats = self.stats.write().await;
        
        if hit {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }

        let access_time = start_time.elapsed().as_micros() as f64;
        let total_accesses = stats.hits + stats.misses;
        stats.avg_access_time_us = (stats.avg_access_time_us * (total_accesses - 1) as f64 + access_time) / total_accesses as f64;
    }
}

impl MemoryCache {
    pub fn new(config: CacheStrategy) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(Mutex::new(Vec::new())),
            memory_usage: Arc::new(RwLock::new(0)),
            config,
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut data = self.data.write().await;
        
        if let Some(entry) = data.get_mut(key) {
            // 检查是否过期
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    data.remove(key);
                    return Ok(None);
                }
            }

            // 更新访问信息
            entry.last_accessed = Instant::now();
            entry.access_count += 1;

            // 更新访问顺序（LRU）
            let mut access_order = self.access_order.lock().await;
            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }
            access_order.push(key.to_string());

            Ok(Some(entry.value.clone()))
        } else {
            Ok(None)
        }
    }

    pub async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()> {
        let expires_at = ttl.map(|duration| Instant::now() + duration);
        let size = value.len();

        let entry = CacheEntry {
            value: value.to_vec(),
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            expires_at,
            priority: CachePriority::Normal,
            tags: Vec::new(),
            size,
        };

        // 检查内存限制
        let current_usage = *self.memory_usage.read().await;
        if current_usage + size > self.config.max_memory_bytes {
            self.evict_entries(size).await?;
        }

        let mut data = self.data.write().await;
        let old_size = data.get(key).map(|e| e.size).unwrap_or(0);
        
        data.insert(key.to_string(), entry);

        // 更新内存使用
        let mut memory_usage = self.memory_usage.write().await;
        *memory_usage = *memory_usage - old_size + size;

        // 更新访问顺序
        let mut access_order = self.access_order.lock().await;
        if let Some(pos) = access_order.iter().position(|k| k == key) {
            access_order.remove(pos);
        }
        access_order.push(key.to_string());

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut data = self.data.write().await;
        
        if let Some(entry) = data.remove(key) {
            // 更新内存使用
            let mut memory_usage = self.memory_usage.write().await;
            *memory_usage -= entry.size;

            // 更新访问顺序
            let mut access_order = self.access_order.lock().await;
            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn clear(&self) -> Result<()> {
        let mut data = self.data.write().await;
        data.clear();

        let mut memory_usage = self.memory_usage.write().await;
        *memory_usage = 0;

        let mut access_order = self.access_order.lock().await;
        access_order.clear();

        Ok(())
    }

    pub async fn get_stats(&self) -> CacheStats {
        let data = self.data.read().await;
        let memory_usage = *self.memory_usage.read().await;

        CacheStats {
            memory_usage_bytes: memory_usage,
            entry_count: data.len(),
            ..Default::default()
        }
    }

    /// 淘汰条目以释放内存
    async fn evict_entries(&self, needed_space: usize) -> Result<()> {
        let mut freed_space = 0;
        let mut keys_to_remove = Vec::new();

        match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                let access_order = self.access_order.lock().await;
                let data = self.data.read().await;

                for key in access_order.iter() {
                    if let Some(entry) = data.get(key) {
                        keys_to_remove.push(key.clone());
                        freed_space += entry.size;
                        
                        if freed_space >= needed_space {
                            break;
                        }
                    }
                }
            }
            EvictionPolicy::TTL => {
                let data = self.data.read().await;
                let now = Instant::now();

                for (key, entry) in data.iter() {
                    if let Some(expires_at) = entry.expires_at {
                        if now > expires_at {
                            keys_to_remove.push(key.clone());
                            freed_space += entry.size;
                        }
                    }
                }
            }
            _ => {
                // 其他淘汰策略的实现
                warn!("Eviction policy not fully implemented: {:?}", self.config.eviction_policy);
            }
        }

        // 删除选中的条目
        drop(self.data.read().await); // 释放读锁
        for key in keys_to_remove {
            self.delete(&key).await?;
        }

        Ok(())
    }
}

impl FileSystemCache {
    pub fn new(cache_dir: std::path::PathBuf, max_file_size: usize) -> Self {
        Self {
            cache_dir,
            max_file_size,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    fn get_file_path(&self, key: &str) -> std::path::PathBuf {
        let hash = format!("{:x}", md5::compute(key));
        self.cache_dir.join(format!("{}.cache", hash))
    }
}

#[async_trait]
impl PersistentCache for FileSystemCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let file_path = self.get_file_path(key);
        
        if file_path.exists() {
            let data = tokio::fs::read(&file_path).await
                .map_err(|e| ClaudeError::fs_error(format!("Failed to read cache file: {}", e)))?;
            
            let mut stats = self.stats.write().await;
            stats.hits += 1;
            
            Ok(Some(data))
        } else {
            let mut stats = self.stats.write().await;
            stats.misses += 1;
            
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: &[u8], _ttl: Option<Duration>) -> Result<()> {
        if value.len() > self.max_file_size {
            return Err(ClaudeError::validation_error("cache", "Value too large for file cache"));
        }

        let file_path = self.get_file_path(key);
        
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| ClaudeError::fs_error(format!("Failed to create cache directory: {}", e)))?;
        }

        tokio::fs::write(&file_path, value).await
            .map_err(|e| ClaudeError::fs_error(format!("Failed to write cache file: {}", e)))?;

        let mut stats = self.stats.write().await;
        stats.writes += 1;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let file_path = self.get_file_path(key);
        
        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await
                .map_err(|e| ClaudeError::fs_error(format!("Failed to delete cache file: {}", e)))?;
            
            let mut stats = self.stats.write().await;
            stats.deletes += 1;
            
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            tokio::fs::remove_dir_all(&self.cache_dir).await
                .map_err(|e| ClaudeError::fs_error(format!("Failed to clear cache directory: {}", e)))?;

            tokio::fs::create_dir_all(&self.cache_dir).await
                .map_err(|e| ClaudeError::fs_error(format!("Failed to recreate cache directory: {}", e)))?;
        }

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let file_path = self.get_file_path(key);
        Ok(file_path.exists())
    }

    async fn get_stats(&self) -> Result<CacheStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}
