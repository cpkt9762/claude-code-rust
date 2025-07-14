use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, error};

/// 性能监控器
pub struct PerformanceMonitor {
    /// 指标存储
    metrics: Arc<RwLock<MetricsStorage>>,
    /// 开始时间
    start_time: Instant,
    /// 系统开始时间
    system_start_time: SystemTime,
}

/// 指标存储
#[derive(Debug, Default)]
struct MetricsStorage {
    /// 计数器指标
    counters: HashMap<String, u64>,
    /// 直方图指标
    histograms: HashMap<String, Vec<f64>>,
    /// 仪表指标
    gauges: HashMap<String, f64>,
    /// 时间序列数据
    time_series: HashMap<String, Vec<TimeSeriesPoint>>,
}

/// 时间序列数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub value: f64,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 系统运行时间（秒）
    pub uptime_seconds: u64,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// P95 响应时间（毫秒）
    pub p95_response_time_ms: f64,
    /// P99 响应时间（毫秒）
    pub p99_response_time_ms: f64,
    /// 当前活跃连接数
    pub active_connections: u64,
    /// 内存使用量（字节）
    pub memory_usage_bytes: u64,
    /// CPU 使用率（百分比）
    pub cpu_usage_percent: f64,
    /// 每秒请求数
    pub requests_per_second: f64,
    /// 错误率（百分比）
    pub error_rate_percent: f64,
}

