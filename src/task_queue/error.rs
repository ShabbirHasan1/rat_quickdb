//! 任务队列错误处理
//! 
//! 定义任务队列相关的错误类型

use thiserror::Error;

/// 任务队列错误类型
#[derive(Error, Debug)]
pub enum TaskQueueError {
    /// 队列已满
    #[error("任务队列已满，无法添加更多任务")]
    QueueFull,
    
    /// 队列已关闭
    #[error("任务队列已关闭")]
    QueueClosed,
    
    /// 工作线程启动失败
    #[error("工作线程启动失败: {0}")]
    WorkerStartFailed(String),
    
    /// 工作线程异常退出
    #[error("工作线程 {worker_id} 异常退出: {reason}")]
    WorkerCrashed { worker_id: usize, reason: String },
    
    /// 任务提交超时
    #[error("任务提交超时")]
    SubmissionTimeout,
    
    /// 任务执行超时
    #[error("任务执行超时")]
    ExecutionTimeout,
    
    /// 响应通道错误
    #[error("响应通道错误: {0}")]
    ResponseChannelError(String),
    
    /// 连接池错误
    #[error("连接池错误: {0}")]
    PoolError(#[from] crate::error::QuickDbError),
    
    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    /// IO错误
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    
    /// 其他错误
    #[error("其他错误: {0}")]
    Other(String),
}

/// 任务队列结果类型
pub type TaskQueueResult<T> = Result<T, TaskQueueError>;

/// 将TaskQueueError转换为QuickDbError
impl From<TaskQueueError> for crate::error::QuickDbError {
    fn from(err: TaskQueueError) -> Self {
        match err {
            TaskQueueError::QueueFull => {
                crate::error::QuickDbError::PoolError {
                    message: "任务队列已满".to_string(),
                }
            }
            TaskQueueError::QueueClosed => {
                crate::error::QuickDbError::ConnectionError {
                    message: "任务队列已关闭".to_string(),
                }
            }
            TaskQueueError::WorkerStartFailed(msg) => {
                crate::error::QuickDbError::ConfigError {
                    message: format!("工作线程启动失败: {}", msg),
                }
            }
            TaskQueueError::WorkerCrashed { worker_id, reason } => {
                crate::error::QuickDbError::ConnectionError {
                    message: format!("工作线程 {} 异常退出: {}", worker_id, reason),
                }
            }
            TaskQueueError::SubmissionTimeout => {
                crate::error::QuickDbError::PoolError {
                    message: "任务提交超时".to_string(),
                }
            }
            TaskQueueError::ExecutionTimeout => {
                crate::error::QuickDbError::QueryError {
                    message: "任务执行超时".to_string(),
                }
            }
            TaskQueueError::ResponseChannelError(msg) => {
                crate::error::QuickDbError::ConnectionError {
                    message: format!("响应通道错误: {}", msg),
                }
            }
            TaskQueueError::PoolError(err) => err,
            TaskQueueError::SerializationError(err) => {
                crate::error::QuickDbError::SerializationError {
                    message: err.to_string(),
                }
            }
            TaskQueueError::IoError(err) => {
                crate::error::QuickDbError::ConnectionError {
                    message: format!("IO错误: {}", err),
                }
            }
            TaskQueueError::Other(msg) => {
                crate::error::QuickDbError::Other(anyhow::anyhow!(msg))
            }
        }
    }
}