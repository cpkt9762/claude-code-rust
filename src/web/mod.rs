use crate::error::{ClaudeError, Result};
use crate::config::ClaudeConfig;
use crate::network::ClaudeApiClient;

pub mod advanced;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json, Sse, sse::Event},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
    services::ServeDir,
};
use futures::stream::{self, Stream};
use futures::StreamExt;
use std::convert::Infallible;

/// Web 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// 服务器端口
    pub port: u16,
    /// 绑定地址
    pub host: String,
    /// 是否启用 CORS
    pub enable_cors: bool,
    /// 静态文件目录
    pub static_dir: Option<String>,
    /// 是否启用压缩
    pub enable_compression: bool,
    /// 请求超时时间（秒）
    pub request_timeout: u64,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "127.0.0.1".to_string(),
            enable_cors: true,
            static_dir: Some("web/static".to_string()),
            enable_compression: true,
            request_timeout: 30,
        }
    }
}

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    /// Claude API 客户端
    pub claude_client: Arc<ClaudeApiClient>,
    /// 配置
    pub config: Arc<RwLock<ClaudeConfig>>,
    /// 活跃连接计数
    pub active_connections: Arc<RwLock<u64>>,
    /// 请求统计
    pub request_stats: Arc<RwLock<RequestStats>>,
}

/// 请求统计
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
}

/// API 请求
#[derive(Debug, Deserialize)]
pub struct ApiRequest {
    pub message: String,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

/// API 响应
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub response: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
    pub processing_time_ms: u64,
}

/// Token 使用统计
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct ChatQuery {
    pub stream: Option<bool>,
}

/// Web 服务器
pub struct WebServer {
    config: WebConfig,
    app_state: AppState,
}

impl WebServer {
    /// 创建新的 Web 服务器
    pub fn new(config: WebConfig, claude_config: ClaudeConfig) -> Result<Self> {
        let api_key = claude_config.api.anthropic_api_key
            .clone()
            .unwrap_or_else(|| "".to_string());
        let claude_client = Arc::new(ClaudeApiClient::new(
            api_key,
            Some(claude_config.api.base_url.clone()),
        )?);

        let app_state = AppState {
            claude_client,
            config: Arc::new(RwLock::new(claude_config)),
            active_connections: Arc::new(RwLock::new(0)),
            request_stats: Arc::new(RwLock::new(RequestStats::default())),
        };

        Ok(Self {
            config,
            app_state,
        })
    }

    /// 启动服务器
    pub async fn start(&self) -> Result<()> {
        let app = self.create_app().await?;
        
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| ClaudeError::network_error(&format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("🌐 Web server starting on http://{}", addr);
        tracing::info!("📊 Dashboard available at http://{}/dashboard", addr);
        tracing::info!("🔧 API endpoint at http://{}/api/chat", addr);

        axum::serve(listener, app).await
            .map_err(|e| ClaudeError::network_error(&format!("Server error: {}", e)))?;

        Ok(())
    }

    /// 创建应用路由
    async fn create_app(&self) -> Result<Router> {
        let mut app = Router::new()
            // API 路由
            .route("/api/chat", post(chat_handler))
            .route("/api/chat/stream", post(chat_stream_handler))
            .route("/api/status", get(status_handler))
            .route("/api/stats", get(stats_handler))
            .route("/api/config", get(get_config_handler))
            .route("/api/config", post(update_config_handler))
            
            // Web 界面路由
            .route("/", get(index_handler))
            .route("/dashboard", get(dashboard_handler))
            .route("/chat", get(chat_page_handler))
            
            // 健康检查
            .route("/health", get(health_handler))
            
            // 状态
            .with_state(self.app_state.clone());

        // 添加中间件
        let middleware = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http());

        if self.config.enable_compression {
            app = app.layer(CompressionLayer::new());
        }

        if self.config.enable_cors {
            app = app.layer(CorsLayer::permissive());
        }

        // 静态文件服务
        if let Some(static_dir) = &self.config.static_dir {
            if tokio::fs::metadata(static_dir).await.is_ok() {
                app = app.nest_service("/static", ServeDir::new(static_dir));
            }
        }

        Ok(app.layer(middleware))
    }
}

