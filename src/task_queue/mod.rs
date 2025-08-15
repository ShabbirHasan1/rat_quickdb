//! 任务队列模块
//! 
//! 实现基于无锁队列的任务委托系统，实现责权分离：
//! - ODM层提交任务到队列
//! - 工作线程从队列获取任务并执行
//! - 通过通道返回执行结果

pub mod task;
pub mod manager;
pub mod worker;
pub mod error;

pub use task::*;
pub use manager::*;
pub use worker::*;
pub use error::*;

use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, oneshot};
use crate::PoolManager;
use crate::table::TableManager;
use crate::error::QuickDbResult;

/// 任务队列系统的全局实例
static GLOBAL_TASK_QUEUE: OnceLock<Arc<TaskQueueManager>> = OnceLock::new();

/// 获取全局任务队列管理器
pub fn get_global_task_queue() -> Arc<TaskQueueManager> {
    GLOBAL_TASK_QUEUE
        .get()
        .expect("任务队列未初始化，请先调用 initialize_global_task_queue")
        .clone()
}

/// 初始化全局任务队列管理器
pub async fn initialize_global_task_queue(
    pool_manager: Arc<PoolManager>,
    table_manager: Arc<TableManager>,
    worker_count: usize,
) -> QuickDbResult<()> {
    let task_queue = TaskQueueManager::new(pool_manager, table_manager, worker_count).await?;
    
    GLOBAL_TASK_QUEUE
        .set(Arc::new(task_queue))
        .map_err(|_| crate::error::QuickDbError::Other(anyhow::anyhow!("任务队列已经初始化")))?;
    
    Ok(())
}

/// 关闭全局任务队列管理器
pub async fn shutdown_global_task_queue() -> QuickDbResult<()> {
    if let Some(task_queue) = GLOBAL_TASK_QUEUE.get() {
        task_queue.shutdown().await?;
    }
    Ok(())
}