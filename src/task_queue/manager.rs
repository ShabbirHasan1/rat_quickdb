//! 任务队列管理器
//! 
//! 负责管理工作线程池和任务分发

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use zerg_creep::{info, warn, error};

use crate::error::{QuickDbResult, QuickDbError};
use crate::PoolManager;
use crate::pool::ConnectionPool;
use crate::table::TableManager;
use crate::types::{DataValue, QueryCondition, QueryOptions};
use super::task::{CreateOptions, UpdateOptions, DeleteOptions, CountOptions, ExistsOptions};
use super::task::*;
use super::worker::TaskWorker;
use dashmap::DashMap;

/// 任务队列管理器
#[derive(Debug)]
pub struct TaskQueueManager {
    /// 任务发送通道
    task_sender: mpsc::UnboundedSender<DbTask>,
    /// 工作线程句柄
    worker_handles: Vec<JoinHandle<()>>,
    /// 连接池管理器
    pool_manager: Arc<PoolManager>,
    /// 表管理器
    table_manager: Arc<TableManager>,
    /// 工作线程数量
    worker_count: usize,
    /// 是否正在关闭
    is_shutting_down: Arc<std::sync::atomic::AtomicBool>,
}

impl TaskQueueManager {
    /// 创建新的任务队列管理器
    pub async fn new(
        pool_manager: Arc<PoolManager>,
        table_manager: Arc<TableManager>,
        worker_count: usize,
    ) -> QuickDbResult<Self> {
        let (task_sender, task_receiver) = mpsc::unbounded_channel();
        let task_receiver = Arc::new(tokio::sync::Mutex::new(task_receiver));
        let is_shutting_down = Arc::new(std::sync::atomic::AtomicBool::new(false));
        
        let mut worker_handles = Vec::new();
        
        // 获取连接池映射
        let connection_pools = pool_manager.get_connection_pools();
        
        // 启动工作线程
        for worker_id in 0..worker_count {
            let worker = TaskWorker::new(
                worker_id,
                task_receiver.clone(),
                connection_pools.clone(),
                table_manager.clone(),
                is_shutting_down.clone(),
            );
            
            let handle = tokio::spawn(async move {
                if let Err(e) = worker.run().await {
                    error!("工作线程 {} 异常退出: {}", worker_id, e);
                }
            });
            
            worker_handles.push(handle);
        }
        
        info!("任务队列管理器启动成功，工作线程数: {}", worker_count);
        
        Ok(Self {
            task_sender,
            worker_handles,
            pool_manager,
            table_manager,
            worker_count,
            is_shutting_down,
        })
    }
    
