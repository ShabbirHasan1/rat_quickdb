//! 数据库队列桥接器模块
//! 提供Python与Rust数据库操作的桥接功能

use crate::config::*;
use crate::message::*;
use crossbeam::queue::SegQueue;
use pyo3::prelude::*;
use rat_quickdb::config::{DatabaseConfigBuilder, PoolConfigBuilder};
use rat_quickdb::manager::{add_database, get_global_pool_manager};
use rat_quickdb::odm::{get_odm_manager, OdmOperations};
use rat_quickdb::types::DatabaseConfig;
use rat_quickdb::types::{
    ConnectionConfig, DataValue, DatabaseType, IdStrategy, PaginationConfig, PoolConfig,
    QueryCondition, QueryOperator, QueryOptions, SortConfig, SortDirection,
    QueryConditionGroup, LogicalOperator,
    CacheConfig, CacheStrategy, L1CacheConfig, L2CacheConfig, TtlConfig, CompressionConfig, CompressionAlgorithm,
    TlsConfig, ZstdConfig,
};
use rat_quickdb::{QuickDbError, QuickDbResult};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use zerg_creep::{debug, error, info, warn};

/// Python数据库队列桥接器
/// 提供异步数据库操作的Python接口
#[pyclass(name = "DbQueueBridge")]
pub struct PyDbQueueBridge {
    /// 请求发送器
    request_sender: Arc<std::sync::Mutex<Option<mpsc::UnboundedSender<PyDbRequest>>>>,
    /// 默认数据库别名
    default_alias: Arc<std::sync::Mutex<Option<String>>>,
    /// 任务句柄
    _task_handle: Arc<std::sync::Mutex<Option<std::thread::JoinHandle<()>>>>,
    /// 初始化状态
    initialized: Arc<std::sync::Mutex<bool>>,
    /// Tokio运行时
    runtime: Arc<Runtime>,
}

