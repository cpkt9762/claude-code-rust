//! 实时 Steering 机制实现
//! 
//! 基于原版 h2A 异步消息队列系统，实现零延迟消息传递和实时中断功能

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::time::{timeout, Duration};
use serde::{Deserialize, Serialize};
use crate::error::{ClaudeError, Result};

/// 异步消息队列系统 (h2A 类的 Rust 实现)
pub struct AsyncMessageQueue<T> {
    /// 消息缓冲队列
    queue: Arc<Mutex<VecDeque<T>>>,
    /// 等待读取的通知器
    read_notify: Arc<Notify>,
    /// 队列完成标志
    is_done: Arc<Mutex<bool>>,
    /// 错误状态
    error_state: Arc<Mutex<Option<ClaudeError>>>,
    /// 清理回调
    cleanup_callback: Option<Box<dyn Fn() + Send + Sync>>,
}

impl<T> AsyncMessageQueue<T> 
where 
    T: Send + Sync + 'static,
{
    /// 创建新的异步消息队列
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            read_notify: Arc::new(Notify::new()),
            is_done: Arc::new(Mutex::new(false)),
            error_state: Arc::new(Mutex::new(None)),
            cleanup_callback: None,
        }
    }

    /// 设置清理回调函数
    pub fn with_cleanup<F>(mut self, callback: F) -> Self 
    where 
        F: Fn() + Send + Sync + 'static,
    {
        self.cleanup_callback = Some(Box::new(callback));
        self
    }

    /// 消息入队 - 支持实时消息插入
    pub async fn enqueue(&self, message: T) -> Result<()> {
        // 检查是否已完成
        if *self.is_done.lock().await {
            return Err(ClaudeError::General("Queue is already done".to_string()));
        }

        // 检查错误状态
        if let Some(error) = &*self.error_state.lock().await {
            return Err(error.clone());
        }

        // 将消息推入队列
        self.queue.lock().await.push_back(message);
        
        // 通知等待的读取者
        self.read_notify.notify_one();
        
        Ok(())
    }

    /// 消息出队 - 非阻塞读取
    pub async fn dequeue(&self) -> Result<Option<T>> {
        loop {
            // 检查错误状态
            if let Some(error) = &*self.error_state.lock().await {
                return Err(error.clone());
            }

            // 尝试从队列中取消息
            {
                let mut queue = self.queue.lock().await;
                if let Some(message) = queue.pop_front() {
                    return Ok(Some(message));
                }
            }

            // 检查是否已完成
            if *self.is_done.lock().await {
                return Ok(None);
            }

            // 等待新消息通知
            self.read_notify.notified().await;
        }
    }

    /// 带超时的消息出队
    pub async fn dequeue_timeout(&self, duration: Duration) -> Result<Option<T>> {
        match timeout(duration, self.dequeue()).await {
            Ok(result) => result,
            Err(_) => Ok(None), // 超时返回 None
        }
    }

    /// 标记队列完成
    pub async fn done(&self) {
        *self.is_done.lock().await = true;
        self.read_notify.notify_waiters();
        
        // 执行清理回调
        if let Some(callback) = &self.cleanup_callback {
            callback();
        }
    }

    /// 设置错误状态
    pub async fn set_error(&self, error: ClaudeError) {
        *self.error_state.lock().await = Some(error);
        self.read_notify.notify_waiters();
    }

    /// 检查队列是否为空
    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }

    /// 获取队列长度
    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// 检查是否已完成
    pub async fn is_done(&self) -> bool {
        *self.is_done.lock().await
    }
}

/// Steering 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SteeringMessage {
    /// 用户输入消息
    UserInput {
        content: String,
        timestamp: u64,
    },
    /// 系统控制消息
    SystemControl {
        command: String,
        params: serde_json::Value,
    },
    /// 中断信号
    Interrupt {
        reason: String,
    },
    /// 状态更新
    StatusUpdate {
        status: String,
        data: serde_json::Value,
    },
}

/// 实时 Steering 控制器
pub struct SteeringController {
    /// 消息队列
    message_queue: AsyncMessageQueue<SteeringMessage>,
    /// 中断信号发送器
    interrupt_sender: mpsc::UnboundedSender<()>,
    /// 中断信号接收器
    interrupt_receiver: Arc<Mutex<mpsc::UnboundedReceiver<()>>>,
    /// 是否启用实时模式
    real_time_enabled: bool,
}