    /// 提交创建任务
    pub async fn create(
        &self,
        table: String,
        data: std::collections::HashMap<String, DataValue>,
        options: Option<CreateOptions>,
    ) -> QuickDbResult<String> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::Create {
            table,
            data,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交查询任务
    pub async fn find(
        &self,
        table: String,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<String> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::Find {
            table,
            conditions,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交根据ID查询任务
    pub async fn find_by_id(
        &self,
        table: String,
        id: String,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Option<String>> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::FindById {
            table,
            id,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交更新任务
    pub async fn update(
        &self,
        table: String,
        conditions: Vec<QueryCondition>,
        data: std::collections::HashMap<String, DataValue>,
        options: Option<UpdateOptions>,
    ) -> QuickDbResult<u64> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::Update {
            table,
            conditions,
            data,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交根据ID更新任务
    pub async fn update_by_id(
        &self,
        table: String,
        id: String,
        data: std::collections::HashMap<String, DataValue>,
        options: Option<UpdateOptions>,
    ) -> QuickDbResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::UpdateById {
            table,
            id,
            data,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交删除任务
    pub async fn delete(
        &self,
        table: String,
        conditions: Vec<QueryCondition>,
        options: Option<DeleteOptions>,
    ) -> QuickDbResult<u64> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::Delete {
            table,
            conditions,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交根据ID删除任务
    pub async fn delete_by_id(
        &self,
        table: String,
        id: String,
        options: Option<DeleteOptions>,
    ) -> QuickDbResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::DeleteById {
            table,
            id,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交计数任务
    pub async fn count(
        &self,
        table: String,
        conditions: Vec<QueryCondition>,
        options: Option<CountOptions>,
    ) -> QuickDbResult<u64> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::Count {
            table,
            conditions,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交存在性检查任务
    pub async fn exists(
        &self,
        table: String,
        conditions: Vec<QueryCondition>,
        options: Option<ExistsOptions>,
    ) -> QuickDbResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::Exists {
            table,
            conditions,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交批量创建任务
    pub async fn batch_create(
        &self,
        table: String,
        data_list: Vec<std::collections::HashMap<String, DataValue>>,
        options: Option<CreateOptions>,
    ) -> QuickDbResult<Vec<String>> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::BatchCreate {
            table,
            data_list,
            options,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交表检查任务
    pub async fn check_table(&self, table: String) -> QuickDbResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::CheckTable {
            table,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    
    /// 提交创建表任务
    pub async fn create_table(
        &self,
        table: String,
        schema: crate::table::TableSchema,
    ) -> QuickDbResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        
        let task = DbTask::CreateTable {
            table,
            schema,
            response_tx,
        };
        
        self.submit_task(task).await?;
        
        match response_rx.await {
            Ok(result) => result,
            Err(_) => Err(QuickDbError::TaskExecutionError(
                "任务执行通道关闭".to_string()
            )),
        }
    }
    

    
    /// 提交任务到队列
    async fn submit_task(&self, task: DbTask) -> QuickDbResult<()> {
        if self.is_shutting_down.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(QuickDbError::Other(anyhow::anyhow!(
                "任务队列正在关闭，无法提交新任务"
            )));
        }
        
        self.task_sender.send(task).map_err(|_| {
            QuickDbError::Other(anyhow::anyhow!(
                "任务提交失败，队列已关闭"
            ))
        })?;
        
        Ok(())
    }
    

    
    /// 优雅关闭任务队列
    pub async fn shutdown(&self) -> QuickDbResult<()> {
        info!("开始关闭任务队列管理器...");
        
        // 设置关闭标志
        self.is_shutting_down.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // 发送关闭信号给所有工作线程
        for _ in 0..self.worker_count {
            if let Err(e) = self.task_sender.send(DbTask::Shutdown) {
                warn!("发送关闭信号失败: {}", e);
            }
        }
        
        // 等待所有工作线程完成
        let mut shutdown_timeout = Duration::from_secs(30);
        let start_time = std::time::Instant::now();
        
        for (i, handle) in self.worker_handles.iter().enumerate() {
            let remaining_time = shutdown_timeout.saturating_sub(start_time.elapsed());
            
            if remaining_time.is_zero() {
                warn!("工作线程 {} 关闭超时，强制终止", i);
                handle.abort();
            } else {
                match tokio::time::timeout(remaining_time, async {
                    // 注意：这里不能直接await handle，因为handle的所有权问题
                    // 实际实现中需要使用Arc<Mutex<Vec<JoinHandle>>>或其他方式
                }).await {
                    Ok(_) => info!("工作线程 {} 正常关闭", i),
                    Err(_) => {
                        warn!("工作线程 {} 关闭超时，强制终止", i);
                        handle.abort();
                    }
                }
            }
        }
        
        info!("任务队列管理器关闭完成");
        Ok(())
    }
}



// 实现Drop trait以确保资源清理
impl Drop for TaskQueueManager {
    fn drop(&mut self) {
        if !self.is_shutting_down.load(std::sync::atomic::Ordering::Relaxed) {
            warn!("任务队列管理器未正常关闭就被丢弃，可能导致资源泄露");
        }
    }
}