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

// 导入模型绑定模块
mod model_bindings;
use model_bindings::*;

/// 获取版本信息
#[pyfunction]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 获取库信息
#[pyfunction]
fn get_info() -> String {
    "Rat QuickDB Python Bindings".to_string()
}

/// 获取库名称
#[pyfunction]
fn get_name() -> String {
    "rat_quickdb_py".to_string()
}

/// Python请求消息结构
#[derive(Debug, Clone)]
pub struct PyRequestMessage {
    pub id: String,
    pub request_type: String,
    pub data: String,
}

/// Python响应消息结构
#[derive(Debug, Clone)]
pub struct PyResponseMessage {
    pub id: String,
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
}

/// 数据库请求结构
#[derive(Debug)]
pub struct PyDbRequest {
    /// 请求ID
    pub request_id: String,
    /// 操作类型
    pub operation: String,
    /// 集合名称
    pub collection: String,
    /// 数据
    pub data: Option<HashMap<String, DataValue>>,
    /// 查询条件
    pub conditions: Option<Vec<QueryCondition>>,
    /// 查询条件组合（支持OR逻辑）
    pub condition_groups: Option<Vec<QueryConditionGroup>>,
    /// 查询选项
    pub options: Option<QueryOptions>,
    /// 更新数据
    pub updates: Option<HashMap<String, DataValue>>,
    /// ID
    pub id: Option<String>,
    /// 数据库别名
    pub alias: Option<String>,
    /// 数据库配置（用于add_database操作）
    pub database_config: Option<DatabaseConfig>,
    /// 响应发送器
    pub response_sender: oneshot::Sender<PyDbResponse>,
}

/// 数据库响应结构
#[derive(Debug, Clone)]
pub struct PyDbResponse {
    /// 请求ID
    pub request_id: String,
    /// 是否成功
    pub success: bool,
    /// 响应数据
    pub data: String,
    /// 错误信息
    pub error: Option<String>,
}

/// 缓存配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyCacheConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub strategy: String,
    #[pyo3(get, set)]
    pub l1_config: Option<PyL1CacheConfig>,
    #[pyo3(get, set)]
    pub l2_config: Option<PyL2CacheConfig>,
    #[pyo3(get, set)]
    pub ttl_config: Option<PyTtlConfig>,
    #[pyo3(get, set)]
    pub compression_config: Option<PyCompressionConfig>,
}

/// L1缓存配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyL1CacheConfig {
    #[pyo3(get, set)]
    pub max_capacity: usize,
    #[pyo3(get, set)]
    pub max_memory_mb: usize,
    #[pyo3(get, set)]
    pub enable_stats: bool,
}

/// L2缓存配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyL2CacheConfig {
    #[pyo3(get, set)]
    pub storage_path: String,
    #[pyo3(get, set)]
    pub max_disk_mb: usize,
    #[pyo3(get, set)]
    pub compression_level: i32,
    #[pyo3(get, set)]
    pub enable_wal: bool,
    #[pyo3(get, set)]
    pub clear_on_startup: bool,
}

/// TTL配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyTtlConfig {
    #[pyo3(get, set)]
    pub default_ttl_secs: u64,
    #[pyo3(get, set)]
    pub max_ttl_secs: u64,
    #[pyo3(get, set)]
    pub check_interval_secs: u64,
}

/// 压缩配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyCompressionConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub algorithm: String,
    #[pyo3(get, set)]
    pub threshold_bytes: usize,
}

/// TLS配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyTlsConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub ca_cert_path: Option<String>,
    #[pyo3(get, set)]
    pub client_cert_path: Option<String>,
    #[pyo3(get, set)]
    pub client_key_path: Option<String>,
    #[pyo3(get, set)]
    pub verify_server_cert: bool,
    #[pyo3(get, set)]
    pub verify_hostname: bool,
    #[pyo3(get, set)]
    pub min_tls_version: Option<String>,
    #[pyo3(get, set)]
    pub cipher_suites: Option<Vec<String>>,
}

/// ZSTD压缩配置
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyZstdConfig {
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get, set)]
    pub compression_level: Option<i32>,
    #[pyo3(get, set)]
    pub compression_threshold: Option<usize>,
}

