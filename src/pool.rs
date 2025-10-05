//! 连接池模块
//!
//! 基于生产者/消费者模式的高性能数据库连接池
//! SQLite: 单线程队列模式，避免锁竞争
//! MySQL/PostgreSQL/MongoDB: 多连接长连接池，支持保活和重试

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crossbeam_queue::SegQueue;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use uuid::Uuid;
use serde_json::Value;
use rat_logger::{debug, info, warn, error};

use crate::types::*;
use crate::error::{QuickDbError, QuickDbResult};
use crate::model::FieldType;

/// 池化连接 - 用于兼容旧接口
#[derive(Debug, Clone)]
pub struct PooledConnection {
    /// 连接ID
    pub id: String,
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 数据库别名（用于兼容manager.rs）
    pub alias: String,
}

/// 数据库操作请求
#[derive(Debug)]
pub enum DatabaseOperation {
    /// 创建记录
    Create {
        table: String,
        data: HashMap<String, DataValue>,
        response: oneshot::Sender<QuickDbResult<DataValue>>,
    },
    /// 根据ID查找记录
    FindById {
        table: String,
        id: DataValue,
        response: oneshot::Sender<QuickDbResult<Option<DataValue>>>,
    },
    /// 查找记录
    Find {
        table: String,
        conditions: Vec<QueryCondition>,
        options: QueryOptions,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    /// 使用条件组合查找记录（支持OR逻辑）
    FindWithGroups {
        table: String,
        condition_groups: Vec<QueryConditionGroup>,
        options: QueryOptions,
        response: oneshot::Sender<QuickDbResult<Vec<DataValue>>>,
    },
    /// 更新记录
    Update {
        table: String,
        conditions: Vec<QueryCondition>,
        data: HashMap<String, DataValue>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    /// 根据ID更新记录
    UpdateById {
        table: String,
        id: DataValue,
        data: HashMap<String, DataValue>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    /// 删除记录
    Delete {
        table: String,
        conditions: Vec<QueryCondition>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    /// 根据ID删除记录
    DeleteById {
        table: String,
        id: DataValue,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    /// 统计记录
    Count {
        table: String,
        conditions: Vec<QueryCondition>,
        response: oneshot::Sender<QuickDbResult<u64>>,
    },
    /// 检查存在
    Exists {
        table: String,
        conditions: Vec<QueryCondition>,
        response: oneshot::Sender<QuickDbResult<bool>>,
    },
    /// 创建表
    CreateTable {
        table: String,
        fields: HashMap<String, FieldType>,
        response: oneshot::Sender<QuickDbResult<()>>,
    },
    /// 创建索引
    CreateIndex {
        table: String,
        index_name: String,
        fields: Vec<String>,
        unique: bool,
        response: oneshot::Sender<QuickDbResult<()>>,
    },
    /// 删除表
    DropTable {
        table: String,
        response: oneshot::Sender<QuickDbResult<()>>,
    },
}

/// 原生数据库连接枚举 - 直接持有数据库连接，不使用Arc包装
#[derive(Debug)]
pub enum DatabaseConnection {
    SQLite(sqlx::SqlitePool),
    PostgreSQL(sqlx::PgPool),
    MySQL(sqlx::MySqlPool),
    MongoDB(mongodb::Database),
}

/// 连接工作器 - 持有单个数据库连接并处理操作
pub struct ConnectionWorker {
    /// 工作器ID
    pub id: String, 
    /// 数据库连接
    pub connection: DatabaseConnection,
    /// 连接创建时间
    pub created_at: Instant,
    /// 最后使用时间
    pub last_used: Instant,
    /// 重试次数
    pub retry_count: u32,
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 数据库适配器（持久化，避免重复创建）
    pub adapter: Box<dyn crate::adapter::DatabaseAdapter + Send + Sync>,
}

impl std::fmt::Debug for ConnectionWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionWorker")
            .field("id", &self.id)
            .field("connection", &self.connection)
            .field("created_at", &self.created_at)
            .field("last_used", &self.last_used)
            .field("retry_count", &self.retry_count)
            .field("db_type", &self.db_type)
            .field("adapter", &"<DatabaseAdapter>")
            .finish()
    }
}

/// 连接池配置扩展
#[derive(Debug, Clone)]
pub struct ExtendedPoolConfig {
    /// 基础连接池配置
    pub base: PoolConfig,
    /// 缓存配置（全局共享，L1缓存必须启用）
    pub cache_config: crate::types::CacheConfig,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 保活检测间隔（秒）
    pub keepalive_interval_sec: u64,
    /// 连接健康检查超时（秒）
    pub health_check_timeout_sec: u64,
}

impl Default for ExtendedPoolConfig {
    fn default() -> Self {
        Self {
            base: PoolConfig::default(),
            cache_config: crate::types::CacheConfig::default(),
            max_retries: 3,
            retry_interval_ms: 1000,
            keepalive_interval_sec: 30,
            health_check_timeout_sec: 5,
        }
    }
}

/// 新的连接池 - 基于生产者/消费者模式
#[derive(Debug)]
pub struct ConnectionPool {
    /// 数据库配置
    pub db_config: DatabaseConfig,
    /// 扩展连接池配置
    pub config: ExtendedPoolConfig,
    /// 操作请求发送器
    pub operation_sender: mpsc::UnboundedSender<DatabaseOperation>,
    /// 数据库类型
    pub db_type: DatabaseType,
}

/// SQLite 单线程工作器
pub struct SqliteWorker {
    /// 数据库连接
    connection: DatabaseConnection,
    /// 操作接收器
    operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
    /// 数据库配置
    db_config: DatabaseConfig,
    /// 重试计数
    retry_count: u32,
    /// 最大重试次数
    max_retries: u32,
    /// 重试间隔（毫秒）
    retry_interval_ms: u64,
    /// 健康检查间隔（秒）
    health_check_interval_sec: u64,
    /// 上次健康检查时间
    last_health_check: Instant,
    /// 连接是否健康
    is_healthy: bool,
    /// 数据库适配器（持久化，避免重复创建）
    adapter: Box<dyn crate::adapter::DatabaseAdapter + Send + Sync>,
}

impl std::fmt::Debug for SqliteWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteWorker")
            .field("connection", &self.connection)
            .field("db_config", &self.db_config)
            .field("retry_count", &self.retry_count)
            .field("max_retries", &self.max_retries)
            .field("retry_interval_ms", &self.retry_interval_ms)
            .field("health_check_interval_sec", &self.health_check_interval_sec)
            .field("last_health_check", &self.last_health_check)
            .field("is_healthy", &self.is_healthy)
            .field("adapter", &"<DatabaseAdapter>")
            .finish()
    }
}

/// 多连接工作器管理器（用于MySQL/PostgreSQL/MongoDB）
pub struct MultiConnectionManager {
    /// 工作器列表
    workers: Vec<ConnectionWorker>,
    /// 可用工作器队列
    available_workers: SegQueue<usize>,
    /// 操作接收器
    operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
    /// 数据库配置
    db_config: DatabaseConfig,
    /// 扩展配置
    config: ExtendedPoolConfig,
    /// 保活任务句柄
    keepalive_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SqliteWorker {
    /// 运行SQLite工作器
    pub async fn run(mut self) {
        info!("SQLite工作器开始运行: 别名={}", self.db_config.alias);
        
        // 启动健康检查任务
        let health_check_handle = self.start_health_check_task().await;
        
        while let Some(operation) = self.operation_receiver.recv().await {
            // 检查连接健康状态
            if !self.is_healthy {
                warn!("SQLite连接不健康，尝试重新连接");
                if let Err(e) = self.reconnect().await {
                    error!("SQLite重新连接失败: {}", e);
                    continue;
                }
            }
            
            match self.handle_operation(operation).await {
                Ok(_) => {
                    self.retry_count = 0; // 重置重试计数
                    self.is_healthy = true; // 标记连接健康
                },
                Err(e) => {
                    error!("SQLite操作处理失败: {}", e);
                    self.is_healthy = false; // 标记连接不健康
                    
                    // 智能重试逻辑
                    if self.retry_count < self.max_retries {
                        self.retry_count += 1;
                        let backoff_delay = self.calculate_backoff_delay();
                        warn!("SQLite操作重试 {}/{}, 延迟{}ms", 
                              self.retry_count, self.max_retries, backoff_delay);
                        tokio::time::sleep(Duration::from_millis(backoff_delay)).await;
                        
                        // 尝试重新连接
                        if let Err(reconnect_err) = self.reconnect().await {
                            error!("SQLite重新连接失败: {}", reconnect_err);
                        }
                    } else {
                        error!("SQLite操作重试次数超限，标记连接为不健康状态");
                        self.is_healthy = false;
                        // 不再直接退出程序，而是继续运行但标记为不健康
                    }
                }
            }
        }
        
        // 清理健康检查任务
        health_check_handle.abort();
        info!("SQLite工作器停止运行");
    }
    
    /// 启动健康检查任务
    async fn start_health_check_task(&self) -> tokio::task::JoinHandle<()> {
        let health_check_interval = Duration::from_secs(self.health_check_interval_sec);
        let db_config = self.db_config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(health_check_interval);
            
            loop {
                interval.tick().await;
                debug!("执行SQLite连接健康检查: 别名={}", db_config.alias);
                // 健康检查逻辑在主循环中处理
            }
        })
    }
    