#[pymethods]
impl PyDbQueueBridge {
    /// 创建新的数据库队列桥接器
    #[new]
    pub fn new() -> PyResult<Self> {
        let runtime = Arc::new(
            Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("创建Tokio运行时失败: {}", e)))?
        );
        
        let (request_sender, request_receiver) = mpsc::unbounded_channel::<PyDbRequest>();
        let request_sender = Arc::new(std::sync::Mutex::new(Some(request_sender)));
        
        let task_handle = thread::spawn(move || {
            let rt = Runtime::new().expect("创建异步运行时失败");
            rt.block_on(PyDbQueueBridgeAsync::daemon_task(request_receiver));
        });
        
        let task_handle = Arc::new(std::sync::Mutex::new(Some(task_handle)));
        let initialized = Arc::new(std::sync::Mutex::new(true));
        let default_alias = Arc::new(std::sync::Mutex::new(None));
        
        Ok(PyDbQueueBridge {
            request_sender,
            default_alias,
            _task_handle: task_handle,
            initialized,
            runtime,
        })
    }

    /// 创建数据记录
    pub fn create(
        &self,
        table: String,
        data_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let data = self.parse_json_to_data_map(&data_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "create".to_string(),
            collection: table,
            data: Some(data),
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 查找数据记录（智能检测查询类型）
    pub fn find(
        &self,
        table: String,
        query_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        // 智能检测查询类型
        if self.is_condition_groups_query(&query_json) {
            // 如果是条件组合查询（OR/AND逻辑），使用find_with_groups逻辑
            let condition_groups = self.parse_condition_groups_json(&query_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
            
            let (response_sender, response_receiver) = oneshot::channel();
            let request = PyDbRequest {
                request_id: Uuid::new_v4().to_string(),
                operation: "find_with_groups".to_string(),
                collection: table,
                data: None,
                conditions: None,
                condition_groups: Some(condition_groups),
                options: None,
                updates: None,
                id: None,
                alias,
                database_config: None,
                fields: None,
                response_sender,
            };
            
            self.send_request(request)?;
            self.wait_for_response(response_receiver)
        } else {
            // 普通查询条件，使用原有逻辑
            let conditions = self.parse_conditions_json(&query_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
            
            let (response_sender, response_receiver) = oneshot::channel();
            let request = PyDbRequest {
                request_id: Uuid::new_v4().to_string(),
                operation: "find".to_string(),
                collection: table,
                data: None,
                conditions: Some(conditions),
                condition_groups: None,
                options: None,
                updates: None,
                id: None,
                alias,
                database_config: None,
                fields: None,
                response_sender,
            };
            
            self.send_request(request)?;
            self.wait_for_response(response_receiver)
        }
    }

    /// 使用条件组合查找数据记录
    pub fn find_with_groups(
        &self,
        table: String,
        query_groups_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let condition_groups = self.parse_condition_groups_json(&query_groups_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "find_with_groups".to_string(),
            collection: table,
            data: None,
            conditions: None,
            condition_groups: Some(condition_groups),
            options: None,
            updates: None,
            id: None,
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 根据ID查找数据记录
    pub fn find_by_id(&self, table: String, id: String, alias: Option<String>) -> PyResult<String> {
        self.check_initialized()?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "find_by_id".to_string(),
            collection: table,
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: Some(id),
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 删除数据记录
    pub fn delete(
        &self,
        table: String,
        conditions_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let conditions = self.parse_conditions_json(&conditions_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "delete".to_string(),
            collection: table,
            data: None,
            conditions: Some(conditions),
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 根据ID删除数据记录
    pub fn delete_by_id(&self, table: String, id: String, alias: Option<String>) -> PyResult<String> {
        self.check_initialized()?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "delete_by_id".to_string(),
            collection: table,
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: Some(id),
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 更新数据记录
    pub fn update(
        &self,
        table: String,
        conditions_json: String,
        updates_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let conditions = self.parse_conditions_json(&conditions_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        let updates = self.parse_json_to_data_map(&updates_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "update".to_string(),
            collection: table,
            data: None,
            conditions: Some(conditions),
            condition_groups: None,
            options: None,
            updates: Some(updates),
            id: None,
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 根据ID更新数据记录
    pub fn update_by_id(
        &self,
        table: String,
        id: String,
        updates_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let updates = self.parse_json_to_data_map(&updates_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "update_by_id".to_string(),
            collection: table,
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: Some(updates),
            id: Some(id),
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 添加SQLite数据库
    pub fn add_sqlite_database(
        &self,
        alias: String,
        path: String,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
        cache_config: Option<PyCacheConfig>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfigBuilder::new();
        
        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }
        
        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite { 
                path,
                create_if_missing: true,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::AutoIncrement);
        
        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }
        
        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;
        
        self.runtime.block_on(async {
            add_database(database_config).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("添加SQLite数据库失败: {}", e)))
        })?;
        
        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias.clone());
            }
        }
        
        Ok(serde_json::json!({
            "success": true,
            "message": format!("SQLite数据库 '{}' 添加成功", alias)
        }).to_string())
    }

    /// 添加PostgreSQL数据库
    pub fn add_postgresql_database(
        &self,
        alias: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        ssl_mode: Option<String>,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
        cache_config: Option<PyCacheConfig>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfigBuilder::new();
        
        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }
        
        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::PostgreSQL)
            .connection(ConnectionConfig::PostgreSQL {
            host,
            port,
            database,
            username,
            password,
            ssl_mode,
            tls_config: None,
        })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::AutoIncrement);
        
        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }
        
        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;
        
        self.runtime.block_on(async {
            add_database(database_config).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("添加PostgreSQL数据库失败: {}", e)))
        })?;
        
        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias.clone());
            }
        }
        
        Ok(serde_json::json!({
            "success": true,
            "message": format!("PostgreSQL数据库 '{}' 添加成功", alias)
        }).to_string())
    }

    /// 添加MySQL数据库
    pub fn add_mysql_database(
        &self,
        alias: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
        cache_config: Option<PyCacheConfig>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfigBuilder::new();
        
        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }
        
        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MySQL)
            .connection(ConnectionConfig::MySQL {
                host,
                port,
                database,
                username,
                password,
                ssl_opts: None,
                tls_config: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::AutoIncrement);
        
        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }
        
        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;
        
        self.runtime.block_on(async {
            add_database(database_config).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("添加MySQL数据库失败: {}", e)))
        })?;
        
        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias.clone());
            }
        }
        
        Ok(serde_json::json!({
            "success": true,
            "message": format!("MySQL数据库 '{}' 添加成功", alias)
        }).to_string())
    }

    /// 添加MongoDB数据库
    pub fn add_mongodb_database(
        &self,
        alias: String,
        host: String,
        port: u16,
        database: String,
        username: Option<String>,
        password: Option<String>,
        auth_source: Option<String>,
        direct_connection: Option<bool>,
        max_connections: Option<u32>,
        min_connections: Option<u32>,
        connection_timeout: Option<u64>,
        idle_timeout: Option<u64>,
        max_lifetime: Option<u64>,
        cache_config: Option<PyCacheConfig>,
        tls_config: Option<PyTlsConfig>,
        zstd_config: Option<PyZstdConfig>,
    ) -> PyResult<String> {
        let mut pool_config_builder = PoolConfigBuilder::new();
        
        if let Some(max_conn) = max_connections {
            pool_config_builder = pool_config_builder.max_connections(max_conn);
        }
        if let Some(min_conn) = min_connections {
            pool_config_builder = pool_config_builder.min_connections(min_conn);
        }
        if let Some(timeout) = connection_timeout {
            pool_config_builder = pool_config_builder.connection_timeout(timeout);
        }
        if let Some(idle) = idle_timeout {
            pool_config_builder = pool_config_builder.idle_timeout(idle);
        }
        if let Some(lifetime) = max_lifetime {
            pool_config_builder = pool_config_builder.max_lifetime(lifetime);
        }
        
        let pool_config = pool_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建连接池配置失败: {}", e)))?;
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MongoDB)
            .connection(ConnectionConfig::MongoDB {
                host,
                port,
                database,
                username,
                password,
                auth_source,
                direct_connection: direct_connection.unwrap_or(false),
                tls_config: tls_config.map(|cfg| cfg.to_rust_config()),
                zstd_config: zstd_config.map(|cfg| cfg.to_rust_config()),
                options: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::ObjectId);
        
        if let Some(cache_cfg) = cache_config {
            db_config_builder = db_config_builder.cache(cache_cfg.to_rust_config());
        }
        
        let database_config = db_config_builder.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("构建数据库配置失败: {}", e)))?;
        
        self.runtime.block_on(async {
            add_database(database_config).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("添加MongoDB数据库失败: {}", e)))
        })?;
        
        // 如果这是第一个数据库，设置为默认别名
        {
            let mut default_alias_guard = self.default_alias.lock().unwrap();
            if default_alias_guard.is_none() {
                *default_alias_guard = Some(alias.clone());
            }
        }
        
        Ok(serde_json::json!({
            "success": true,
            "message": format!("MongoDB数据库 '{}' 添加成功", alias)
        }).to_string())
    }

    /// 设置默认数据库别名
    pub fn set_default_alias(&self, alias: String) -> PyResult<()> {
        let mut default_alias_guard = self.default_alias.lock().unwrap();
        *default_alias_guard = Some(alias);
        Ok(())
    }

    /// 删除表
    pub fn drop_table(
        &self,
        table: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;

        let request_id = Uuid::new_v4().to_string();
        let (response_sender, response_receiver) = oneshot::channel();

        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "drop_table".to_string(),
            collection: table,
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias,
            database_config: None,
            fields: None,
            response_sender,
        };

        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 创建表
    pub fn create_table(
        &self,
        table: String,
        fields_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let fields = self.parse_fields_json(&fields_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
        
        let (response_sender, response_receiver) = oneshot::channel();
        let request = PyDbRequest {
            request_id: Uuid::new_v4().to_string(),
            operation: "create_table".to_string(),
            collection: table,
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias,
            database_config: None,
            fields: Some(fields),
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }
}

impl PyDbQueueBridge {
    /// 发送请求
    fn send_request(&self, request: PyDbRequest) -> PyResult<()> {
        let sender_guard = self.request_sender.lock().unwrap();
        if let Some(sender) = sender_guard.as_ref() {
            sender.send(request)
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("发送请求失败"))?;
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("请求发送器未初始化"))
        }
    }

    /// 等待响应
    fn wait_for_response(
        &self,
        response_receiver: oneshot::Receiver<PyDbResponse>,
    ) -> PyResult<String> {
        let response = self.runtime.block_on(response_receiver).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("等待响应失败: {}", e))
        })?;
        
        // 构建统一的响应格式，包含success字段
        let unified_response = if response.success {
            // 尝试解析data字段，如果是JSON字符串则解析，否则直接使用
            let data_value = if response.data.starts_with('{') || response.data.starts_with('[') {
                // 尝试解析为JSON值
                serde_json::from_str::<serde_json::Value>(&response.data)
                    .unwrap_or_else(|_| serde_json::Value::String(response.data.clone()))
            } else {
                serde_json::Value::String(response.data)
            };
            
            serde_json::json!({
                "success": true,
                "data": data_value,
                "error": null
            })
        } else {
            serde_json::json!({
                "success": false,
                "data": null,
                "error": response.error.unwrap_or_else(|| "未知错误".to_string())
            })
        };
        
        Ok(unified_response.to_string())
    }

    /// 检查初始化状态
    fn check_initialized(&self) -> PyResult<()> {
        let initialized = self.initialized.lock().unwrap();
        if *initialized {
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("桥接器未初始化"))
        }
    }

    /// 解析JSON字符串为数据映射
    fn parse_json_to_data_map(&self, json_str: &str) -> Result<HashMap<String, DataValue>, String> {
        let parser = crate::parser::Parser;
        parser.parse_json_to_data_map(json_str)
    }

    /// 解析查询条件JSON
    /// 解析查询条件JSON字符串（支持条件组合）
    fn parse_condition_groups_json(&self, json_str: &str) -> Result<Vec<QueryConditionGroup>, String> {
        let parser = crate::parser::Parser;
        parser.parse_condition_groups_json(json_str)
    }

    /// 解析单个条件组合
    fn parse_single_condition_group(&self, json_value: &JsonValue) -> Result<QueryConditionGroup, String> {
        let parser = crate::parser::Parser;
        parser.parse_single_condition_group(json_value)
    }

    /// 从对象解析单个条件
    fn parse_single_condition_from_object(&self, map: &serde_json::Map<String, JsonValue>) -> Result<QueryCondition, String> {
        let parser = crate::parser::Parser;
        parser.parse_single_condition_from_object(map)
    }

    fn parse_conditions_json(&self, json_str: &str) -> Result<Vec<QueryCondition>, String> {
        let parser = crate::parser::Parser;
        parser.parse_conditions_json(json_str)
    }
    
    /// 解析JSON值为DataValue
    fn parse_data_value(&self, json_value: &JsonValue) -> Result<DataValue, String> {
        let parser = crate::parser::Parser;
        parser.parse_data_value(json_value)
    }

    /// 解析字段定义JSON
    fn parse_fields_json(&self, json_str: &str) -> Result<HashMap<String, rat_quickdb::model::FieldType>, String> {
        // 简单的JSON解析，将字段名映射到FieldType
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
        let mut fields = HashMap::new();
        
        if let JsonValue::Object(obj) = json_value {
            for (field_name, field_def) in obj {
                let field_type = self.parse_field_type(&field_def)?;
                fields.insert(field_name, field_type);
            }
        } else {
            return Err("字段定义必须是JSON对象".to_string());
        }
        
        Ok(fields)
    }
    
    /// 解析单个字段类型
    fn parse_field_type(&self, json_value: &JsonValue) -> Result<rat_quickdb::model::FieldType, String> {
        use rat_quickdb::model::FieldType;
        
        match json_value {
            JsonValue::String(type_str) => {
                match type_str.as_str() {
                    "string" => Ok(FieldType::String { max_length: None, min_length: None, regex: None }),
                    "integer" => Ok(FieldType::Integer { min_value: None, max_value: None }),
                    "biginteger" => Ok(FieldType::BigInteger),
                    "float" => Ok(FieldType::Float { min_value: None, max_value: None }),
                    "double" => Ok(FieldType::Double),
                    "text" => Ok(FieldType::Text),
                    "boolean" => Ok(FieldType::Boolean),
                    "datetime" => Ok(FieldType::DateTime),
                    "date" => Ok(FieldType::Date),
                    "time" => Ok(FieldType::Time),
                    "uuid" => Ok(FieldType::Uuid),
                    "json" => Ok(FieldType::Json),
                    "binary" => Ok(FieldType::Binary),
                    _ => Err(format!("不支持的字段类型: {}", type_str)),
                }
            },
            JsonValue::Object(obj) => {
                if let Some(type_name) = obj.get("type").and_then(|v| v.as_str()) {
                    match type_name {
                        "array" => {
                            if let Some(item_type_json) = obj.get("item_type") {
                                let item_type = self.parse_field_type(item_type_json)?;
                                let max_items = obj.get("max_items").and_then(|v| v.as_u64()).map(|v| v as usize);
                                let min_items = obj.get("min_items").and_then(|v| v.as_u64()).map(|v| v as usize);
                                Ok(FieldType::Array {
                                    item_type: Box::new(item_type),
                                    max_items,
                                    min_items,
                                })
                            } else {
                                Err("数组类型缺少item_type定义".to_string())
                            }
                        },
                        _ => Err(format!("不支持的复合字段类型: {}", type_name)),
                    }
                } else {
                    Err("字段定义对象缺少type字段".to_string())
                }
            },
            _ => Err("字段类型定义必须是字符串或对象".to_string()),
        }
    }
    
    /// 检测是否为条件组合查询（包含operator字段的OR/AND逻辑查询）
    fn is_condition_groups_query(&self, json_str: &str) -> bool {
        if let Ok(json_value) = serde_json::from_str::<JsonValue>(json_str) {
            match json_value {
                // 检查单个对象是否包含operator字段
                JsonValue::Object(ref obj) => {
                    obj.contains_key("operator") && obj.contains_key("conditions")
                },
                // 检查数组中是否有任何元素包含operator字段
                JsonValue::Array(ref arr) => {
                    arr.iter().any(|item| {
                        if let JsonValue::Object(ref obj) = item {
                            obj.contains_key("operator") && obj.contains_key("conditions")
                        } else {
                            false
                        }
                    })
                },
                _ => false,
            }
        } else {
            false
        }
    }
}