#[pymethods]
impl PyCacheConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            enabled: false,
            strategy: "L1Only".to_string(),
            l1_config: None,
            l2_config: None,
            ttl_config: None,
            compression_config: None,
        }
    }

    /// 启用缓存
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用缓存
    pub fn disable(&mut self) {
        self.enabled = false;
    }


}

#[pymethods]
impl PyL1CacheConfig {
    #[new]
    pub fn new(max_capacity: usize) -> Self {
        Self {
            max_capacity,
            max_memory_mb: 100,
            enable_stats: false,
        }
    }
}

impl PyTlsConfig {
    /// 转换为Rust的TlsConfig
    pub fn to_rust_config(&self) -> TlsConfig {
        TlsConfig {
            enabled: self.enabled,
            ca_cert_path: self.ca_cert_path.clone(),
            client_cert_path: self.client_cert_path.clone(),
            client_key_path: self.client_key_path.clone(),
            verify_server_cert: self.verify_server_cert,
            verify_hostname: self.verify_hostname,
            min_tls_version: self.min_tls_version.clone(),
            cipher_suites: self.cipher_suites.clone(),
        }
    }
}

impl PyZstdConfig {
    /// 转换为Rust的ZstdConfig
    pub fn to_rust_config(&self) -> ZstdConfig {
        ZstdConfig {
            enabled: self.enabled,
            compression_level: self.compression_level,
            compression_threshold: self.compression_threshold,
        }
    }
}

impl PyCacheConfig {
    /// 转换为Rust的CacheConfig
    pub fn to_rust_config(&self) -> CacheConfig {
        let mut config = CacheConfig::default();
        config.enabled = self.enabled;
        
        // 设置缓存策略
        config.strategy = match self.strategy.as_str() {
            "lru" => CacheStrategy::Lru,
            "lfu" => CacheStrategy::Lfu,
            "fifo" => CacheStrategy::Fifo,
            _ => CacheStrategy::Lru,
        };
        
        // 设置L1缓存配置（必需字段）
        config.l1_config = if let Some(ref l1_config) = self.l1_config {
            L1CacheConfig {
                max_capacity: l1_config.max_capacity,
                max_memory_mb: l1_config.max_memory_mb,
                enable_stats: l1_config.enable_stats,
            }
        } else {
            L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: false,
            }
        };
        
        // 设置L2缓存配置（可选字段）
        config.l2_config = if let Some(ref l2_config) = self.l2_config {
            Some(L2CacheConfig {
                storage_path: l2_config.storage_path.clone(),
                max_disk_mb: l2_config.max_disk_mb,
                compression_level: l2_config.compression_level,
                enable_wal: l2_config.enable_wal,
                clear_on_startup: l2_config.clear_on_startup,
            })
        } else {
            None
        };
        
        // 设置TTL配置（必需字段）
        config.ttl_config = if let Some(ref ttl_config) = self.ttl_config {
            TtlConfig {
                default_ttl_secs: ttl_config.default_ttl_secs,
                max_ttl_secs: ttl_config.max_ttl_secs,
                check_interval_secs: ttl_config.check_interval_secs,
            }
        } else {
            TtlConfig {
                default_ttl_secs: 3600,
                max_ttl_secs: 7200,
                check_interval_secs: 300,
            }
        };
        
        // 设置压缩配置（必需字段）
        config.compression_config = if let Some(ref compression_config) = self.compression_config {
            CompressionConfig {
                enabled: compression_config.enabled,
                algorithm: match compression_config.algorithm.as_str() {
                    "gzip" => CompressionAlgorithm::Gzip,
                    "zstd" => CompressionAlgorithm::Zstd,
                    "lz4" => CompressionAlgorithm::Lz4,
                    _ => CompressionAlgorithm::Gzip,
                },
                threshold_bytes: compression_config.threshold_bytes,
            }
        } else {
            CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::Gzip,
                threshold_bytes: 1024,
            }
        };
        
        config
    }
}

#[pymethods]
impl PyL2CacheConfig {
    #[new]
    pub fn new(storage_path: String) -> Self {
        Self {
            storage_path,
            max_disk_mb: 1000,
            compression_level: 3,
            enable_wal: true,
            clear_on_startup: false,
        }
    }
}

