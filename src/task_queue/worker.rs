//! 任务工作线程
//! 
//! 基于新的生产者/消费者模式的任务处理器
//! 直接使用ConnectionPool，避免连接克隆和锁竞争

use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};
use dashmap::DashMap;
use zerg_creep::{info, warn, error, debug};

use crate::error::{QuickDbResult, QuickDbError};
use crate::pool::{ConnectionPool, DatabaseOperation};
use crate::table::TableManager;
use crate::types::{DataValue, QueryCondition, QueryOptions};
use super::task::{CreateOptions, UpdateOptions, DeleteOptions, CountOptions, ExistsOptions, TransactionOptions};

use super::task::*;

/// 任务工作线程 - 重新设计为直接使用ConnectionPool
#[derive(Debug)]
pub struct TaskWorker {
    /// 工作线程ID
    worker_id: usize,
    /// 任务接收通道
    task_receiver: Arc<Mutex<mpsc::UnboundedReceiver<DbTask>>>,
    /// 连接池映射 (alias -> ConnectionPool)
    connection_pools: Arc<DashMap<String, Arc<ConnectionPool>>>,
    /// 表管理器
    table_manager: Arc<TableManager>,
    /// 是否正在关闭
    is_shutting_down: Arc<std::sync::atomic::AtomicBool>,
}