/// 异步数据库队列桥接器
/// 处理异步数据库操作请求
struct PyDbQueueBridgeAsync;

impl PyDbQueueBridgeAsync {
    /// 守护进程任务
    async fn daemon_task(mut request_receiver: mpsc::UnboundedReceiver<PyDbRequest>) {
        info!("数据库队列桥接器守护进程已启动");
        
        while let Some(request) = request_receiver.recv().await {
            debug!("收到数据库操作请求: {}", request.operation);
            
            let response = Self::process_request(&request).await;
            
            if let Err(e) = request.response_sender.send(response) {
                error!("发送响应失败: {:?}", e);
            }
        }
        
        info!("数据库队列桥接器守护进程已停止");
    }

    /// 处理请求
    async fn process_request(request: &PyDbRequest) -> PyDbResponse {
        let result = match request.operation.as_str() {
            "create" => {
                if let Some(ref data) = request.data {
                    Self::handle_create_direct(&request.collection, data.clone(), request.alias.clone()).await
                        .map(|id| id)
                } else {
                    Err(QuickDbError::ValidationError { field: "data".to_string(), message: "缺少数据字段".to_string() })
                }
            },
            "find_by_id" => {
                if let Some(ref id) = request.id {
                    Self::handle_find_by_id_direct(&request.collection, id, request.alias.clone()).await
                        .map(|opt| opt.unwrap_or_else(|| "null".to_string()))
                } else {
                    Err(QuickDbError::ValidationError { field: "id".to_string(), message: "缺少ID字段".to_string() })
                }
            },
            "find" => {
                if let Some(ref conditions) = request.conditions {
                    Self::handle_find_direct(&request.collection, conditions.clone(), request.options.clone(), request.alias.clone()).await
                } else {
                    Err(QuickDbError::ValidationError { field: "conditions".to_string(), message: "缺少查询条件".to_string() })
                }
            },
            "find_with_groups" => {
                if let Some(ref condition_groups) = request.condition_groups {
                    Self::handle_find_with_groups_direct(&request.collection, condition_groups.clone(), request.options.clone(), request.alias.clone()).await
                } else {
                    Err(QuickDbError::ValidationError { field: "condition_groups".to_string(), message: "缺少查询条件组合".to_string() })
                }
            },
            "delete" => {
                if let Some(ref conditions) = request.conditions {
                    Self::handle_delete_direct(&request.collection, conditions.clone(), request.alias.clone()).await
                        .map(|count| count.to_string())
                } else {
                    Err(QuickDbError::ValidationError { field: "conditions".to_string(), message: "缺少删除条件".to_string() })
                }
            },
            "delete_by_id" => {
                if let Some(ref id) = request.id {
                    Self::handle_delete_by_id_direct(&request.collection, id, request.alias.clone()).await
                        .map(|success| success.to_string())
                } else {
                    Err(QuickDbError::ValidationError { field: "id".to_string(), message: "缺少ID字段".to_string() })
                }
            },
            "update" => {
                if let (Some(ref conditions), Some(ref updates)) = (&request.conditions, &request.updates) {
                    Self::handle_update_direct(&request.collection, conditions.clone(), updates.clone(), request.alias.clone()).await
                        .map(|count| count.to_string())
                } else {
                    Err(QuickDbError::ValidationError { field: "conditions_or_data".to_string(), message: "缺少更新条件或更新数据".to_string() })
                }
            },
            "update_by_id" => {
                if let (Some(ref id), Some(ref updates)) = (&request.id, &request.updates) {
                    Self::handle_update_by_id_direct(&request.collection, id, updates.clone(), request.alias.clone()).await
                        .map(|success| success.to_string())
                } else {
                    Err(QuickDbError::ValidationError { field: "id_or_data".to_string(), message: "缺少ID字段或更新数据".to_string() })
                }
            },
            "drop_table" => {
                Self::handle_drop_table_direct(&request.collection, request.alias.clone()).await
                    .map(|success| success.to_string())
            },
            "create_table" => {
                if let Some(ref fields) = request.fields {
                    Self::handle_create_table_direct(&request.collection, fields.clone(), request.alias.clone()).await
                        .map(|success| success.to_string())
                } else {
                    Err(QuickDbError::ValidationError { field: "fields".to_string(), message: "缺少字段定义".to_string() })
                }
            },
            _ => Err(QuickDbError::ValidationError { field: "operation".to_string(), message: format!("不支持的操作: {}", request.operation) })
        };
        
        match result {
            Ok(data) => PyDbResponse {
                request_id: request.request_id.clone(),
                success: true,
                data,
                error: None,
            },
            Err(e) => {
                error!("数据库操作失败: {}", e);
                PyDbResponse {
                    request_id: request.request_id.clone(),
                    success: false,
                    data: String::new(),
                    error: Some(e.to_string()),
                }
            }
        }
    }

