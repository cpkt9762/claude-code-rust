//! 进程管理模块
//! 
//! 实现子进程启动、监控、通信和资源管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::error::{ClaudeError, Result};

/// 进程管理器
pub struct ProcessManager {
    /// 运行中的进程
    processes: Arc<Mutex<HashMap<String, ProcessInstance>>>,
    /// 下一个进程ID
    next_id: Arc<Mutex<u32>>,
}

/// 进程实例
pub struct ProcessInstance {
    /// 进程ID
    pub id: String,
    /// 进程配置
    pub config: ProcessConfig,
    /// 子进程句柄
    pub child: Option<Child>,
    /// 进程状态
    pub status: ProcessStatus,
    /// 标准输入发送器
    pub stdin_sender: Option<mpsc::UnboundedSender<String>>,
    /// 标准输出接收器
    pub stdout_receiver: Option<mpsc::UnboundedReceiver<String>>,
    /// 标准错误接收器
    pub stderr_receiver: Option<mpsc::UnboundedReceiver<String>>,
    /// 退出码
    pub exit_code: Option<i32>,
}

/// 进程配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    /// 进程名称
    pub name: String,
    /// 执行命令
    pub command: String,
    /// 命令参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: HashMap<String, String>,
    /// 工作目录
    pub working_dir: Option<String>,
    /// 超时时间（秒）
    pub timeout: Option<u64>,
    /// 是否捕获输出
    pub capture_output: bool,
    /// 是否自动重启
    pub auto_restart: bool,
}

/// 进程状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// 未启动
    NotStarted,
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 错误
    Error(String),
    /// 超时
    Timeout,
}

/// 进程输出
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub exit_code: Option<i32>,
}

