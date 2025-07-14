//! 成本跟踪和使用统计模块
//! 
//! 实现API调用成本跟踪、token使用统计和费用计算功能

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{ClaudeError, Result};

/// Claude模型定价信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// 模型名称
    pub model_name: String,
    /// 输入Token价格（每1000个token的美元价格）
    pub input_price_per_1k: f64,
    /// 输出Token价格（每1000个token的美元价格）
    pub output_price_per_1k: f64,
    /// 最大上下文长度
    pub max_context_length: u32,
}

/// API调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallRecord {
    /// 调用ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 模型名称
    pub model: String,
    /// 输入Token数
    pub input_tokens: u32,
    /// 输出Token数
    pub output_tokens: u32,
    /// 总Token数
    pub total_tokens: u32,
    /// 计算的成本（美元）
    pub cost: f64,
    /// 请求类型（chat, completion等）
    pub request_type: String,
    /// 会话ID（如果有）
    pub conversation_id: Option<String>,
}

/// 使用统计摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStatistics {
    /// 统计时间范围开始
    pub start_time: DateTime<Utc>,
    /// 统计时间范围结束
    pub end_time: DateTime<Utc>,
    /// 总API调用次数
    pub total_calls: u32,
    /// 总输入Token数
    pub total_input_tokens: u32,
    /// 总输出Token数
    pub total_output_tokens: u32,
    /// 总Token数
    pub total_tokens: u32,
    /// 总成本（美元）
    pub total_cost: f64,
    /// 按模型分组的统计
    pub by_model: HashMap<String, ModelUsage>,
    /// 按日期分组的统计
    pub by_date: HashMap<String, DailyUsage>,
}

/// 模型使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    /// 调用次数
    pub calls: u32,
    /// 输入Token数
    pub input_tokens: u32,
    /// 输出Token数
    pub output_tokens: u32,
    /// 总成本
    pub cost: f64,
}

/// 每日使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    /// 日期（YYYY-MM-DD格式）
    pub date: String,
    /// 调用次数
    pub calls: u32,
    /// 总Token数
    pub total_tokens: u32,
    /// 总成本
    pub cost: f64,
}

/// 成本跟踪管理器
pub struct CostTracker {
    /// 存储目录
    storage_dir: PathBuf,
    /// 模型定价信息
    model_pricing: HashMap<String, ModelPricing>,
    /// 调用记录缓存
    call_cache: Vec<ApiCallRecord>,
    /// 最大缓存大小
    max_cache_size: usize,
}

impl CostTracker {
    /// 创建新的成本跟踪器
    pub fn new(storage_dir: PathBuf) -> Result<Self> {
        // 确保存储目录存在
        std::fs::create_dir_all(&storage_dir)
            .map_err(|e| ClaudeError::General(format!("Failed to create storage directory: {}", e)))?;

        let mut tracker = Self {
            storage_dir,
            model_pricing: HashMap::new(),
            call_cache: Vec::new(),
            max_cache_size: 1000,
        };

        // 初始化默认定价
        tracker.initialize_default_pricing();
        
        Ok(tracker)
    }

    /// 初始化默认的模型定价
    fn initialize_default_pricing(&mut self) {
        // Claude 3.5 Sonnet 定价（2024年价格）
        self.model_pricing.insert(
            "claude-3-5-sonnet-20241022".to_string(),
            ModelPricing {
                model_name: "claude-3-5-sonnet-20241022".to_string(),
                input_price_per_1k: 0.003,  // $3.00 per 1M tokens
                output_price_per_1k: 0.015, // $15.00 per 1M tokens
                max_context_length: 200000,
            },
        );

        // Claude 3 Sonnet 定价
        self.model_pricing.insert(
            "claude-3-sonnet-20240229".to_string(),
            ModelPricing {
                model_name: "claude-3-sonnet-20240229".to_string(),
                input_price_per_1k: 0.003,  // $3.00 per 1M tokens
                output_price_per_1k: 0.015, // $15.00 per 1M tokens
                max_context_length: 200000,
            },
        );

        // Claude 3 Haiku 定价
        self.model_pricing.insert(
            "claude-3-haiku-20240307".to_string(),
            ModelPricing {
                model_name: "claude-3-haiku-20240307".to_string(),
                input_price_per_1k: 0.00025, // $0.25 per 1M tokens
                output_price_per_1k: 0.00125, // $1.25 per 1M tokens
                max_context_length: 200000,
            },
        );

        // Claude 3 Opus 定价
        self.model_pricing.insert(
            "claude-3-opus-20240229".to_string(),
            ModelPricing {
                model_name: "claude-3-opus-20240229".to_string(),
                input_price_per_1k: 0.015,  // $15.00 per 1M tokens
                output_price_per_1k: 0.075, // $75.00 per 1M tokens
                max_context_length: 200000,
            },
        );
    }

    /// 记录API调用
    pub fn record_api_call(
        &mut self,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
        request_type: &str,
        conversation_id: Option<&str>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let total_tokens = input_tokens + output_tokens;
        
        // 计算成本
        let cost = self.calculate_cost(model, input_tokens, output_tokens)?;

        let record = ApiCallRecord {
            id: id.clone(),
            timestamp: Utc::now(),
            model: model.to_string(),
            input_tokens,
            output_tokens,
            total_tokens,
            cost,
            request_type: request_type.to_string(),
            conversation_id: conversation_id.map(|s| s.to_string()),
        };

        // 添加到缓存
        self.call_cache.push(record.clone());

        // 如果缓存满了，保存到文件并清理
        if self.call_cache.len() >= self.max_cache_size {
            self.flush_cache()?;
        }

        // 保存单个记录到文件
        self.save_call_record(&record)?;

        Ok(id)
    }