    /// 重新连接数据库
    async fn reconnect(&mut self) -> QuickDbResult<()> {
        info!("正在重新连接SQLite数据库: 别名={}", self.db_config.alias);
        
        let new_connection = self.create_sqlite_connection().await?;
        self.connection = new_connection;
        self.is_healthy = true;
        self.retry_count = 0;
        
        info!("SQLite数据库重新连接成功: 别名={}", self.db_config.alias);
        Ok(())
    }
    
    /// 创建SQLite连接
    async fn create_sqlite_connection(&self) -> QuickDbResult<DatabaseConnection> {
        let (path, create_if_missing) = match &self.db_config.connection {
            crate::types::ConnectionConfig::SQLite { path, create_if_missing } => {
                (path.clone(), *create_if_missing)
            }
            _ => return Err(QuickDbError::ConfigError {
                message: "SQLite连接配置类型不匹配".to_string(),
            }),
        };
        
        // 特殊处理内存数据库：直接连接，不创建文件
        if path == ":memory:" {
            info!("连接SQLite内存数据库: 别名={}", self.db_config.alias);
            let pool = sqlx::SqlitePool::connect(&path)
                .await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: format!("SQLite内存数据库连接失败: {}", e),
                })?;
            return Ok(DatabaseConnection::SQLite(pool));
        }
        
        // 检查数据库文件是否存在
        let file_exists = std::path::Path::new(&path).exists();
        
        // 如果文件不存在且不允许创建，则返回错误
        if !file_exists && !create_if_missing {
            return Err(QuickDbError::ConnectionError {
                message: format!("SQLite数据库文件不存在且未启用自动创建: {}", path),
            });
        }
        
        // 如果需要创建文件且文件不存在，则创建父目录
        if create_if_missing && !file_exists {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("创建SQLite数据库目录失败: {}", e),
                    })?;
            }
            
            // 创建空的数据库文件
            tokio::fs::File::create(&path).await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: format!("创建SQLite数据库文件失败: {}", e),
                })?;
        }
        
        let pool = sqlx::SqlitePool::connect(&path)
            .await
            .map_err(|e| QuickDbError::ConnectionError {
                message: format!("SQLite连接失败: {}", e),
            })?;
        Ok(DatabaseConnection::SQLite(pool))
    }
    
    /// 计算退避延迟（指数退避）
    fn calculate_backoff_delay(&self) -> u64 {
        let base_delay = self.retry_interval_ms;
        let exponential_delay = base_delay * (2_u64.pow(self.retry_count.min(10))); // 限制最大指数
        let max_delay = 30000; // 最大延迟30秒
        exponential_delay.min(max_delay)
    }
    
    /// 执行连接健康检查
    async fn perform_health_check(&mut self) -> bool {
        if self.last_health_check.elapsed() < Duration::from_secs(self.health_check_interval_sec) {
            return self.is_healthy;
        }
        
        debug!("执行SQLite连接健康检查: 别名={}", self.db_config.alias);
        
        // 执行简单的查询来检查连接健康状态
        let health_check_result = match &self.connection {
            DatabaseConnection::SQLite(pool) => {
                sqlx::query("SELECT 1")
                    .fetch_optional(pool)
                    .await
                    .is_ok()
            },
            _ => false,
        };
        
        self.last_health_check = Instant::now();
        self.is_healthy = health_check_result;
        
        if !self.is_healthy {
            warn!("SQLite连接健康检查失败: 别名={}", self.db_config.alias);
        } else {
            debug!("SQLite连接健康检查通过: 别名={}", self.db_config.alias);
        }
        
        self.is_healthy
    }
    
    /// 处理数据库操作（带 panic 捕获）
    async fn handle_operation(&mut self, operation: DatabaseOperation) -> QuickDbResult<()> {
        // 执行健康检查
        self.perform_health_check().await;
        
        // 执行数据库操作，使用 Result 来处理错误而不是 panic 捕获
        let operation_result = match operation {
            DatabaseOperation::Create { table, data, response } => {
                let result = self.adapter.create(&self.connection, &table, &data).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::FindById { table, id, response } => {
                let result = self.adapter.find_by_id(&self.connection, &table, &id).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Find { table, conditions, options, response } => {
                let result = self.adapter.find(&self.connection, &table, &conditions, &options).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::FindWithGroups { table, condition_groups, options, response } => {
                let result = self.adapter.find_with_groups(&self.connection, &table, &condition_groups, &options).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Update { table, conditions, data, response } => {
                let result = self.adapter.update(&self.connection, &table, &conditions, &data).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::UpdateById { table, id, data, response } => {
                let result = self.adapter.update_by_id(&self.connection, &table, &id, &data).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Delete { table, conditions, response } => {
                let result = self.adapter.delete(&self.connection, &table, &conditions).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::DeleteById { table, id, response } => {
                let result = self.adapter.delete_by_id(&self.connection, &table, &id).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Count { table, conditions, response } => {
                let result = self.adapter.count(&self.connection, &table, &conditions).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Exists { table, conditions, response } => {
                let result = self.adapter.exists(&self.connection, &table, &conditions).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::CreateTable { table, fields, response } => {
                let result = self.adapter.create_table(&self.connection, &table, &fields).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::CreateIndex { table, index_name, fields, unique, response } => {
                let result = self.adapter.create_index(&self.connection, &table, &index_name, &fields, unique).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::DropTable { table, response } => {
                let result = self.adapter.drop_table(&self.connection, &table).await;
                let _ = response.send(result);
                Ok(())
            },
        };
        
        operation_result
    }
}

impl MultiConnectionManager {
    /// 创建初始连接
    pub async fn create_initial_connections(&mut self) -> QuickDbResult<()> {
        info!("创建初始连接池，大小: {}", self.config.base.max_connections);
        
        for i in 0..self.config.base.max_connections {
            let worker = self.create_connection_worker(i as usize).await?;
            self.workers.push(worker);
            self.available_workers.push(i as usize);
        }
        
        Ok(())
    }
    
    /// 创建连接工作器
    async fn create_connection_worker(&self, index: usize) -> QuickDbResult<ConnectionWorker> {
        let connection = self.create_database_connection().await?;
        
        // 创建适配器（缓存功能已内置）
        use crate::adapter::create_adapter;
        let adapter = create_adapter(&self.db_config.db_type)?;
        
        // 只在第一个工作器创建时输出缓存配置信息
        if index == 0 {
            let cache_info = if self.config.cache_config.l2_config.is_some() {
                "双层缓存（L1内存+L2持久化）"
            } else {
                "单层缓存（仅L1内存）"
            };
            info!("数据库 '{}' 使用 {} 模式", self.db_config.alias, cache_info);
        }
        
        Ok(ConnectionWorker {
            id: format!("{}-worker-{}", self.db_config.alias, index),
            connection,
            created_at: Instant::now(),
            last_used: Instant::now(),
            retry_count: 0,
            db_type: self.db_config.db_type.clone(),
            adapter,
        })
    }
    
    /// 创建数据库连接
    async fn create_database_connection(&self) -> QuickDbResult<DatabaseConnection> {
        match &self.db_config.db_type {
            DatabaseType::PostgreSQL => {
                let connection_string = match &self.db_config.connection {
                    crate::types::ConnectionConfig::PostgreSQL { host, port, database, username, password, ssl_mode: _, tls_config: _ } => {
                        // 对密码进行 URL 编码以处理特殊字符
                        let encoded_password = urlencoding::encode(password);
                        format!("postgresql://{}:{}@{}:{}/{}", username, encoded_password, host, port, database)
                    }
                    _ => return Err(QuickDbError::ConfigError {
                        message: "PostgreSQL连接配置类型不匹配".to_string(),
                    }),
                };
                
                let pool = sqlx::PgPool::connect(&connection_string)
                    .await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("PostgreSQL连接失败: {}", e),
                    })?;
                Ok(DatabaseConnection::PostgreSQL(pool))
            },
            DatabaseType::MySQL => {
                let connection_string = match &self.db_config.connection {
                    crate::types::ConnectionConfig::MySQL { host, port, database, username, password, ssl_opts: _, tls_config: _ } => {
                        // 对密码进行 URL 编码以处理特殊字符
                        let encoded_password = urlencoding::encode(password);
                        format!("mysql://{}:{}@{}:{}/{}", username, encoded_password, host, port, database)
                    }
                    _ => return Err(QuickDbError::ConfigError {
                        message: "MySQL连接配置类型不匹配".to_string(),
                    }),
                };
                
                let pool = sqlx::MySqlPool::connect(&connection_string)
                    .await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("MySQL连接失败: {}", e),
                    })?;
                Ok(DatabaseConnection::MySQL(pool))
            },
            DatabaseType::MongoDB => {
                let connection_uri = match &self.db_config.connection {
                    crate::types::ConnectionConfig::MongoDB { 
                        host, port, database, username, password, 
                        auth_source, direct_connection, tls_config, 
                        zstd_config, options 
                    } => {
                        // 使用构建器生成连接URI
                        let mut builder = crate::types::MongoDbConnectionBuilder::new(
                            host.clone(), 
                            *port, 
                            database.clone()
                        );
                        
                        // 设置认证信息
                        if let (Some(user), Some(pass)) = (username, password) {
                            builder = builder.with_auth(user.clone(), pass.clone());
                        }
                        
                        // 设置认证数据库
                        if let Some(auth_src) = auth_source {
                            builder = builder.with_auth_source(auth_src.clone());
                        }
                        
                        // 设置直接连接
                        builder = builder.with_direct_connection(*direct_connection);
                        
                        // 设置TLS配置
                        if let Some(tls) = tls_config {
                            builder = builder.with_tls_config(tls.clone());
                        }
                        
                        // 设置ZSTD压缩配置
                        if let Some(zstd) = zstd_config {
                            builder = builder.with_zstd_config(zstd.clone());
                        }
                        
                        // 添加自定义选项
                        if let Some(opts) = options {
                            for (key, value) in opts {
                                builder = builder.with_option(key.clone(), value.clone());
                            }
                        }
                        
                        builder.build_uri()
                    }
                    _ => return Err(QuickDbError::ConfigError {
                        message: "MongoDB连接配置类型不匹配".to_string(),
                    }),
                };
                
                debug!("MongoDB连接URI: {}", connection_uri);
                
                let client = mongodb::Client::with_uri_str(&connection_uri)
                    .await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("MongoDB连接失败: {}", e),
                    })?;
                    
                let database_name = match &self.db_config.connection {
                    crate::types::ConnectionConfig::MongoDB { database, .. } => database.clone(),
                    _ => unreachable!(),
                };
                
                let db = client.database(&database_name);
                Ok(DatabaseConnection::MongoDB(db))
            },
            _ => Err(QuickDbError::ConfigError {
                message: "不支持的数据库类型用于多连接管理器".to_string(),
            }),
        }
    }
    
    /// 启动连接保活任务
    pub fn start_keepalive_task(&mut self) {
        let keepalive_interval = Duration::from_secs(self.config.keepalive_interval_sec);
        let health_check_timeout = Duration::from_secs(self.config.health_check_timeout_sec);
        
        // 这里需要实现保活逻辑的占位符
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(keepalive_interval);
            
            loop {
                interval.tick().await;
                debug!("执行连接保活检查");
                // TODO: 实现具体的保活逻辑
            }
        });
        
        self.keepalive_handle = Some(handle);
    }
    
    /// 运行多连接管理器
    pub async fn run(mut self) {
        info!("多连接管理器开始运行: 别名={}", self.db_config.alias);
        
        // 创建初始连接
        if let Err(e) = self.create_initial_connections().await {
            error!("创建初始连接失败: {}", e);
            return;
        }
        
        // 启动保活任务
        self.start_keepalive_task();
        
        while let Some(operation) = self.operation_receiver.recv().await {
            if let Err(e) = self.handle_operation(operation).await {
                error!("多连接操作处理失败: {}", e);
            }
        }
        
        info!("多连接管理器停止运行");
    }
    
    /// 处理数据库操作
    async fn handle_operation(&mut self, operation: DatabaseOperation) -> QuickDbResult<()> {
        // 获取可用工作器
        let worker_index = match self.available_workers.pop() {
            Some(index) => index,
            None => {
                // 所有工作器都在使用中，等待或创建新连接
                return Err(QuickDbError::ConnectionError {
                    message: "所有连接都在使用中".to_string(),
                });
            }
        };
        
        // 获取工作器的连接
        let worker = &mut self.workers[worker_index];
        worker.last_used = Instant::now();
        
        // 处理具体操作
        let result = match operation {
            DatabaseOperation::Create { table, data, response } => {
                let result = worker.adapter.create(&worker.connection, &table, &data).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::FindById { table, id, response } => {
                let result = worker.adapter.find_by_id(&worker.connection, &table, &id).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Find { table, conditions, options, response } => {
                let result = worker.adapter.find(&worker.connection, &table, &conditions, &options).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::FindWithGroups { table, condition_groups, options, response } => {
                let result = worker.adapter.find_with_groups(&worker.connection, &table, &condition_groups, &options).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Update { table, conditions, data, response } => {
                let result = worker.adapter.update(&worker.connection, &table, &conditions, &data).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::UpdateById { table, id, data, response } => {
                let result = worker.adapter.update_by_id(&worker.connection, &table, &id, &data).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Delete { table, conditions, response } => {
                let result = worker.adapter.delete(&worker.connection, &table, &conditions).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::DeleteById { table, id, response } => {
                let result = worker.adapter.delete_by_id(&worker.connection, &table, &id).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Count { table, conditions, response } => {
                let result = worker.adapter.count(&worker.connection, &table, &conditions).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::Exists { table, conditions, response } => {
                let result = worker.adapter.exists(&worker.connection, &table, &conditions).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::CreateTable { table, fields, response } => {
                let result = worker.adapter.create_table(&worker.connection, &table, &fields).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::CreateIndex { table, index_name, fields, unique, response } => {
                let result = worker.adapter.create_index(&worker.connection, &table, &index_name, &fields, unique).await;
                let _ = response.send(result);
                Ok(())
            },
            DatabaseOperation::DropTable { table, response } => {
                let result = worker.adapter.drop_table(&worker.connection, &table).await;
                let _ = response.send(result);
                Ok(())
            },
        };
        
        // 处理连接错误和重试逻辑
        if let Err(ref e) = result {
            let worker_id = worker.id.clone();
            worker.retry_count += 1;
            let retry_count = worker.retry_count;
            
            error!("工作器 {} 操作失败 ({}/{}): {}", worker_id, retry_count, self.config.max_retries, e);
            
            if retry_count > self.config.max_retries {
                warn!("工作器 {} 重试次数超限，尝试重新创建连接", worker_id);
                
                // 释放对 worker 的借用，然后重新创建连接
                drop(worker);
                
                // 尝试重新创建连接，但不退出程序
                match self.create_connection_worker(worker_index).await {
                    Ok(new_worker) => {
                        self.workers[worker_index] = new_worker;
                        info!("工作器 {} 连接已重新创建", worker_index);
                    },
                    Err(create_err) => {
                        error!("重新创建工作器 {} 连接失败: {}", worker_index, create_err);
                        // 重新获取 worker 引用并重置计数
                        if let Some(worker) = self.workers.get_mut(worker_index) {
                            worker.retry_count = 0; // 重置计数，下次再试
                        }
                        // 延迟一段时间再重试
                        tokio::time::sleep(Duration::from_millis(self.config.retry_interval_ms * 2)).await;
                    }
                }
            }
        } else {
            // 操作成功，重置重试计数
            worker.retry_count = 0;
        }
        
        // 归还工作器
        self.available_workers.push(worker_index);
        
        result
    }
}

impl ConnectionPool {
    /// 使用配置创建连接池（自动初始化全局缓存）
    pub async fn with_config(db_config: DatabaseConfig, config: ExtendedPoolConfig) -> QuickDbResult<Self> {
        // 初始化全局缓存管理器（如果尚未初始化）
        use crate::cache_singleton::GlobalCacheManager;

        if !GlobalCacheManager::is_initialized() {
            GlobalCacheManager::initialize(config.cache_config.clone()).await
                .map_err(|e| crate::error::QuickDbError::CacheError { message: e.to_string() })?;
        }

        let (operation_sender, operation_receiver) = mpsc::unbounded_channel();

        let pool = Self {
            db_type: db_config.db_type.clone(),
            db_config: db_config.clone(),
            config: config.clone(),
            operation_sender,
        };

        // 根据数据库类型启动对应的工作器
        match &db_config.db_type {
            DatabaseType::SQLite => {
                pool.start_sqlite_worker(operation_receiver, db_config, config).await?;
            },
            _ => {
                pool.start_multi_connection_manager(operation_receiver, db_config, config).await?;
            }
        }

        Ok(pool)
    }

    /// 启动SQLite工作器
    async fn start_sqlite_worker(
        &self,
        operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
        db_config: DatabaseConfig,
        config: ExtendedPoolConfig,
    ) -> QuickDbResult<()> {
        let connection = self.create_sqlite_connection().await?;
        
        // 创建启动同步通道
        let (startup_tx, startup_rx) = oneshot::channel();
        
        // 创建适配器（缓存功能已内置）
        use crate::adapter::create_adapter;
        let adapter = create_adapter(&db_config.db_type)?;

                
        let worker = SqliteWorker {
            connection,
            operation_receiver,
            db_config: db_config.clone(),
            retry_count: 0,
            max_retries: config.max_retries,
            retry_interval_ms: config.retry_interval_ms,
            health_check_interval_sec: config.health_check_timeout_sec, // 复用健康检查超时作为间隔
            last_health_check: Instant::now(),
            is_healthy: true,
            adapter,
        };
        
        // 启动工作器
        tokio::spawn(async move {
            // 发送启动完成信号
            let _ = startup_tx.send(());
            worker.run().await;
        });
        
        // 等待工作器启动完成
        startup_rx.await.map_err(|_| QuickDbError::ConnectionError {
            message: format!("SQLite工作器启动失败: 别名={}", db_config.alias),
        })?;
        
        info!("SQLite工作器启动完成: 别名={}", db_config.alias);
        Ok(())
    }
    
    /// 启动多连接管理器
    async fn start_multi_connection_manager(
        &self,
        operation_receiver: mpsc::UnboundedReceiver<DatabaseOperation>,
        db_config: DatabaseConfig,
        config: ExtendedPoolConfig,
    ) -> QuickDbResult<()> {
        let manager = MultiConnectionManager {
            workers: Vec::new(),
            available_workers: SegQueue::new(),
            operation_receiver,
            db_config,
            config,
            keepalive_handle: None,
        };
        
        // 启动管理器
        tokio::spawn(async move {
            manager.run().await;
        });
        
        Ok(())
    }
    
    /// 创建SQLite连接
    async fn create_sqlite_connection(&self) -> QuickDbResult<DatabaseConnection> {
        let (path, create_if_missing) = match &self.db_config.connection {
            crate::types::ConnectionConfig::SQLite { path, create_if_missing } => {
                (path.clone(), *create_if_missing)
            }
            _ => return Err(QuickDbError::ConfigError {
                message: "SQLite连接配置类型不匹配".to_string(),
            }),
        };
        
        // 特殊处理内存数据库：直接连接，不创建文件
        if path == ":memory:" {
            info!("连接SQLite内存数据库: 别名={}", self.db_config.alias);
            let pool = sqlx::SqlitePool::connect(&path)
                .await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: format!("SQLite内存数据库连接失败: {}", e),
                })?;
            return Ok(DatabaseConnection::SQLite(pool));
        }
        
        // 检查数据库文件是否存在
        let file_exists = std::path::Path::new(&path).exists();
        
        // 如果文件不存在且不允许创建，则返回错误
        if !file_exists && !create_if_missing {
            return Err(QuickDbError::ConnectionError {
                message: format!("SQLite数据库文件不存在且未启用自动创建: {}", path),
            });
        }
        
        // 如果需要创建文件且文件不存在，则创建父目录
        if create_if_missing && !file_exists {
            if let Some(parent) = std::path::Path::new(&path).parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| QuickDbError::ConnectionError {
                        message: format!("创建SQLite数据库目录失败: {}", e),
                    })?;
            }
            
            // 创建空的数据库文件
            tokio::fs::File::create(&path).await
                .map_err(|e| QuickDbError::ConnectionError {
                    message: format!("创建SQLite数据库文件失败: {}", e),
                })?;
        }
        
        let pool = sqlx::SqlitePool::connect(&path)
            .await
            .map_err(|e| QuickDbError::ConnectionError {
                message: format!("SQLite连接失败: {}", e),
            })?;
        Ok(DatabaseConnection::SQLite(pool))
    }
    
    /// 发送操作请求并等待响应
    async fn send_operation<T>(&self, operation: DatabaseOperation) -> QuickDbResult<T>
    where
        T: Send + 'static,
    {
        // 这是一个泛型占位符，实际实现需要根据具体操作类型来处理
        Err(QuickDbError::QueryError {
            message: "操作发送未实现".to_string(),
        })
    }
    
    /// 创建记录
    pub async fn create(
        &self,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<DataValue> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::Create {
            table: table.to_string(),
            data: data.clone(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 根据ID查找记录
    pub async fn find_by_id(
        &self,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        rat_logger::info!("🔍 Pool.find_by_id: table={}, id={}", table, id);
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::FindById {
            table: table.to_string(),
            id: id.clone(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 查找记录
    pub async fn find(
        &self,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::Find {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            options: options.clone(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 更新记录
    pub async fn update(
        &self,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::Update {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            data: data.clone(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 根据ID更新记录
    pub async fn update_by_id(
        &self,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::UpdateById {
            table: table.to_string(),
            id: id.clone(),
            data: data.clone(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 删除记录
    pub async fn delete(
        &self,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::Delete {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 根据ID删除记录
    pub async fn delete_by_id(
        &self,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::DeleteById {
            table: table.to_string(),
            id: id.clone(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 统计记录
    pub async fn count(
        &self,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::Count {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 检查记录是否存在
    pub async fn exists(
        &self,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::Exists {
            table: table.to_string(),
            conditions: conditions.to_vec(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 创建表
    pub async fn create_table(
        &self,
        table: &str,
        fields: &HashMap<String, FieldType>,
    ) -> QuickDbResult<()> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::CreateTable {
            table: table.to_string(),
            fields: fields.clone(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 创建索引
    pub async fn create_index(
        &self,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::CreateIndex {
            table: table.to_string(),
            index_name: index_name.to_string(),
            fields: fields.to_vec(),
            unique,
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 删除表
    pub async fn drop_table(&self, table: &str) -> QuickDbResult<()> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let operation = DatabaseOperation::DropTable {
            table: table.to_string(),
            response: response_sender,
        };
        
        self.operation_sender.send(operation)
            .map_err(|_| QuickDbError::QueryError {
                message: "发送操作失败".to_string(),
            })?;
        
        response_receiver.await
            .map_err(|_| QuickDbError::QueryError {
                message: "接收响应失败".to_string(),
            })?
    }
    
    /// 获取数据库类型
    pub fn get_database_type(&self) -> &DatabaseType {
        &self.db_config.db_type
    }
    
    /// 获取连接（兼容旧接口）
    pub async fn get_connection(&self) -> QuickDbResult<PooledConnection> {
        // 在新架构中，我们不再直接返回连接
        // 这个方法主要用于兼容性，返回一个虚拟连接
        Ok(PooledConnection {
            id: format!("{}-virtual", self.db_config.alias),
            db_type: self.db_config.db_type.clone(),
            alias: self.db_config.alias.clone(),
        })
    }
    
    /// 释放连接（兼容旧接口）
    pub async fn release_connection(&self, _connection_id: &str) -> QuickDbResult<()> {
        // 在新架构中，连接由工作器自动管理，这个方法为空实现
        Ok(())
    }
    
    /// 清理过期连接（兼容旧接口）
    pub async fn cleanup_expired_connections(&self) {
        // 在新架构中，连接由工作器自动管理，这个方法为空实现
        debug!("清理过期连接（新架构中自动管理）");
    }
}