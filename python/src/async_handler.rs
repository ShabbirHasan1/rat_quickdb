use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use rat_quickdb::manager::{add_database, get_global_pool_manager};
use rat_quickdb::types::{DataValue, QueryCondition, QueryConditionGroup, QueryOptions, DatabaseConfig};
use rat_quickdb::{QuickDbError, QuickDbResult};
use rat_logger::{debug, error, info};
use crate::message::{PyDbRequest, PyDbResponse};

/// 异步数据库队列桥接器
/// 负责处理异步数据库操作请求
pub struct PyDbQueueBridgeAsync;

impl PyDbQueueBridgeAsync {
    /// 守护任务主循环
    pub async fn daemon_task(mut request_receiver: mpsc::UnboundedReceiver<PyDbRequest>) {
        info!("启动数据库队列守护任务");

        while let Some(request) = request_receiver.recv().await {
            debug!("收到请求: operation={}, collection={}", request.operation, request.collection);
            
            // 直接处理请求，不使用ODM管理器
            let response = Self::process_request(&request).await;
            
            if let Err(e) = request.response_sender.send(response) {
                error!("发送响应失败: {:?}", e);
            }
        }
        
        info!("数据库队列守护任务结束");
    }

    /// 直接处理请求
    async fn process_request(request: &PyDbRequest) -> PyDbResponse {
        let request_id = request.request_id.clone();
        
        match request.operation.as_str() {
            "add_database" => {
                if let Some(db_config) = &request.database_config {
                    match add_database(db_config.clone()).await {
                        Ok(_) => PyDbResponse {
                            request_id,
                            success: true,
                            data: serde_json::json!({
                                "success": true,
                                "message": format!("数据库 '{}' 添加成功", db_config.alias)
                            }).to_string(),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("添加数据库失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少数据库配置".to_string()),
                    }
                }
            }
            "create" => {
                if let Some(data) = &request.data {
                    match Self::handle_create_direct(&request.collection, data.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result,
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("创建失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少数据".to_string()),
                    }
                }
            }
            "find_by_id" => {
                if let Some(id) = &request.id {
                    match Self::handle_find_by_id_direct(&request.collection, id, request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result.unwrap_or_else(|| "null".to_string()),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("查询失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少ID".to_string()),
                    }
                }
            }
            "find" => {
                if let Some(conditions) = &request.conditions {
                    match Self::handle_find_direct(&request.collection, conditions.clone(), request.options.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result,
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("查询失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少查询条件".to_string()),
                    }
                }
            }
            "find_with_groups" => {
                if let Some(condition_groups) = &request.condition_groups {
                    match Self::handle_find_with_groups_direct(&request.collection, condition_groups.clone(), request.options.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result,
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("条件组合查询失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少查询条件组合".to_string()),
                    }
                }
            }
            "delete" => {
                if let Some(conditions) = &request.conditions {
                    match Self::handle_delete_direct(&request.collection, conditions.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result.to_string(),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("删除失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少删除条件".to_string()),
                    }
                }
            }
            "delete_by_id" => {
                if let Some(id) = &request.id {
                    match Self::handle_delete_by_id_direct(&request.collection, id, request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result.to_string(),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("按ID删除失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少ID".to_string()),
                    }
                }
            }
            "update" => {
                if let (Some(conditions), Some(updates)) = (&request.conditions, &request.updates) {
                    match Self::handle_update_direct(&request.collection, conditions.clone(), updates.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result.to_string(),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("更新失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少更新条件或数据".to_string()),
                    }
                }
            }
            "update_by_id" => {
                if let (Some(id), Some(updates)) = (&request.id, &request.updates) {
                    match Self::handle_update_by_id_direct(&request.collection, id, updates.clone(), request.alias.clone()).await {
                        Ok(result) => PyDbResponse {
                            request_id,
                            success: true,
                            data: result.to_string(),
                            error: None,
                        },
                        Err(e) => PyDbResponse {
                            request_id,
                            success: false,
                            data: String::new(),
                            error: Some(format!("按ID更新失败: {}", e)),
                        },
                    }
                } else {
                    PyDbResponse {
                        request_id,
                        success: false,
                        data: String::new(),
                        error: Some("缺少ID或更新数据".to_string()),
                    }
                }
            }
            _ => PyDbResponse {
                request_id,
                success: false,
                data: String::new(),
                error: Some(format!("不支持的操作: {}", request.operation)),
            },
        }
    }

    /// 直接使用连接池处理创建请求
    async fn handle_create_direct(
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<String> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理创建请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Create请求到连接池
        let operation = DatabaseOperation::Create {
            table: collection.to_string(),
            data,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        // 将DataValue转换为JsonValue，避免Object包装问题
        let json_value = result.to_json_value();
        Ok(serde_json::to_string(&json_value)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?)
    }
    
    /// 直接使用连接池处理根据ID查询请求
    async fn handle_find_by_id_direct(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<Option<String>> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理根据ID查询请求: collection={}, id={}, alias={}", collection, id, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::FindById请求到连接池
        let operation = DatabaseOperation::FindById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        if let Some(value) = result {
            // 将DataValue转换为JsonValue，避免Object包装问题
            let json_value = value.to_json_value();
            Ok(Some(serde_json::to_string(&json_value)
                .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?))
        } else {
            Ok(None)
        }
    }
    
    /// 直接使用连接池处理查询请求
    async fn handle_find_direct(
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<String> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理查询请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Find请求到连接池
        let operation = DatabaseOperation::Find {
            table: collection.to_string(),
            conditions,
            options: options.unwrap_or_default(),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        // 将DataValue结果转换为JsonValue，避免Object包装问题
        let json_array: Vec<serde_json::Value> = result.into_iter()
            .map(|data_value| data_value.to_json_value())
            .collect();
        
        serde_json::to_string(&json_array)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })
    }

    /// 处理支持OR逻辑的条件组合查询
    async fn handle_find_with_groups_direct(
        collection: &str,
        condition_groups: Vec<QueryConditionGroup>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<String> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理条件组合查询请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::FindWithGroups请求到连接池
        let operation = DatabaseOperation::FindWithGroups {
            table: collection.to_string(),
            condition_groups,
            options: options.unwrap_or_default(),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        // 将DataValue结果转换为JsonValue，避免Object包装问题
        let json_array: Vec<serde_json::Value> = result.into_iter()
            .map(|data_value| data_value.to_json_value())
            .collect();
        
        serde_json::to_string(&json_array)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })
    }

    /// 直接使用连接池处理删除请求
    async fn handle_delete_direct(
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理删除请求: collection={}, alias={}", collection, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Delete请求到连接池
        let operation = DatabaseOperation::Delete {
            table: collection.to_string(),
            conditions,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(result)
    }

    /// 直接使用连接池处理按ID删除请求
    async fn handle_delete_by_id_direct(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理按ID删除请求: collection={}, id={}, alias={}", collection, id, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::DeleteById请求到连接池
        let operation = DatabaseOperation::DeleteById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(result)
    }

    /// 处理更新操作
    async fn handle_update_direct(
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理更新请求: collection={}, conditions={:?}, updates={:?}, alias={}", collection, conditions, updates, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::Update请求到连接池
        let operation = DatabaseOperation::Update {
            table: collection.to_string(),
            conditions,
            data: updates,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(result)
    }

    /// 处理按ID更新操作
    async fn handle_update_by_id_direct(
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理按ID更新请求: collection={}, id={}, updates={:?}, alias={}", collection, id, updates, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::UpdateById请求到连接池
        let operation = DatabaseOperation::UpdateById {
            table: collection.to_string(),
            id: DataValue::String(id.to_string()),
            data: updates,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        Ok(result)
    }
}