    /// 计算API调用成本
    pub fn calculate_cost(&self, model: &str, input_tokens: u32, output_tokens: u32) -> Result<f64> {
        let pricing = self.model_pricing.get(model)
            .ok_or_else(|| ClaudeError::General(format!("Unknown model: {}", model)))?;

        let input_cost = (input_tokens as f64 / 1000.0) * pricing.input_price_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * pricing.output_price_per_1k;

        Ok(input_cost + output_cost)
    }

    /// 获取使用统计
    pub fn get_usage_statistics(&self, days: Option<u32>) -> Result<UsageStatistics> {
        let end_time = Utc::now();
        let start_time = match days {
            Some(d) => end_time - chrono::Duration::days(d as i64),
            None => end_time - chrono::Duration::days(30), // 默认30天
        };

        let records = self.load_records_in_range(start_time, end_time)?;
        
        let mut stats = UsageStatistics {
            start_time,
            end_time,
            total_calls: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_tokens: 0,
            total_cost: 0.0,
            by_model: HashMap::new(),
            by_date: HashMap::new(),
        };

        for record in records {
            // 更新总计
            stats.total_calls += 1;
            stats.total_input_tokens += record.input_tokens;
            stats.total_output_tokens += record.output_tokens;
            stats.total_tokens += record.total_tokens;
            stats.total_cost += record.cost;

            // 按模型统计
            let model_usage = stats.by_model.entry(record.model.clone()).or_insert(ModelUsage {
                calls: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost: 0.0,
            });
            model_usage.calls += 1;
            model_usage.input_tokens += record.input_tokens;
            model_usage.output_tokens += record.output_tokens;
            model_usage.cost += record.cost;

            // 按日期统计
            let date_str = record.timestamp.format("%Y-%m-%d").to_string();
            let daily_usage = stats.by_date.entry(date_str.clone()).or_insert(DailyUsage {
                date: date_str,
                calls: 0,
                total_tokens: 0,
                cost: 0.0,
            });
            daily_usage.calls += 1;
            daily_usage.total_tokens += record.total_tokens;
            daily_usage.cost += record.cost;
        }

        Ok(stats)
    }

    /// 获取今日使用统计
    pub fn get_today_usage(&self) -> Result<DailyUsage> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let start_time = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_time = Utc::now();

        let records = self.load_records_in_range(start_time, end_time)?;
        
        let mut usage = DailyUsage {
            date: today,
            calls: 0,
            total_tokens: 0,
            cost: 0.0,
        };

        for record in records {
            usage.calls += 1;
            usage.total_tokens += record.total_tokens;
            usage.cost += record.cost;
        }

        Ok(usage)
    }

    /// 保存调用记录到文件
    fn save_call_record(&self, record: &ApiCallRecord) -> Result<()> {
        let date_str = record.timestamp.format("%Y-%m-%d").to_string();
        let file_path = self.storage_dir.join(format!("calls_{}.jsonl", date_str));
        
        let json_line = serde_json::to_string(record)
            .map_err(|e| ClaudeError::General(format!("Failed to serialize record: {}", e)))?;
        
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| ClaudeError::General(format!("Failed to open file: {}", e)))?;
        
        writeln!(file, "{}", json_line)
            .map_err(|e| ClaudeError::General(format!("Failed to write record: {}", e)))?;

        Ok(())
    }

    /// 加载指定时间范围内的记录
    fn load_records_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ApiCallRecord>> {
        let mut records = Vec::new();
        
        // 包含缓存中的记录
        for record in &self.call_cache {
            if record.timestamp >= start && record.timestamp <= end {
                records.push(record.clone());
            }
        }

        // 从文件加载记录
        let mut current_date = start.date_naive();
        let end_date = end.date_naive();

        while current_date <= end_date {
            let date_str = current_date.format("%Y-%m-%d").to_string();
            let file_path = self.storage_dir.join(format!("calls_{}.jsonl", date_str));
            
            if file_path.exists() {
                let content = std::fs::read_to_string(&file_path)
                    .map_err(|e| ClaudeError::General(format!("Failed to read file: {}", e)))?;
                
                for line in content.lines() {
                    if let Ok(record) = serde_json::from_str::<ApiCallRecord>(line) {
                        if record.timestamp >= start && record.timestamp <= end {
                            records.push(record);
                        }
                    }
                }
            }

            current_date = current_date.succ_opt().unwrap_or(current_date);
        }

        Ok(records)
    }

    /// 刷新缓存到文件
    fn flush_cache(&mut self) -> Result<()> {
        for record in &self.call_cache {
            self.save_call_record(record)?;
        }
        self.call_cache.clear();
        Ok(())
    }

    /// 获取模型定价信息
    pub fn get_model_pricing(&self, model: &str) -> Option<&ModelPricing> {
        self.model_pricing.get(model)
    }

    /// 设置模型定价
    pub fn set_model_pricing(&mut self, pricing: ModelPricing) {
        self.model_pricing.insert(pricing.model_name.clone(), pricing);
    }
}