    /// 处理创建操作
    async fn handle_create_direct(
        collection: &str,
        data: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<String> {
        let odm_manager = get_odm_manager().await;
        let alias_ref = alias.as_deref();
        let created_id = odm_manager.create(collection, data, alias_ref).await?;
        
        let json_str = serde_json::to_string(&created_id)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化结果失败: {}", e) })?;
        Ok(json_str)
    }

    /// 处理根据ID查找操作
    async fn handle_find_by_id_direct(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<Option<String>> {
        let odm_manager = get_odm_manager().await;
        let alias_ref = alias.as_deref();
        let result = odm_manager.find_by_id(collection, id, alias_ref).await?;
        
        match result {
            Some(data) => {
                // 将DataValue转换为JsonValue，避免Object包装
                let json_value = data.to_json_value();
                let json_str = serde_json::to_string(&json_value)
                    .map_err(|e| QuickDbError::SerializationError { message: format!("序列化结果失败: {}", e) })?;
                Ok(Some(json_str))
            },
            None => Ok(None)
        }
    }

    /// 处理查找操作
    async fn handle_find_direct(
        collection: &str,
        conditions: Vec<QueryCondition>,
        options: Option<QueryOptions>,
        alias: Option<String>,
    ) -> QuickDbResult<String> {
        let odm_manager = get_odm_manager().await;
        let alias_ref = alias.as_deref();
        let results = odm_manager.find(collection, conditions, options, alias_ref).await?;
        
        // 将DataValue数组转换为JsonValue数组，避免Object包装
        let json_array: Vec<serde_json::Value> = results.iter()
            .map(|data_value| data_value.to_json_value())
            .collect();
        
        let json_str = serde_json::to_string(&json_array)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化结果失败: {}", e) })?;
        
        Ok(json_str)
    }

    /// 处理使用条件组合查找操作
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
        let results = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;

        // 将DataValue结果转换为JsonValue，避免Object包装问题
        let json_array: Vec<JsonValue> = results.into_iter()
            .map(|data_value| data_value.to_json_value())
            .collect();
        
        let json_str = serde_json::to_string(&json_array)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化结果失败: {}", e) })?;
        
        Ok(json_str)
    }

    /// 处理删除操作
    async fn handle_delete_direct(
        collection: &str,
        conditions: Vec<QueryCondition>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let odm_manager = get_odm_manager().await;
        let alias_ref = alias.as_deref();
        let deleted_count = odm_manager.delete(collection, conditions, alias_ref).await?;
        
        Ok(deleted_count)
    }

    /// 处理根据ID删除操作
    async fn handle_delete_by_id_direct(
        collection: &str,
        id: &str,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        let odm_manager = get_odm_manager().await;
        let alias_ref = alias.as_deref();
        let success = odm_manager.delete_by_id(collection, id, alias_ref).await?;
        
        Ok(success)
    }

    /// 处理更新操作
    async fn handle_update_direct(
        collection: &str,
        conditions: Vec<QueryCondition>,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<u64> {
        let odm_manager = get_odm_manager().await;
        let alias_ref = alias.as_deref();
        let updated_count = odm_manager.update(collection, conditions, updates, alias_ref).await?;
        
        Ok(updated_count)
    }

    /// 处理根据ID更新操作
    async fn handle_update_by_id_direct(
        collection: &str,
        id: &str,
        updates: HashMap<String, DataValue>,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        let odm_manager = get_odm_manager().await;
        let alias_ref = alias.as_deref();
        let result = odm_manager.update_by_id(collection, id, updates, alias_ref).await?;
        
        Ok(result)
    }

    /// 处理删除表操作
    async fn handle_drop_table_direct(
        table: &str,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理删除表请求: table={}, alias={}", table, actual_alias);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::DropTable请求到连接池
        let operation = DatabaseOperation::DropTable {
            table: table.to_string(),
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let _result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        // drop_table 成功执行返回 true
        Ok(true)
    }

    /// 处理创建表操作
    async fn handle_create_table_direct(
        table: &str,
        fields: HashMap<String, rat_quickdb::model::FieldType>,
        alias: Option<String>,
    ) -> QuickDbResult<bool> {
        use rat_quickdb::pool::DatabaseOperation;
        use tokio::sync::oneshot;
        
        let actual_alias = alias.unwrap_or_else(|| "default".to_string());
        debug!("直接处理创建表请求: table={}, alias={}, fields={:?}", table, actual_alias, fields);
        
        let manager = get_global_pool_manager();
        let connection_pools = manager.get_connection_pools();
        let connection_pool = connection_pools.get(&actual_alias)
            .ok_or_else(|| QuickDbError::AliasNotFound {
                alias: actual_alias.clone(),
            })?;
        
        // 创建oneshot通道用于接收响应
        let (response_tx, response_rx) = oneshot::channel();
        
        // 发送DatabaseOperation::CreateTable请求到连接池
        let operation = DatabaseOperation::CreateTable {
            table: table.to_string(),
            fields,
            response: response_tx,
        };
        
        connection_pool.operation_sender.send(operation)
            .map_err(|_| QuickDbError::ConnectionError {
                message: "连接池操作通道已关闭".to_string(),
            })?;
        
        // 等待响应
        let _result = response_rx.await
            .map_err(|_| QuickDbError::ConnectionError {
                message: "等待连接池响应超时".to_string(),
            })??;
        
        // create_table 成功执行返回 true
        Ok(true)
    }
}

/// 创建数据库队列桥接器
#[pyfunction]
pub fn create_db_queue_bridge() -> PyResult<PyDbQueueBridge> {
    PyDbQueueBridge::new()
}