impl TaskWorker {
    /// 创建新的任务工作线程
    pub fn new(
        worker_id: usize,
        task_receiver: Arc<Mutex<mpsc::UnboundedReceiver<DbTask>>>,
        connection_pools: Arc<DashMap<String, Arc<ConnectionPool>>>,
        table_manager: Arc<TableManager>,
        is_shutting_down: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            worker_id,
            task_receiver,
            connection_pools,
            table_manager,
            is_shutting_down,
        }
    }
    
    /// 运行工作线程
    pub async fn run(mut self) -> QuickDbResult<()> {
        info!("工作线程 {} 启动", self.worker_id);
        
        loop {
            // 检查是否需要关闭
            if self.is_shutting_down.load(std::sync::atomic::Ordering::Relaxed) {
                info!("工作线程 {} 收到关闭信号", self.worker_id);
                break;
            }
            
            // 从队列获取任务
            let task = {
                let mut receiver = self.task_receiver.lock().await;
                receiver.recv().await
            };
            
            match task {
                Some(DbTask::Shutdown) => {
                    info!("工作线程 {} 收到关闭任务", self.worker_id);
                    break;
                }
                Some(task) => {
                    // 处理任务
                    match self.process_task(task).await {
                        Ok(_) => {
                            debug!("工作线程 {} 成功处理任务", self.worker_id);
                        }
                        Err(e) => {
                            error!("工作线程 {} 处理任务失败: {}", self.worker_id, e);
                        }
                    }
                }
                None => {
                    // 通道关闭，退出循环
                    info!("工作线程 {} 任务通道关闭", self.worker_id);
                    break;
                }
            }
        }
        
        info!("工作线程 {} 退出", self.worker_id);
        Ok(())
    }
    
    /// 处理单个任务
    async fn process_task(&self, task: DbTask) -> QuickDbResult<()> {
        match task {
            DbTask::Create { table, data, options, response_tx } => {
                let result = self.handle_create(&table, data, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::Find { table, conditions, options, response_tx } => {
                let result = self.handle_find(&table, conditions, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::FindById { table, id, options, response_tx } => {
                let result = self.handle_find_by_id(&table, &id, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::Update { table, conditions, data, options, response_tx } => {
                let result = self.handle_update(&table, conditions, data, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::UpdateById { table, id, data, options, response_tx } => {
                let result = self.handle_update_by_id(&table, &id, data, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::Delete { table, conditions, options, response_tx } => {
                let result = self.handle_delete(&table, conditions, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::DeleteById { table, id, options, response_tx } => {
                let result = self.handle_delete_by_id(&table, &id, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::Count { table, conditions, options, response_tx } => {
                let result = self.handle_count(&table, conditions, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::Exists { table, conditions, options, response_tx } => {
                let result = self.handle_exists(&table, conditions, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::BatchCreate { table, data_list, options, response_tx } => {
                let result = self.handle_batch_create(&table, data_list, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::BatchUpdate { table, updates, options, response_tx } => {
                let result = self.handle_batch_update(&table, updates, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::BatchDelete { table, conditions_list, options, response_tx } => {
                let result = self.handle_batch_delete(&table, conditions_list, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::CheckTable { table, response_tx } => {
                let result = self.handle_check_table(&table).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::CreateTable { table, schema, response_tx } => {
                let result = self.handle_create_table(&table, schema).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::DropTable { table, cascade, response_tx } => {
                let result = self.handle_drop_table(&table, cascade).await;
                let _ = response_tx.send(result);
            }
            DbTask::DropAndRecreateTable { schema, options, response_tx } => {
                let result = self.handle_drop_and_recreate_table(schema, options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::MigrateTable { table, from_version, to_version, response_tx } => {
                let result = self.handle_migrate_table(&table, from_version, to_version).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::GetTableSchema { table, response_tx } => {
                let result = self.handle_get_table_schema(&table).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::ListTables { response_tx } => {
                let result = self.handle_list_tables().await;
                let _ = response_tx.send(result);
            }
            
            DbTask::BeginTransaction { options, response_tx } => {
                let result = self.handle_begin_transaction(options).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::CommitTransaction { transaction_id, response_tx } => {
                let result = self.handle_commit_transaction(&transaction_id).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::RollbackTransaction { transaction_id, response_tx } => {
                let result = self.handle_rollback_transaction(&transaction_id).await;
                let _ = response_tx.send(result);
            }
            

            
            DbTask::ExecuteRaw { sql, params, response_tx } => {
                let result = self.handle_execute_raw(&sql, params).await;
                let _ = response_tx.send(result);
            }
            
            DbTask::Shutdown => {
                // 这个case在上层已经处理
                unreachable!()
            }
        }
        
        Ok(())
    }
    
    /// 获取指定别名的连接池
    fn get_connection_pool(&self, alias: Option<&str>) -> QuickDbResult<Arc<ConnectionPool>> {
        let alias = alias.unwrap_or("default");
        self.connection_pools
            .get(alias)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| QuickDbError::ConfigError {
                message: format!("连接池别名 '{}' 不存在", alias),
            })
    }
    
    // === 数据操作处理函数 ===
    
    async fn handle_create(
        &self,
        table: &str,
        data: HashMap<String, DataValue>,
        options: Option<CreateOptions>,
    ) -> QuickDbResult<String> {
        let pool = self.get_connection_pool(None)?;
        
        // 使用新的ConnectionPool的消息传递机制
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::Create {
            table: table.to_string(),
            data,
            response: tx,
        };
        
        // 发送操作到连接池
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待结果
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result.to_string())
    }
    
    async fn handle_find(
        &self,
        table: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<String> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::Find {
            table: table.to_string(),
            conditions,
            options: options.unwrap_or_default(),
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(serde_json::to_string(&result).unwrap_or_default())
    }
    
    async fn handle_find_by_id(
        &self,
        table: &str,
        id: &str,
        options: Option<QueryOptions>,
    ) -> QuickDbResult<Option<String>> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::FindById {
            table: table.to_string(),
            id: DataValue::String(id.to_string()),
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result.map(|v| v.to_string()))
    }
    
    async fn handle_update(
        &self,
        table: &str,
        conditions: Vec<QueryCondition>,
        data: HashMap<String, DataValue>,
        options: Option<UpdateOptions>,
    ) -> QuickDbResult<u64> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::Update {
            table: table.to_string(),
            conditions,
            data,
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result)
    }
    
    async fn handle_update_by_id(
        &self,
        table: &str,
        id: &str,
        data: HashMap<String, DataValue>,
        options: Option<UpdateOptions>,
    ) -> QuickDbResult<bool> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::UpdateById {
            table: table.to_string(),
            id: DataValue::String(id.to_string()),
            data,
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result)
    }
    
    async fn handle_delete(
        &self,
        table: &str,
        conditions: Vec<QueryCondition>,
        options: Option<DeleteOptions>,
    ) -> QuickDbResult<u64> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::Delete {
            table: table.to_string(),
            conditions,
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result)
    }
    
    async fn handle_delete_by_id(
        &self,
        table: &str,
        id: &str,
        options: Option<DeleteOptions>,
    ) -> QuickDbResult<bool> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::DeleteById {
            table: table.to_string(),
            id: DataValue::String(id.to_string()),
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result)
    }
    
    async fn handle_count(
        &self,
        table: &str,
        conditions: Vec<QueryCondition>,
        _options: Option<CountOptions>,
    ) -> QuickDbResult<u64> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::Count {
            table: table.to_string(),
            conditions,
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result)
    }
    
    async fn handle_exists(
        &self,
        table: &str,
        conditions: Vec<QueryCondition>,
        _options: Option<ExistsOptions>,
    ) -> QuickDbResult<bool> {
        let pool = self.get_connection_pool(None)?;
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let operation = DatabaseOperation::Exists {
            table: table.to_string(),
            conditions,
            response: tx,
        };
        
        pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        let result = rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待数据库操作结果超时".to_string(),
            })??;
        
        Ok(result)
    }
    
    async fn handle_batch_create(
        &self,
        table: &str,
        data_list: Vec<std::collections::HashMap<String, DataValue>>,
        options: Option<CreateOptions>,
    ) -> QuickDbResult<Vec<String>> {
        let pool = self.get_connection_pool(None)?;
        
        let mut results = Vec::new();
        
        // 批量处理每个创建操作
        for data in data_list {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let operation = DatabaseOperation::Create {
                table: table.to_string(),
                data,
                response: tx,
            };
            
            pool.operation_sender.send(operation)
                .map_err(|_| QuickDbError::ConnectionError {
                    message: "连接池操作通道已关闭".to_string(),
                })?;
            
            let result = rx.await
                .map_err(|_| QuickDbError::ConnectionError {
                    message: "等待数据库操作结果超时".to_string(),
                })??;
            
            results.push(result.to_string());
        }
        
        Ok(results)
    }
    
    async fn handle_batch_update(
        &self,
        table: &str,
        updates: Vec<(Vec<QueryCondition>, std::collections::HashMap<String, DataValue>)>,
        options: Option<UpdateOptions>,
    ) -> QuickDbResult<u64> {
        let pool = self.get_connection_pool(None)?;
        
        let mut total_updated = 0;
        
        // 批量处理每个更新操作
        for (conditions, data) in updates {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let operation = DatabaseOperation::Update {
                table: table.to_string(),
                conditions,
                data,
                response: tx,
            };
            
            pool.operation_sender.send(operation)
                .map_err(|_| QuickDbError::ConnectionError {
                    message: "连接池操作通道已关闭".to_string(),
                })?;
            
            let updated = rx.await
                .map_err(|_| QuickDbError::ConnectionError {
                    message: "等待数据库操作结果超时".to_string(),
                })??;
            
            total_updated += updated;
        }
        
        Ok(total_updated)
    }
    
    async fn handle_batch_delete(
        &self,
        table: &str,
        conditions_list: Vec<Vec<QueryCondition>>,
        options: Option<DeleteOptions>,
    ) -> QuickDbResult<u64> {
        let pool = self.get_connection_pool(None)?;
        
        let mut total_deleted = 0;
        
        // 批量处理每个删除操作
        for conditions in conditions_list {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let operation = DatabaseOperation::Delete {
                table: table.to_string(),
                conditions,
                response: tx,
            };
            
            pool.operation_sender.send(operation)
                .map_err(|_| QuickDbError::ConnectionError {
                    message: "连接池操作通道已关闭".to_string(),
                })?;
            
            let deleted = rx.await
                .map_err(|_| QuickDbError::ConnectionError {
                    message: "等待数据库操作结果超时".to_string(),
                })??;
            
            total_deleted += deleted;
        }
        
        Ok(total_deleted)
    }
    
    // === 表管理处理函数 ===
    
    async fn handle_check_table(&self, table: &str) -> QuickDbResult<bool> {
        self.table_manager.check_table_exists(table).await
    }
    
    async fn handle_create_table(
        &self,
        table: &str,
        schema: crate::table::TableSchema,
    ) -> QuickDbResult<()> {
        self.table_manager.create_table(&schema, None).await
    }
    
    async fn handle_drop_table(&self, table: &str, _cascade: bool) -> QuickDbResult<()> {
        // 调用表管理器的删除表方法
        self.table_manager.drop_table(table).await
    }
    
    /// 处理删除并重建表任务
    async fn handle_drop_and_recreate_table(
        &self,
        schema: crate::table::TableSchema,
        options: Option<crate::table::manager::TableCreateOptions>,
    ) -> QuickDbResult<()> {
        // 调用表管理器的删除并重建表方法
        self.table_manager.drop_and_recreate_table(&schema, options).await
    }
    
    async fn handle_migrate_table(
        &self,
        table: &str,
        from_version: u32,
        to_version: u32,
    ) -> QuickDbResult<()> {
        self.table_manager.migrate_table(table, Some(to_version)).await
    }
    
    async fn handle_get_table_schema(&self, table: &str) -> QuickDbResult<Option<crate::table::TableSchema>> {
        self.table_manager.get_table_schema(table).await
    }
    
    async fn handle_list_tables(&self) -> QuickDbResult<Vec<String>> {
        self.table_manager.list_tables().await
    }
    
    // === 事务处理函数 ===
    
    async fn handle_begin_transaction(
        &self,
        _options: Option<TransactionOptions>,
    ) -> QuickDbResult<String> {
        // TODO: 实现事务功能
        Err(QuickDbError::Other(anyhow::anyhow!("事务功能尚未实现")))
    }
    
    async fn handle_commit_transaction(&self, _transaction_id: &str) -> QuickDbResult<()> {
        // TODO: 实现事务功能
        Err(QuickDbError::Other(anyhow::anyhow!("事务功能尚未实现")))
    }
    
    async fn handle_rollback_transaction(&self, _transaction_id: &str) -> QuickDbResult<()> {
        // TODO: 实现事务功能
        Err(QuickDbError::Other(anyhow::anyhow!("事务功能尚未实现")))
    }
    
    // === 系统处理函数 ===
    
    async fn handle_execute_raw(
        &self,
        _sql: &str,
        _params: Vec<DataValue>,
    ) -> QuickDbResult<String> {
        // TODO: 实现原始SQL执行功能
        Err(QuickDbError::Other(anyhow::anyhow!(
            "原始SQL执行功能尚未实现"
        )))
    }
    

}