#[pymethods]
impl PyTtlConfig {
    #[new]
    pub fn new(default_ttl_secs: u64) -> Self {
        Self {
            default_ttl_secs,
            max_ttl_secs: default_ttl_secs * 2,
            check_interval_secs: 300,
        }
    }
}

#[pymethods]
impl PyCompressionConfig {
    #[new]
    pub fn new(algorithm: String) -> Self {
        Self {
            enabled: false,
            algorithm,
            threshold_bytes: 1024,
        }
    }
}

#[pymethods]
impl PyTlsConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            enabled: false,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            verify_server_cert: true,
            verify_hostname: true,
            min_tls_version: Some("1.2".to_string()),
            cipher_suites: None,
        }
    }
    
    /// 启用TLS
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// 禁用TLS
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

#[pymethods]
impl PyZstdConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            enabled: false,
            compression_level: Some(3),
            compression_threshold: Some(1024),
        }
    }
    
    /// 启用ZSTD压缩
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// 禁用ZSTD压缩
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

/// Python数据库队列桥接器
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
    /// 共享的Tokio运行时，避免每次查询都创建新的Runtime
    runtime: Arc<Runtime>,
}

#[pymethods]
impl PyDbQueueBridge {
    /// 创建新的数据库队列桥接器
    #[new]
    pub fn new() -> PyResult<Self> {
        info!("创建新的数据库队列桥接器");
        let (request_sender, request_receiver) = mpsc::unbounded_channel::<PyDbRequest>();

        // 创建共享的Tokio运行时
        let runtime = Arc::new(Runtime::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("创建运行时失败: {}", e))
        })?);

        // 启动后台守护任务线程
        info!("启动后台守护任务线程");
        let task_handle = thread::spawn(move || {
            info!("守护任务线程开始运行");
            let rt = Runtime::new().expect("创建Tokio运行时失败");
            rt.block_on(async {
                info!("守护任务异步运行时启动");
                PyDbQueueBridgeAsync::daemon_task(request_receiver).await;
                info!("守护任务异步运行时结束");
            });
            info!("守护任务线程结束");
        });

        // 等待一小段时间确保守护任务启动
        thread::sleep(Duration::from_millis(100));

        let bridge = PyDbQueueBridge {
            request_sender: Arc::new(std::sync::Mutex::new(Some(request_sender))),
            default_alias: Arc::new(std::sync::Mutex::new(None)),
            _task_handle: Arc::new(std::sync::Mutex::new(Some(task_handle))),
            initialized: Arc::new(std::sync::Mutex::new(true)),
            runtime,
        };

        info!("数据库队列桥接器创建完成");
        Ok(bridge)
    }

    /// 创建数据
    pub fn create(
        &self,
        table: String,
        data_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        let data = self.parse_json_to_data_map(&data_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析数据失败: {}", e)))?;
        
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
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 查询数据
    pub fn find(
        &self,
        table: String,
        query_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        let conditions = self.parse_conditions_json(&query_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件失败: {}", e)))?;
        
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
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 使用条件组合查询数据（支持OR逻辑）
    pub fn find_with_groups(
        &self,
        table: String,
        query_groups_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        let condition_groups = self.parse_condition_groups_json(&query_groups_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件组合失败: {}", e)))?;
        
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
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 根据ID查询数据
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
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 删除记录
    pub fn delete(
        &self,
        table: String,
        conditions_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let conditions = self.parse_conditions_json(&conditions_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件失败: {}", e)))?;
        
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
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 根据ID删除记录
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
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 更新记录
    pub fn update(
        &self,
        table: String,
        conditions_json: String,
        updates_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let conditions = self.parse_conditions_json(&conditions_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析查询条件失败: {}", e)))?;
        
        let updates = self.parse_json_to_data_map(&updates_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析更新数据失败: {}", e)))?;
        
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
            response_sender,
        };
        
        self.send_request(request)?;
        self.wait_for_response(response_receiver)
    }

    /// 按ID更新记录
    pub fn update_by_id(
        &self,
        table: String,
        id: String,
        updates_json: String,
        alias: Option<String>,
    ) -> PyResult<String> {
        self.check_initialized()?;
        
        let updates = self.parse_json_to_data_map(&updates_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("解析更新数据失败: {}", e)))?;
        
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
        info!("添加SQLite数据库: alias={}, path={}", alias, path);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::SQLite)
            .connection(ConnectionConfig::SQLite { 
                path: path.clone(),
                create_if_missing: true,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid);
        
        // 添加缓存配置（只有当缓存启用时才添加）
        if let Some(cache_config) = cache_config {
            if cache_config.enabled {
                db_config_builder = db_config_builder.cache(cache_config.to_rust_config());
            }
            // 如果 enabled=false，则不添加缓存配置，相当于 cache=None
        }
        
        let db_config = match db_config_builder.build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库，确保在正确的运行时中启动
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(), // 不需要集合名
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        // 发送请求到守护任务
        self.send_request(request)?;
        
        // 等待响应
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            // 更新默认别名
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
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
        info!("添加PostgreSQL数据库: alias={}, host={}, port={}, database={}", alias, host, port, database);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::PostgreSQL)
            .connection(ConnectionConfig::PostgreSQL { 
                host: host.clone(),
                port,
                database: database.clone(),
                username: username.clone(),
                password: password.clone(),
                ssl_mode: ssl_mode.or_else(|| Some("prefer".to_string())),
                tls_config: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid);
        
        // 添加缓存配置
        if let Some(cache_config) = cache_config {
            db_config_builder = db_config_builder.cache(cache_config.to_rust_config());
        }
        
        let db_config = match db_config_builder.build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(),
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        self.send_request(request)?;
        
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
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
        info!("添加MySQL数据库: alias={}, host={}, port={}, database={}", alias, host, port, database);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MySQL)
            .connection(ConnectionConfig::MySQL { 
                host: host.clone(),
                port,
                database: database.clone(),
                username: username.clone(),
                password: password.clone(),
                ssl_opts: None,
                tls_config: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid);
        
        // 添加缓存配置
         if let Some(cache_config) = cache_config {
             db_config_builder = db_config_builder.cache(cache_config.to_rust_config());
         }
        
        let db_config = match db_config_builder.build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(),
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        self.send_request(request)?;
        
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
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
        info!("添加MongoDB数据库: alias={}, host={}, port={}, database={}", alias, host, port, database);
        
        let pool_config = match PoolConfigBuilder::new()
            .max_connections(max_connections.unwrap_or(10))
            .min_connections(min_connections.unwrap_or(1))
            .connection_timeout(connection_timeout.unwrap_or(30))
            .idle_timeout(idle_timeout.unwrap_or(600))
            .max_lifetime(max_lifetime.unwrap_or(3600))
            .build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("连接池配置构建失败: {}", e)
                    ));
                }
            };
        
        let mut db_config_builder = DatabaseConfigBuilder::new()
            .db_type(DatabaseType::MongoDB)
            .connection(ConnectionConfig::MongoDB { 
                host: host.clone(),
                port,
                database: database.clone(),
                username,
                password,
                auth_source,
                direct_connection: direct_connection.unwrap_or(false),
                tls_config: tls_config.map(|config| config.to_rust_config()),
                zstd_config: zstd_config.map(|config| config.to_rust_config()),
                options: None,
            })
            .pool(pool_config)
            .alias(alias.clone())
            .id_strategy(IdStrategy::Uuid);
        
        // 添加缓存配置（只有当缓存启用时才添加）
        if let Some(cache_config) = cache_config {
            if cache_config.enabled {
                db_config_builder = db_config_builder.cache(cache_config.to_rust_config());
            }
            // 如果 enabled=false，则不添加缓存配置，相当于 cache=None
        }
        
        let db_config = match db_config_builder.build() {
                Ok(config) => config,
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("数据库配置构建失败: {}", e)
                    ));
                }
            };
        
        // 通过守护任务添加数据库
        let (response_tx, response_rx) = oneshot::channel();
        let request_id = Uuid::new_v4().to_string();
        
        let request = PyDbRequest {
            request_id: request_id.clone(),
            operation: "add_database".to_string(),
            collection: "".to_string(),
            data: None,
            conditions: None,
            condition_groups: None,
            options: None,
            updates: None,
            id: None,
            alias: Some(alias.clone()),
            database_config: Some(db_config),
            response_sender: response_tx,
        };
        
        self.send_request(request)?;
        
        let response_json = self.wait_for_response(response_rx)?;
        let response: serde_json::Value = serde_json::from_str(&response_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("解析响应失败: {}", e)
            ))?;
        
        if response["success"].as_bool().unwrap_or(false) {
            *self.default_alias.lock().unwrap() = Some(alias.clone());
            Ok(response_json)
        } else {
            let error_msg = response["error"].as_str().unwrap_or("未知错误");
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg.to_string()))
        }
    }

    /// 设置默认别名
    pub fn set_default_alias(&self, alias: String) -> PyResult<()> {
        let mut default_alias = self.default_alias.lock().unwrap();
        *default_alias = Some(alias);
        Ok(())
    }
}