/// 聊天处理器
async fn chat_handler(
    State(state): State<AppState>,
    Json(request): Json<ApiRequest>,
) -> std::result::Result<Json<ApiResponse>, StatusCode> {
    let start_time = std::time::Instant::now();
    
    // 更新连接计数
    {
        let mut connections = state.active_connections.write().await;
        *connections += 1;
    }

    let result = process_chat_request(&state, request).await;

    // 更新统计
    let processing_time = start_time.elapsed().as_millis() as u64;
    update_request_stats(&state, result.is_ok(), processing_time).await;

    // 减少连接计数
    {
        let mut connections = state.active_connections.write().await;
        *connections = connections.saturating_sub(1);
    }

    match result {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 流式聊天处理器
async fn chat_stream_handler(
    State(state): State<AppState>,
    Json(request): Json<ApiRequest>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let stream = create_chat_stream(state, request).await;
    Sse::new(stream)
}

/// 创建聊天流
async fn create_chat_stream(
    state: AppState,
    request: ApiRequest,
) -> impl Stream<Item = std::result::Result<Event, Infallible>> {
    // 这里应该实现真实的流式处理
    // 为了演示，我们创建一个模拟流
    let messages = vec![
        "Hello! I'm processing your request...",
        "Analyzing the input...",
        "Generating response...",
        "Here's my response to your message.",
    ];

    stream::iter(messages)
        .enumerate()
        .map(|(i, msg)| {
            let event = Event::default()
                .id(i.to_string())
                .event("message")
                .data(msg);
            Ok(event)
        })
}

/// 处理聊天请求
async fn process_chat_request(
    state: &AppState,
    request: ApiRequest,
) -> Result<ApiResponse> {
    // 这里应该调用真实的 Claude API
    // 为了演示，我们返回一个模拟响应
    let response = format!("Echo: {}", request.message);
    
    Ok(ApiResponse {
        response,
        model: request.model.unwrap_or_else(|| "claude-3-haiku-20240307".to_string()),
        usage: Some(TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            total_tokens: 30,
        }),
        processing_time_ms: 100,
    })
}

/// 更新请求统计
async fn update_request_stats(state: &AppState, success: bool, processing_time: u64) {
    let mut stats = state.request_stats.write().await;
    stats.total_requests += 1;
    
    if success {
        stats.successful_requests += 1;
    } else {
        stats.failed_requests += 1;
    }
    
    // 更新平均响应时间
    let total_time = stats.average_response_time_ms * (stats.total_requests - 1) as f64 + processing_time as f64;
    stats.average_response_time_ms = total_time / stats.total_requests as f64;
}

/// 状态处理器
async fn status_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let connections = *state.active_connections.read().await;
    let stats = {
        let guard = state.request_stats.read().await;
        guard.clone()
    };
    
    Json(serde_json::json!({
        "status": "healthy",
        "active_connections": connections,
        "stats": stats,
        "uptime": "unknown", // 这里可以添加真实的运行时间
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// 统计处理器
async fn stats_handler(State(state): State<AppState>) -> Json<RequestStats> {
    let stats = {
        let guard = state.request_stats.read().await;
        guard.clone()
    };
    Json(stats)
}

/// 获取配置处理器
async fn get_config_handler(State(state): State<AppState>) -> Json<ClaudeConfig> {
    let config = state.config.read().await.clone();
    Json(config)
}

/// 更新配置处理器
async fn update_config_handler(
    State(state): State<AppState>,
    Json(new_config): Json<ClaudeConfig>,
) -> StatusCode {
    let mut config = state.config.write().await;
    *config = new_config;
    StatusCode::OK
}

/// 首页处理器
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../web/templates/index.html"))
}

/// 仪表板处理器
async fn dashboard_handler() -> Html<&'static str> {
    Html(include_str!("../web/templates/dashboard.html"))
}

/// 聊天页面处理器
async fn chat_page_handler() -> Html<&'static str> {
    Html(include_str!("../web/templates/chat.html"))
}

/// 健康检查处理器
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