impl SteeringController {
    /// 创建新的 Steering 控制器
    pub fn new() -> Self {
        let (interrupt_sender, interrupt_receiver) = mpsc::unbounded_channel();
        
        Self {
            message_queue: AsyncMessageQueue::new(),
            interrupt_sender,
            interrupt_receiver: Arc::new(Mutex::new(interrupt_receiver)),
            real_time_enabled: true,
        }
    }

    /// 启用/禁用实时模式
    pub fn set_real_time_mode(&mut self, enabled: bool) {
        self.real_time_enabled = enabled;
    }

    /// 发送用户输入
    pub async fn send_user_input(&self, content: String) -> Result<()> {
        let message = SteeringMessage::UserInput {
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.message_queue.enqueue(message).await
    }

    /// 发送系统控制命令
    pub async fn send_system_control(&self, command: String, params: serde_json::Value) -> Result<()> {
        let message = SteeringMessage::SystemControl { command, params };
        self.message_queue.enqueue(message).await
    }

    /// 发送中断信号
    pub async fn send_interrupt(&self, reason: String) -> Result<()> {
        // 发送中断消息到队列
        let message = SteeringMessage::Interrupt { reason };
        self.message_queue.enqueue(message).await?;
        
        // 发送中断信号
        self.interrupt_sender.send(()).map_err(|_| {
            ClaudeError::General("Failed to send interrupt signal".to_string())
        })?;
        
        Ok(())
    }

    /// 接收消息
    pub async fn receive_message(&self) -> Result<Option<SteeringMessage>> {
        self.message_queue.dequeue().await
    }

    /// 带超时的消息接收
    pub async fn receive_message_timeout(&self, duration: Duration) -> Result<Option<SteeringMessage>> {
        self.message_queue.dequeue_timeout(duration).await
    }

    /// 检查是否有中断信号
    pub async fn check_interrupt(&self) -> bool {
        let mut receiver = self.interrupt_receiver.lock().await;
        receiver.try_recv().is_ok()
    }

    /// 等待中断信号
    pub async fn wait_for_interrupt(&self) -> Result<()> {
        let mut receiver = self.interrupt_receiver.lock().await;
        receiver.recv().await.ok_or_else(|| {
            ClaudeError::General("Interrupt channel closed".to_string())
        })?;
        Ok(())
    }

    /// 关闭控制器
    pub async fn shutdown(&self) {
        self.message_queue.done().await;
    }
}

/// Steering 会话管理器
pub struct SteeringSession {
    /// 控制器
    controller: SteeringController,
    /// 会话ID
    session_id: String,
    /// 是否活跃
    is_active: bool,
}

impl SteeringSession {
    /// 创建新的 Steering 会话
    pub fn new(session_id: String) -> Self {
        Self {
            controller: SteeringController::new(),
            session_id,
            is_active: true,
        }
    }

    /// 获取会话ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 获取控制器引用
    pub fn controller(&self) -> &SteeringController {
        &self.controller
    }

    /// 获取可变控制器引用
    pub fn controller_mut(&mut self) -> &mut SteeringController {
        &mut self.controller
    }

    /// 检查会话是否活跃
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// 关闭会话
    pub async fn close(&mut self) {
        self.is_active = false;
        self.controller.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_async_message_queue() {
        let queue = AsyncMessageQueue::new();
        
        // 测试入队和出队
        queue.enqueue("test message".to_string()).await.unwrap();
        let message = queue.dequeue().await.unwrap();
        assert_eq!(message, Some("test message".to_string()));
        
        // 测试队列完成
        queue.done().await;
        let message = queue.dequeue().await.unwrap();
        assert_eq!(message, None);
    }

    #[tokio::test]
    async fn test_steering_controller() {
        let controller = SteeringController::new();
        
        // 测试用户输入
        controller.send_user_input("Hello".to_string()).await.unwrap();
        let message = controller.receive_message().await.unwrap();
        
        match message {
            Some(SteeringMessage::UserInput { content, .. }) => {
                assert_eq!(content, "Hello");
            }
            _ => panic!("Expected UserInput message"),
        }
        
        // 测试中断信号
        controller.send_interrupt("Test interrupt".to_string()).await.unwrap();
        assert!(controller.check_interrupt().await);
    }

    #[tokio::test]
    async fn test_steering_session() {
        let mut session = SteeringSession::new("test-session".to_string());
        assert_eq!(session.session_id(), "test-session");
        assert!(session.is_active());
        
        session.close().await;
        assert!(!session.is_active());
    }
}