impl ProcessManager {
    /// 创建新的进程管理器
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// 启动进程
    pub async fn start_process(&self, config: ProcessConfig) -> Result<String> {
        let process_id = self.generate_process_id();
        
        tracing::info!("Starting process '{}' with ID: {}", config.name, process_id);
        
        // 检查进程是否已存在
        {
            let processes = self.processes.lock().unwrap();
            if processes.contains_key(&process_id) {
                return Err(ClaudeError::General(format!(
                    "Process with ID '{}' already exists", process_id
                )));
            }
        }

        // 创建进程实例
        let mut instance = ProcessInstance {
            id: process_id.clone(),
            config: config.clone(),
            child: None,
            status: ProcessStatus::Starting,
            stdin_sender: None,
            stdout_receiver: None,
            stderr_receiver: None,
            exit_code: None,
        };

        // 构建命令
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        // 设置环境变量
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // 设置工作目录
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        // 配置标准输入输出
        if config.capture_output {
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
        }

        // 启动子进程
        let mut child = cmd.spawn().map_err(|e| {
            ClaudeError::General(format!("Failed to start process '{}': {}", config.name, e))
        })?;

        // 设置通信通道
        if config.capture_output {
            // 标准输入通道
            if let Some(stdin) = child.stdin.take() {
                let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel();
                instance.stdin_sender = Some(stdin_tx);

                // 启动标准输入写入任务
                let mut stdin_writer = stdin;
                tokio::spawn(async move {
                    while let Some(input) = stdin_rx.recv().await {
                        if let Err(e) = stdin_writer.write_all(input.as_bytes()).await {
                            tracing::error!("Failed to write to stdin: {}", e);
                            break;
                        }
                        if let Err(e) = stdin_writer.write_all(b"\n").await {
                            tracing::error!("Failed to write newline to stdin: {}", e);
                            break;
                        }
                    }
                });
            }

            // 标准输出通道
            if let Some(stdout) = child.stdout.take() {
                let (stdout_tx, stdout_rx) = mpsc::unbounded_channel();
                instance.stdout_receiver = Some(stdout_rx);

                // 启动标准输出读取任务
                let process_name = config.name.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stdout);
                    let mut line = String::new();
                    
                    while let Ok(n) = reader.read_line(&mut line).await {
                        if n == 0 {
                            break;
                        }
                        
                        let output = line.trim_end().to_string();
                        tracing::debug!("Process '{}' stdout: {}", process_name, output);
                        
                        if stdout_tx.send(output).is_err() {
                            break;
                        }
                        
                        line.clear();
                    }
                });
            }

            // 标准错误通道
            if let Some(stderr) = child.stderr.take() {
                let (stderr_tx, stderr_rx) = mpsc::unbounded_channel();
                instance.stderr_receiver = Some(stderr_rx);

                // 启动标准错误读取任务
                let process_name = config.name.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stderr);
                    let mut line = String::new();
                    
                    while let Ok(n) = reader.read_line(&mut line).await {
                        if n == 0 {
                            break;
                        }
                        
                        let output = line.trim_end().to_string();
                        tracing::warn!("Process '{}' stderr: {}", process_name, output);
                        
                        if stderr_tx.send(output).is_err() {
                            break;
                        }
                        
                        line.clear();
                    }
                });
            }
        }

        instance.child = Some(child);
        instance.status = ProcessStatus::Running;

        // 存储进程实例
        {
            let mut processes = self.processes.lock().unwrap();
            processes.insert(process_id.clone(), instance);
        }

        // 启动进程监控任务
        self.start_process_monitor(process_id.clone()).await;

        tracing::info!("Process '{}' started with ID: {}", config.name, process_id);
        Ok(process_id)
    }

    /// 停止进程
    pub async fn stop_process(&self, process_id: &str) -> Result<()> {
        tracing::info!("Stopping process: {}", process_id);

        let mut instance = {
            let mut processes = self.processes.lock().unwrap();
            processes.remove(process_id).ok_or_else(|| {
                ClaudeError::General(format!("Process '{}' not found", process_id))
            })?
        };

        instance.status = ProcessStatus::Stopping;

        if let Some(mut child) = instance.child.take() {
            // 尝试优雅关闭
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill process '{}': {}", process_id, e);
            }

            // 等待进程结束
            match child.wait().await {
                Ok(status) => {
                    instance.exit_code = status.code();
                    tracing::info!("Process '{}' exited with status: {:?}", process_id, status);
                }
                Err(e) => {
                    tracing::error!("Error waiting for process '{}': {}", process_id, e);
                    instance.status = ProcessStatus::Error(e.to_string());
                }
            }
        }

        instance.status = ProcessStatus::Stopped;
        tracing::info!("Process '{}' stopped", process_id);
        Ok(())
    }

    /// 发送输入到进程
    pub async fn send_input(&self, process_id: &str, input: &str) -> Result<()> {
        let processes = self.processes.lock().unwrap();
        let instance = processes.get(process_id).ok_or_else(|| {
            ClaudeError::General(format!("Process '{}' not found", process_id))
        })?;

        if instance.status != ProcessStatus::Running {
            return Err(ClaudeError::General(format!(
                "Process '{}' is not running", process_id
            )));
        }

        if let Some(sender) = &instance.stdin_sender {
            sender.send(input.to_string()).map_err(|e| {
                ClaudeError::General(format!("Failed to send input: {}", e))
            })?;
        } else {
            return Err(ClaudeError::General(
                "Process does not support input".to_string()
            ));
        }

        Ok(())
    }

    /// 获取进程输出
    pub async fn get_process_output(&self, process_id: &str) -> Result<ProcessOutput> {
        let mut processes = self.processes.lock().unwrap();
        let instance = processes.get_mut(process_id).ok_or_else(|| {
            ClaudeError::General(format!("Process '{}' not found", process_id))
        })?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        // 收集标准输出
        if let Some(receiver) = &mut instance.stdout_receiver {
            while let Ok(line) = receiver.try_recv() {
                stdout.push(line);
            }
        }

        // 收集标准错误
        if let Some(receiver) = &mut instance.stderr_receiver {
            while let Ok(line) = receiver.try_recv() {
                stderr.push(line);
            }
        }

        Ok(ProcessOutput {
            stdout,
            stderr,
            exit_code: instance.exit_code,
        })
    }

    /// 获取进程状态
    pub fn get_process_status(&self, process_id: &str) -> Option<ProcessStatus> {
        let processes = self.processes.lock().unwrap();
        processes.get(process_id).map(|instance| instance.status.clone())
    }

    /// 列出所有进程
    pub fn list_processes(&self) -> Vec<(String, ProcessStatus)> {
        let processes = self.processes.lock().unwrap();
        processes.iter()
            .map(|(id, instance)| (id.clone(), instance.status.clone()))
            .collect()
    }

    /// 等待进程完成
    pub async fn wait_for_process(&self, process_id: &str, timeout_secs: Option<u64>) -> Result<ProcessOutput> {
        let wait_future = async {
            loop {
                let status = self.get_process_status(process_id);
                match status {
                    Some(ProcessStatus::Stopped) | Some(ProcessStatus::Error(_)) => {
                        return self.get_process_output(process_id).await;
                    }
                    None => {
                        return Err(ClaudeError::General(format!(
                            "Process '{}' not found", process_id
                        )));
                    }
                    _ => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        };

        if let Some(timeout_secs) = timeout_secs {
            match timeout(Duration::from_secs(timeout_secs), wait_future).await {
                Ok(result) => result,
                Err(_) => {
                    // 超时，强制停止进程
                    self.stop_process(process_id).await?;
                    Err(ClaudeError::General(format!(
                        "Process '{}' timed out after {} seconds", process_id, timeout_secs
                    )))
                }
            }
        } else {
            wait_future.await
        }
    }

    /// 生成进程ID
    fn generate_process_id(&self) -> String {
        let mut next_id = self.next_id.lock().unwrap();
        let id = format!("proc_{}", *next_id);
        *next_id += 1;
        id
    }

    /// 启动进程监控任务
    async fn start_process_monitor(&self, process_id: String) {
        let processes = self.processes.clone();
        
        tokio::spawn(async move {
            // 监控进程状态
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                let should_break = {
                    let mut processes_guard = processes.lock().unwrap();
                    if let Some(instance) = processes_guard.get_mut(&process_id) {
                        if let Some(child) = &mut instance.child {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    instance.exit_code = status.code();
                                    instance.status = ProcessStatus::Stopped;
                                    tracing::info!("Process '{}' finished with status: {:?}", process_id, status);
                                    true
                                }
                                Ok(None) => false, // 仍在运行
                                Err(e) => {
                                    instance.status = ProcessStatus::Error(e.to_string());
                                    tracing::error!("Error monitoring process '{}': {}", process_id, e);
                                    true
                                }
                            }
                        } else {
                            true // 没有子进程，退出监控
                        }
                    } else {
                        true // 进程不存在，退出监控
                    }
                };
                
                if should_break {
                    break;
                }
            }
        });
    }
}