/// 异步数据库队列桥接器实现
struct PyDbQueueBridgeAsync;

impl PyDbQueueBridgeAsync {
    /// 守护任务主循环
    async fn daemon_task(mut request_receiver: mpsc::UnboundedReceiver<PyDbRequest>) {
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
        
        Ok(serde_json::to_string(&result)
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
            Ok(Some(serde_json::to_string(&value)
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
        
        serde_json::to_string(&result)
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
        
        serde_json::to_string(&result)
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

/// PyDbQueueBridge的辅助方法实现
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

    /// 解析JSON到数据映射
    fn parse_json_to_data_map(&self, json_str: &str) -> Result<HashMap<String, DataValue>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
        if let JsonValue::Object(map) = json_value {
            let mut data_map = HashMap::new();
            for (key, value) in map {
                let data_value = self.parse_data_value(&value)?;
                data_map.insert(key, data_value);
            }
            Ok(data_map)
        } else {
            Err("JSON必须是对象类型".to_string())
        }
    }

    /// 解析查询条件JSON
    /// 解析查询条件JSON字符串（支持条件组合）
    fn parse_condition_groups_json(&self, json_str: &str) -> Result<Vec<QueryConditionGroup>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;

        match json_value {
            JsonValue::Array(arr) => {
                let mut groups = Vec::new();
                for item in arr {
                    let group = self.parse_single_condition_group(&item)?;
                    groups.push(group);
                }
                Ok(groups)
            }
            _ => {
                let group = self.parse_single_condition_group(&json_value)?;
                Ok(vec![group])
            }
        }
    }

    /// 解析单个条件组合
    fn parse_single_condition_group(&self, json_value: &JsonValue) -> Result<QueryConditionGroup, String> {
        if let JsonValue::Object(ref obj) = json_value {
            // 检查是否是逻辑组合
            if let (Some(operator), Some(conditions)) = (obj.get("operator"), obj.get("conditions")) {
                let logical_op = match operator.as_str() {
                    Some("and") | Some("AND") | Some("And") => LogicalOperator::And,
                    Some("or") | Some("OR") | Some("Or") => LogicalOperator::Or,
                    _ => return Err("不支持的逻辑操作符，支持: and, or".to_string())
                };

                if let JsonValue::Array(conditions_array) = conditions {
                    let mut condition_groups = Vec::new();
                    for condition_item in conditions_array {
                        let group = self.parse_single_condition_group(condition_item)?;
                        condition_groups.push(group);
                    }
                    return Ok(QueryConditionGroup::Group {
                        operator: logical_op,
                        conditions: condition_groups,
                    });
                }
            }

            // 解析为单个条件
            let condition = self.parse_single_condition_from_object(obj)?;
            Ok(QueryConditionGroup::Single(condition))
        } else {
            Err("查询条件必须是对象类型".to_string())
        }
    }

    /// 从对象解析单个条件
    fn parse_single_condition_from_object(&self, map: &serde_json::Map<String, JsonValue>) -> Result<QueryCondition, String> {
        let field = map.get("field")
            .and_then(|v| v.as_str())
            .ok_or("field字段必须是字符串")?;
        
        let operator_str = map.get("operator")
            .and_then(|v| v.as_str())
            .ok_or("operator字段必须是字符串")?;
        
        let operator = match operator_str.to_lowercase().as_str() {
            "eq" => QueryOperator::Eq,
            "ne" => QueryOperator::Ne,
            "gt" => QueryOperator::Gt,
            "gte" => QueryOperator::Gte,
            "lt" => QueryOperator::Lt,
            "lte" => QueryOperator::Lte,
            "contains" => QueryOperator::Contains,
            "startswith" | "starts_with" => QueryOperator::StartsWith,
            "endswith" | "ends_with" => QueryOperator::EndsWith,
            "in" => QueryOperator::In,
            "notin" | "not_in" => QueryOperator::NotIn,
            "regex" => QueryOperator::Regex,
            "exists" => QueryOperator::Exists,
            "isnull" | "is_null" => QueryOperator::IsNull,
            "isnotnull" | "is_not_null" => QueryOperator::IsNotNull,
            _ => return Err(format!("不支持的操作符: {}", operator_str)),
        };
        
        let value = self.parse_data_value(map.get("value").ok_or("缺少value字段")?)?;
        
        Ok(QueryCondition {
            field: field.to_string(),
            operator,
            value,
        })
    }

    fn parse_conditions_json(&self, json_str: &str) -> Result<Vec<QueryCondition>, String> {
        let json_value: JsonValue = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON解析失败: {}", e))?;
        
        match json_value {
            // 支持单个查询条件对象格式: {"field": "name", "operator": "Eq", "value": "张三"}
            JsonValue::Object(ref map) if map.contains_key("field") && map.contains_key("operator") && map.contains_key("value") => {
                let field = map.get("field")
                    .and_then(|v| v.as_str())
                    .ok_or("field字段必须是字符串")?;
                
                let operator_str = map.get("operator")
                    .and_then(|v| v.as_str())
                    .ok_or("operator字段必须是字符串")?;
                
                let operator = match operator_str.to_lowercase().as_str() {
                    "eq" => QueryOperator::Eq,
                    "ne" => QueryOperator::Ne,
                    "gt" => QueryOperator::Gt,
                    "gte" => QueryOperator::Gte,
                    "lt" => QueryOperator::Lt,
                    "lte" => QueryOperator::Lte,
                    "contains" => QueryOperator::Contains,
                    "startswith" | "starts_with" => QueryOperator::StartsWith,
                    "endswith" | "ends_with" => QueryOperator::EndsWith,
                    "in" => QueryOperator::In,
                    "notin" | "not_in" => QueryOperator::NotIn,
                    "regex" => QueryOperator::Regex,
                    "exists" => QueryOperator::Exists,
                    "isnull" | "is_null" => QueryOperator::IsNull,
                    "isnotnull" | "is_not_null" => QueryOperator::IsNotNull,
                    _ => return Err(format!("不支持的操作符: {}", operator_str)),
                };
                
                let value = self.parse_data_value(map.get("value").unwrap())?;
                
                let condition = QueryCondition {
                    field: field.to_string(),
                    operator,
                    value,
                };
                
                Ok(vec![condition])
            },
            // 支持多个查询条件数组格式: [{"field": "name", "operator": "Eq", "value": "张三"}, {"field": "age", "operator": "Gt", "value": 20}]
            JsonValue::Array(conditions_array) => {
                let mut conditions = Vec::new();
                
                for condition_value in conditions_array {
                    if let JsonValue::Object(ref condition_map) = condition_value {
                        let field = condition_map.get("field")
                            .and_then(|v| v.as_str())
                            .ok_or("每个查询条件的field字段必须是字符串")?;
                        
                        let operator_str = condition_map.get("operator")
                            .and_then(|v| v.as_str())
                            .ok_or("每个查询条件的operator字段必须是字符串")?;
                        
                        let operator = match operator_str.to_lowercase().as_str() {
                            "eq" => QueryOperator::Eq,
                            "ne" => QueryOperator::Ne,
                            "gt" => QueryOperator::Gt,
                            "gte" => QueryOperator::Gte,
                            "lt" => QueryOperator::Lt,
                            "lte" => QueryOperator::Lte,
                            "contains" => QueryOperator::Contains,
                            "startswith" | "starts_with" => QueryOperator::StartsWith,
                            "endswith" | "ends_with" => QueryOperator::EndsWith,
                            "in" => QueryOperator::In,
                            "notin" | "not_in" => QueryOperator::NotIn,
                            "regex" => QueryOperator::Regex,
                            "exists" => QueryOperator::Exists,
                            "isnull" | "is_null" => QueryOperator::IsNull,
                            "isnotnull" | "is_not_null" => QueryOperator::IsNotNull,
                            _ => return Err(format!("不支持的操作符: {}", operator_str)),
                        };
                        
                        let value = self.parse_data_value(
                            condition_map.get("value")
                                .ok_or("每个查询条件必须包含value字段")?
                        )?;
                        
                        let condition = QueryCondition {
                            field: field.to_string(),
                            operator,
                            value,
                        };
                        
                        conditions.push(condition);
                    } else {
                        return Err("查询条件数组中的每个元素必须是对象".to_string());
                    }
                }
                
                Ok(conditions)
            },
            // 支持简化的键值对格式: {"name": "张三", "age": 20} (默认使用Eq操作符)
            JsonValue::Object(map) => {
                let mut conditions = Vec::new();
                for (field, value) in map {
                    let condition = QueryCondition {
                        field,
                        operator: QueryOperator::Eq,
                        value: self.parse_data_value(&value)?,
                    };
                    conditions.push(condition);
                }
                Ok(conditions)
            },
            _ => Err("查询条件JSON必须是对象或数组类型".to_string()),
        }
    }
    
    /// 解析JSON值为DataValue
    fn parse_data_value(&self, json_value: &JsonValue) -> Result<DataValue, String> {
        match json_value {
            JsonValue::String(s) => Ok(DataValue::String(s.clone())),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(DataValue::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(DataValue::Float(f))
                } else {
                    Err(format!("不支持的数字类型: {}", n))
                }
            }
            JsonValue::Bool(b) => Ok(DataValue::Bool(*b)),
            JsonValue::Null => Ok(DataValue::Null),
            JsonValue::Array(arr) => {
                let mut data_values = Vec::new();
                for item in arr {
                    data_values.push(self.parse_data_value(item)?);
                }
                Ok(DataValue::Array(data_values))
            }
            JsonValue::Object(obj) => {
                let mut data_map = HashMap::new();
                for (key, value) in obj {
                    data_map.insert(key.clone(), self.parse_data_value(value)?);
                }
                Ok(DataValue::Object(data_map))
            }
        }
    }
}

/// 创建数据库队列桥接器
#[pyfunction]
pub fn create_db_queue_bridge() -> PyResult<PyDbQueueBridge> {
    PyDbQueueBridge::new()
}

/// Python模块定义
#[pymodule]
fn rat_quickdb_py(_py: Python, m: &PyModule) -> PyResult<()> {
    // 初始化日志系统
    rat_quickdb::init();
    info!("Python绑定模块已初始化，日志系统已启动");
    
    // 基础函数
    m.add_function(wrap_pyfunction!(get_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_info, m)?)?;
    m.add_function(wrap_pyfunction!(get_name, m)?)?;
    m.add_function(wrap_pyfunction!(create_db_queue_bridge, m)?)?;
    
    // 数据库桥接器
    m.add_class::<PyDbQueueBridge>()?;
    
    // 缓存配置类
    m.add_class::<PyCacheConfig>()?;
    m.add_class::<PyL1CacheConfig>()?;
    m.add_class::<PyL2CacheConfig>()?;
    m.add_class::<PyTtlConfig>()?;
    m.add_class::<PyCompressionConfig>()?;
    
    // TLS和ZSTD配置类
    m.add_class::<PyTlsConfig>()?;
    m.add_class::<PyZstdConfig>()?;
    
    // ODM 模型系统类
    m.add_class::<PyFieldType>()?;
    m.add_class::<PyFieldDefinition>()?;
    m.add_class::<PyIndexDefinition>()?;
    m.add_class::<PyModelMeta>()?;
    
    // 便捷字段创建函数
    m.add_function(wrap_pyfunction!(string_field, m)?)?;
    m.add_function(wrap_pyfunction!(integer_field, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_field, m)?)?;
    m.add_function(wrap_pyfunction!(datetime_field, m)?)?;
    m.add_function(wrap_pyfunction!(uuid_field, m)?)?;
    m.add_function(wrap_pyfunction!(reference_field, m)?)?;
    m.add_function(wrap_pyfunction!(array_field, m)?)?;
    m.add_function(wrap_pyfunction!(list_field, m)?)?;
    m.add_function(wrap_pyfunction!(json_field, m)?)?;
    
    Ok(())
}