/// 系统资源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// 操作系统
    pub os: String,
    /// 架构
    pub arch: String,
    /// CPU 核心数
    pub cpu_cores: usize,
    /// 总内存（字节）
    pub total_memory_bytes: u64,
    /// Rust 版本
    pub rust_version: String,
    /// 应用版本
    pub app_version: String,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MetricsStorage::default())),
            start_time: Instant::now(),
            system_start_time: SystemTime::now(),
        }
    }

    /// 增加计数器
    pub async fn increment_counter(&self, name: &str, value: u64) {
        let mut metrics = self.metrics.write().await;
        *metrics.counters.entry(name.to_string()).or_insert(0) += value;
    }

    /// 记录直方图值
    pub async fn record_histogram(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.histograms.entry(name.to_string()).or_insert_with(Vec::new).push(value);
        
        // 保持最近1000个值
        let histogram = metrics.histograms.get_mut(name).unwrap();
        if histogram.len() > 1000 {
            histogram.drain(0..histogram.len() - 1000);
        }
    }

    /// 设置仪表值
    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.gauges.insert(name.to_string(), value);
    }

    /// 记录时间序列数据
    pub async fn record_time_series(&self, name: &str, value: f64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut metrics = self.metrics.write().await;
        let series = metrics.time_series.entry(name.to_string()).or_insert_with(Vec::new);
        
        series.push(TimeSeriesPoint { timestamp, value });
        
        // 保持最近24小时的数据（假设每分钟一个点）
        let cutoff_time = timestamp.saturating_sub(24 * 60 * 60);
        series.retain(|point| point.timestamp >= cutoff_time);
    }

    /// 获取计数器值
    pub async fn get_counter(&self, name: &str) -> u64 {
        let metrics = self.metrics.read().await;
        metrics.counters.get(name).copied().unwrap_or(0)
    }

    /// 获取仪表值
    pub async fn get_gauge(&self, name: &str) -> Option<f64> {
        let metrics = self.metrics.read().await;
        metrics.gauges.get(name).copied()
    }

    /// 计算百分位数
    pub async fn calculate_percentile(&self, name: &str, percentile: f64) -> Option<f64> {
        let metrics = self.metrics.read().await;
        if let Some(values) = metrics.histograms.get(name) {
            if values.is_empty() {
                return None;
            }
            
            let mut sorted_values = values.clone();
            sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            
            let index = ((percentile / 100.0) * (sorted_values.len() - 1) as f64).round() as usize;
            Some(sorted_values[index.min(sorted_values.len() - 1)])
        } else {
            None
        }
    }

    /// 计算平均值
    pub async fn calculate_average(&self, name: &str) -> Option<f64> {
        let metrics = self.metrics.read().await;
        if let Some(values) = metrics.histograms.get(name) {
            if values.is_empty() {
                None
            } else {
                Some(values.iter().sum::<f64>() / values.len() as f64)
            }
        } else {
            None
        }
    }

    /// 获取时间序列数据
    pub async fn get_time_series(&self, name: &str) -> Vec<TimeSeriesPoint> {
        let metrics = self.metrics.read().await;
        metrics.time_series.get(name).cloned().unwrap_or_default()
    }

    /// 获取完整的性能指标
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        let uptime = self.start_time.elapsed().as_secs();
        
        let total_requests = self.get_counter("requests_total").await;
        let successful_requests = self.get_counter("requests_successful").await;
        let failed_requests = self.get_counter("requests_failed").await;
        
        let avg_response_time = self.calculate_average("response_time_ms").await.unwrap_or(0.0);
        let p95_response_time = self.calculate_percentile("response_time_ms", 95.0).await.unwrap_or(0.0);
        let p99_response_time = self.calculate_percentile("response_time_ms", 99.0).await.unwrap_or(0.0);
        
        let active_connections = self.get_gauge("active_connections").await.unwrap_or(0.0) as u64;
        let memory_usage = self.get_gauge("memory_usage_bytes").await.unwrap_or(0.0) as u64;
        let cpu_usage = self.get_gauge("cpu_usage_percent").await.unwrap_or(0.0);
        
        let requests_per_second = if uptime > 0 {
            total_requests as f64 / uptime as f64
        } else {
            0.0
        };
        
        let error_rate = if total_requests > 0 {
            (failed_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        PerformanceMetrics {
            uptime_seconds: uptime,
            total_requests,
            successful_requests,
            failed_requests,
            avg_response_time_ms: avg_response_time,
            p95_response_time_ms: p95_response_time,
            p99_response_time_ms: p99_response_time,
            active_connections,
            memory_usage_bytes: memory_usage,
            cpu_usage_percent: cpu_usage,
            requests_per_second,
            error_rate_percent: error_rate,
        }
    }

    /// 获取系统信息
    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_cores: num_cpus::get(),
            total_memory_bytes: self.get_total_memory(),
            rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 获取总内存（简化实现）
    fn get_total_memory(&self) -> u64 {
        // 这里应该使用系统调用获取真实内存信息
        // 为了演示，返回一个估计值
        8 * 1024 * 1024 * 1024 // 8GB
    }

    /// 开始性能监控任务
    pub async fn start_monitoring_task(&self) -> Result<()> {
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // 更新系统指标
                if let Err(e) = Self::update_system_metrics(&metrics).await {
                    error!("Failed to update system metrics: {}", e);
                }
            }
        });
        
        info!("Performance monitoring task started");
        Ok(())
    }

    /// 更新系统指标
    async fn update_system_metrics(metrics: &Arc<RwLock<MetricsStorage>>) -> Result<()> {
        // 获取内存使用情况
        let memory_usage = Self::get_memory_usage()?;
        
        // 获取 CPU 使用情况
        let cpu_usage = Self::get_cpu_usage()?;
        
        // 更新指标
        {
            let mut storage = metrics.write().await;
            storage.gauges.insert("memory_usage_bytes".to_string(), memory_usage as f64);
            storage.gauges.insert("cpu_usage_percent".to_string(), cpu_usage);
        }
        
        Ok(())
    }

    /// 获取内存使用情况（简化实现）
    fn get_memory_usage() -> Result<u64> {
        // 这里应该使用系统调用获取真实内存使用情况
        // 为了演示，返回一个模拟值
        Ok(8 * 1024 * 1024) // 8MB
    }

    /// 获取 CPU 使用情况（简化实现）
    fn get_cpu_usage() -> Result<f64> {
        // 这里应该使用系统调用获取真实 CPU 使用情况
        // 为了演示，返回一个模拟值
        Ok(2.5) // 2.5%
    }

    /// 记录请求开始
    pub async fn record_request_start(&self) {
        self.increment_counter("requests_total", 1).await;
        self.increment_counter("active_requests", 1).await;
    }

    /// 记录请求完成
    pub async fn record_request_end(&self, duration: Duration, success: bool) {
        let duration_ms = duration.as_millis() as f64;
        
        self.record_histogram("response_time_ms", duration_ms).await;
        self.increment_counter("active_requests", u64::MAX).await; // 减1
        
        if success {
            self.increment_counter("requests_successful", 1).await;
        } else {
            self.increment_counter("requests_failed", 1).await;
        }
        
        // 记录时间序列数据
        self.record_time_series("response_time", duration_ms).await;
    }

    /// 导出指标为 Prometheus 格式
    pub async fn export_prometheus_metrics(&self) -> String {
        let metrics = self.get_performance_metrics().await;
        
        format!(
            "# HELP claude_code_requests_total Total number of requests\n\
             # TYPE claude_code_requests_total counter\n\
             claude_code_requests_total {}\n\
             \n\
             # HELP claude_code_requests_successful_total Total number of successful requests\n\
             # TYPE claude_code_requests_successful_total counter\n\
             claude_code_requests_successful_total {}\n\
             \n\
             # HELP claude_code_response_time_seconds Response time in seconds\n\
             # TYPE claude_code_response_time_seconds histogram\n\
             claude_code_response_time_seconds_sum {}\n\
             claude_code_response_time_seconds_count {}\n\
             \n\
             # HELP claude_code_active_connections Current number of active connections\n\
             # TYPE claude_code_active_connections gauge\n\
             claude_code_active_connections {}\n\
             \n\
             # HELP claude_code_memory_usage_bytes Memory usage in bytes\n\
             # TYPE claude_code_memory_usage_bytes gauge\n\
             claude_code_memory_usage_bytes {}\n",
            metrics.total_requests,
            metrics.successful_requests,
            metrics.avg_response_time_ms / 1000.0,
            metrics.total_requests,
            metrics.active_connections,
            metrics.memory_usage_bytes
        )
    }
}
