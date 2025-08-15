//! 数据库类型定义和配置
//!
//! 定义支持的数据库类型、连接配置和通用数据类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 支持的数据库类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DatabaseType {
    /// SQLite 数据库
    SQLite,
    /// PostgreSQL 数据库
    PostgreSQL,
    /// MySQL 数据库
    MySQL,
    /// MongoDB 数据库
    MongoDB,
}

impl DatabaseType {
    /// 获取数据库类型的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::SQLite => "sqlite",
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::MySQL => "mysql",
            DatabaseType::MongoDB => "mongodb",
        }
    }

    /// 从字符串解析数据库类型
    pub fn from_str(s: &str) -> Result<Self, crate::error::QuickDbError> {
        match s.to_lowercase().as_str() {
            "sqlite" => Ok(DatabaseType::SQLite),
            "postgresql" | "postgres" | "pg" => Ok(DatabaseType::PostgreSQL),
            "mysql" => Ok(DatabaseType::MySQL),
            "mongodb" | "mongo" => Ok(DatabaseType::MongoDB),
            _ => Err(crate::quick_error!(unsupported_db, s)),
        }
    }
}

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库类型
    pub db_type: DatabaseType,
    /// 连接字符串或配置
    pub connection: ConnectionConfig,
    /// 连接池配置
    pub pool: PoolConfig,
    /// 数据库别名（默认为 "default"）
    pub alias: String,
    /// 缓存配置（可选）
    #[cfg(feature = "cache")]
    pub cache: Option<CacheConfig>,
    /// ID 生成策略
    pub id_strategy: IdStrategy,
}

/// 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionConfig {
    /// SQLite 文件路径
    SQLite {
        /// 数据库文件路径
        path: String,
        /// 是否创建数据库文件（如果不存在）
        create_if_missing: bool,
    },
    /// PostgreSQL 连接配置
    PostgreSQL {
        /// 主机地址
        host: String,
        /// 端口号
        port: u16,
        /// 数据库名
        database: String,
        /// 用户名
        username: String,
        /// 密码
        password: String,
        /// SSL 模式 (disable, allow, prefer, require, verify-ca, verify-full)
        ssl_mode: Option<String>,
        /// TLS 配置选项
        tls_config: Option<TlsConfig>,
    },
    /// MySQL 连接配置
    MySQL {
        /// 主机地址
        host: String,
        /// 端口号
        port: u16,
        /// 数据库名
        database: String,
        /// 用户名
        username: String,
        /// 密码
        password: String,
        /// SSL 配置
        ssl_opts: Option<HashMap<String, String>>,
        /// TLS 配置选项
        tls_config: Option<TlsConfig>,
    },
    /// MongoDB 连接配置
    MongoDB {
        /// 连接字符串
        uri: String,
        /// 数据库名
        database: String,
        /// 认证数据库
        auth_source: Option<String>,
        /// TLS 配置选项
        tls_config: Option<TlsConfig>,
        /// ZSTD 压缩配置
        zstd_config: Option<ZstdConfig>,
    },
}

/// TLS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// 是否启用 TLS
    pub enabled: bool,
    /// CA 证书文件路径
    pub ca_cert_path: Option<String>,
    /// 客户端证书文件路径
    pub client_cert_path: Option<String>,
    /// 客户端私钥文件路径
    pub client_key_path: Option<String>,
    /// 是否验证服务器证书
    pub verify_server_cert: bool,
    /// 是否验证主机名
    pub verify_hostname: bool,
    /// 允许的 TLS 版本（如 "1.2", "1.3"）
    pub min_tls_version: Option<String>,
    /// 允许的密码套件
    pub cipher_suites: Option<Vec<String>>,
}

impl Default for TlsConfig {
    fn default() -> Self {
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
}

impl TlsConfig {
    /// 创建启用 TLS 的配置
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// 设置 CA 证书路径
    pub fn with_ca_cert<P: Into<String>>(mut self, path: P) -> Self {
        self.ca_cert_path = Some(path.into());
        self
    }

    /// 设置客户端证书和私钥路径
    pub fn with_client_cert<P1: Into<String>, P2: Into<String>>(mut self, cert_path: P1, key_path: P2) -> Self {
        self.client_cert_path = Some(cert_path.into());
        self.client_key_path = Some(key_path.into());
        self
    }

    /// 设置是否验证服务器证书
    pub fn verify_server_cert(mut self, verify: bool) -> Self {
        self.verify_server_cert = verify;
        self
    }

    /// 设置是否验证主机名
    pub fn verify_hostname(mut self, verify: bool) -> Self {
        self.verify_hostname = verify;
        self
    }

    /// 设置最小 TLS 版本
    pub fn with_min_tls_version<V: Into<String>>(mut self, version: V) -> Self {
        self.min_tls_version = Some(version.into());
        self
    }

    /// 设置允许的密码套件
    pub fn with_cipher_suites(mut self, suites: Vec<String>) -> Self {
        self.cipher_suites = Some(suites);
        self
    }
}

/// ZSTD 压缩配置（主要用于 MongoDB）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZstdConfig {
    /// 是否启用 ZSTD 压缩
    pub enabled: bool,
    /// 压缩级别（1-22，默认为 3）
    pub compression_level: Option<i32>,
    /// 压缩阈值（字节数，小于此值不压缩）
    pub compression_threshold: Option<usize>,
}

impl Default for ZstdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            compression_level: Some(3),
            compression_threshold: Some(1024), // 1KB
        }
    }
}

impl ZstdConfig {
    /// 创建启用 ZSTD 压缩的配置
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// 设置压缩级别（1-22）
    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = Some(level.clamp(1, 22));
        self
    }

    /// 设置压缩阈值（字节数）
    pub fn with_compression_threshold(mut self, threshold: usize) -> Self {
        self.compression_threshold = Some(threshold);
        self
    }
}

/// 连接池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// 最小连接数
    pub min_connections: u32,
    /// 最大连接数
    pub max_connections: u32,
    /// 连接超时时间（秒）
    pub connection_timeout: u64,
    /// 空闲连接超时时间（秒）
    pub idle_timeout: u64,
    /// 连接最大生存时间（秒）
    pub max_lifetime: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 3600,
        }
    }
}

/// 通用数据值类型 - 支持跨数据库的数据表示
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataValue {
    /// 空值
    Null,
    /// 布尔值
    Bool(bool),
    /// 整数
    Int(i64),
    /// 浮点数
    Float(f64),
    /// 字符串
    String(String),
    /// 字节数组
    Bytes(Vec<u8>),
    /// 日期时间
    DateTime(DateTime<Utc>),
    /// UUID
    Uuid(Uuid),
    /// JSON 对象
    Json(serde_json::Value),
    /// 数组
    Array(Vec<DataValue>),
    /// 对象/文档
    Object(HashMap<String, DataValue>),
}

impl std::fmt::Display for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataValue::Null => write!(f, "null"),
            DataValue::Bool(b) => write!(f, "{}", b),
            DataValue::Int(i) => write!(f, "{}", i),
            DataValue::Float(fl) => write!(f, "{}", fl),
            DataValue::String(s) => write!(f, "{}", s),
            DataValue::Bytes(bytes) => write!(f, "[{} bytes]", bytes.len()),
            DataValue::DateTime(dt) => write!(f, "{}", dt.to_rfc3339()),
            DataValue::Uuid(uuid) => write!(f, "{}", uuid),
            DataValue::Json(json) => write!(f, "{}", json),
            DataValue::Array(arr) => {
                let json_str = serde_json::to_string(arr).unwrap_or_default();
                write!(f, "{}", json_str)
            },
            DataValue::Object(obj) => {
                let json_str = serde_json::to_string(obj).unwrap_or_default();
                write!(f, "{}", json_str)
            },
        }
    }
}

impl DataValue {
    /// 获取数据类型名称
    pub fn type_name(&self) -> &'static str {
        match self {
            DataValue::Null => "null",
            DataValue::Bool(_) => "bool",
            DataValue::Int(_) => "int",
            DataValue::Float(_) => "float",
            DataValue::String(_) => "string",
            DataValue::Bytes(_) => "bytes",
            DataValue::DateTime(_) => "datetime",
            DataValue::Uuid(_) => "uuid",
            DataValue::Json(_) => "json",
            DataValue::Array(_) => "array",
            DataValue::Object(_) => "object",
        }
    }

    /// 判断是否为空值
    pub fn is_null(&self) -> bool {
        matches!(self, DataValue::Null)
    }

    /// 转换为 JSON 字符串
    pub fn to_json_string(&self) -> Result<String, crate::error::QuickDbError> {
        serde_json::to_string(self).map_err(|e| {
            crate::quick_error!(serialization, format!("DataValue 转换为 JSON 失败: {}", e))
        })
    }

    /// 从 JSON 字符串解析
    pub fn from_json_string(json: &str) -> Result<Self, crate::error::QuickDbError> {
        serde_json::from_str(json).map_err(|e| {
            crate::quick_error!(serialization, format!("JSON 解析为 DataValue 失败: {}", e))
        })
    }

    /// 转换为 JSON 值
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// 从 JSON 值解析
    pub fn from_json_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap_or(DataValue::Null)
    }

    /// 转换为 JSON（兼容旧代码）
    pub fn to_json(&self) -> serde_json::Value {
        self.to_json_value()
    }

    /// 从 JSON 解析（兼容旧代码）
    pub fn from_json(value: serde_json::Value) -> Self {
        Self::from_json_value(value)
    }
}

/// 查询条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCondition {
    /// 字段名
    pub field: String,
    /// 操作符
    pub operator: QueryOperator,
    /// 值
    pub value: DataValue,
}

/// 查询操作符
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryOperator {
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 大于
    Gt,
    /// 大于等于
    Gte,
    /// 小于
    Lt,
    /// 小于等于
    Lte,
    /// 包含（字符串）
    Contains,
    /// 开始于（字符串）
    StartsWith,
    /// 结束于（字符串）
    EndsWith,
    /// 在列表中
    In,
    /// 不在列表中
    NotIn,
    /// 正则表达式匹配
    Regex,
    /// 存在（字段存在）
    Exists,
    /// 为空
    IsNull,
    /// 不为空
    IsNotNull,
}

/// 排序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortConfig {
    /// 字段名
    pub field: String,
    /// 排序方向
    pub direction: SortDirection,
}

/// 排序方向
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    /// 升序
    Asc,
    /// 降序
    Desc,
}

/// 分页配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    /// 跳过的记录数
    pub skip: u64,
    /// 限制返回的记录数
    pub limit: u64,
}

/// 查询选项
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryOptions {
    /// 查询条件
    pub conditions: Vec<QueryCondition>,
    /// 排序配置
    pub sort: Vec<SortConfig>,
    /// 分页配置
    pub pagination: Option<PaginationConfig>,
    /// 选择的字段（空表示选择所有字段）
    pub fields: Vec<String>,
}

impl QueryOptions {
    /// 创建新的查询选项
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置条件
    pub fn with_conditions(mut self, conditions: Vec<QueryCondition>) -> Self {
        self.conditions = conditions;
        self
    }

    /// 设置排序
    pub fn with_sort(mut self, sort: Vec<SortConfig>) -> Self {
        self.sort = sort;
        self
    }

    /// 设置分页
    pub fn with_pagination(mut self, pagination: PaginationConfig) -> Self {
        self.pagination = Some(pagination);
        self
    }

    /// 设置字段选择
    pub fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }
}

/// 缓存配置
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 是否启用缓存
    pub enabled: bool,
    /// 缓存策略
    pub strategy: CacheStrategy,
    /// L1 缓存配置
    pub l1_config: L1CacheConfig,
    /// L2 缓存配置（可选）
    pub l2_config: Option<L2CacheConfig>,
    /// TTL 配置
    pub ttl_config: TtlConfig,
    /// 压缩配置
    pub compression_config: CompressionConfig,
}

/// 缓存策略
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    /// LRU（最近最少使用）
    Lru,
    /// LFU（最少使用频率）
    Lfu,
    /// FIFO（先进先出）
    Fifo,
    /// 自定义策略
    Custom(String),
}

/// L1 缓存配置（内存缓存）
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1CacheConfig {
    /// 最大容量（条目数）
    pub max_capacity: usize,
    /// 最大内存使用（字节）
    pub max_memory_mb: usize,
    /// 是否启用统计
    pub enable_stats: bool,
}

/// L2 缓存配置（持久化缓存）
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CacheConfig {
    /// 存储路径
    pub storage_path: String,
    /// 最大磁盘使用（MB）
    pub max_disk_mb: usize,
    /// 压缩级别（0-22）
    pub compression_level: i32,
    /// 是否启用 WAL
    pub enable_wal: bool,
}

/// TTL 配置
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtlConfig {
    /// 默认 TTL（秒）
    pub default_ttl_secs: u64,
    /// 最大 TTL（秒）
    pub max_ttl_secs: u64,
    /// TTL 检查间隔（秒）
    pub check_interval_secs: u64,
}

/// 压缩配置
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// 是否启用压缩
    pub enabled: bool,
    /// 压缩算法
    pub algorithm: CompressionAlgorithm,
    /// 压缩阈值（字节）
    pub threshold_bytes: usize,
}

/// 压缩算法
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// ZSTD 压缩
    Zstd,
    /// LZ4 压缩
    Lz4,
    /// Gzip 压缩
    Gzip,
}

/// ID 生成策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdStrategy {
    /// 数据库自增 ID（数字）
    AutoIncrement,
    /// UUID v4（字符串）
    Uuid,
    /// 雪花算法 ID（字符串）
    #[cfg(feature = "snowflake-id")]
    Snowflake {
        /// 机器 ID（0-1023）
        machine_id: u16,
        /// 数据中心 ID（0-31）
        datacenter_id: u8,
    },
    /// MongoDB ObjectId（字符串）
    ObjectId,
    /// 自定义 ID 生成器
    Custom(String),
}

/// ID 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdType {
    /// 数字 ID
    Number(i64),
    /// 字符串 ID
    String(String),
}

impl std::fmt::Display for IdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdType::Number(n) => write!(f, "{}", n),
            IdType::String(s) => write!(f, "{}", s),
        }
    }
}

// 实现默认值和构建器方法
#[cfg(feature = "cache")]
impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: CacheStrategy::Lru,
            l1_config: L1CacheConfig::default(),
            l2_config: None,
            ttl_config: TtlConfig::default(),
            compression_config: CompressionConfig::default(),
        }
    }
}

#[cfg(feature = "cache")]
impl CacheConfig {
    /// 创建新的缓存配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 启用缓存
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置缓存策略
    pub fn with_strategy(mut self, strategy: CacheStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// 设置 L1 缓存配置
    pub fn with_l1_config(mut self, config: L1CacheConfig) -> Self {
        self.l1_config = config;
        self
    }

    /// 设置 L2 缓存配置
    pub fn with_l2_config(mut self, config: L2CacheConfig) -> Self {
        self.l2_config = Some(config);
        self
    }

    /// 设置 TTL 配置
    pub fn with_ttl_config(mut self, config: TtlConfig) -> Self {
        self.ttl_config = config;
        self
    }

    /// 设置压缩配置
    pub fn with_compression_config(mut self, config: CompressionConfig) -> Self {
        self.compression_config = config;
        self
    }
}

#[cfg(feature = "cache")]
impl Default for L1CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10000,
            max_memory_mb: 100,
            enable_stats: true,
        }
    }
}

#[cfg(feature = "cache")]
impl L1CacheConfig {
    /// 创建新的 L1 缓存配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大容量
    pub fn with_max_capacity(mut self, capacity: usize) -> Self {
        self.max_capacity = capacity;
        self
    }

    /// 设置最大内存使用
    pub fn with_max_memory_mb(mut self, memory_mb: usize) -> Self {
        self.max_memory_mb = memory_mb;
        self
    }

    /// 启用统计
    pub fn enable_stats(mut self, enable: bool) -> Self {
        self.enable_stats = enable;
        self
    }
}

#[cfg(feature = "cache")]
impl L2CacheConfig {
    /// 创建新的 L2 缓存配置
    pub fn new(storage_path: String) -> Self {
        Self {
            storage_path,
            max_disk_mb: 1000,
            compression_level: 3,
            enable_wal: true,
        }
    }

    /// 设置最大磁盘使用
    pub fn with_max_disk_mb(mut self, disk_mb: usize) -> Self {
        self.max_disk_mb = disk_mb;
        self
    }

    /// 设置压缩级别
    pub fn with_compression_level(mut self, level: i32) -> Self {
        self.compression_level = level;
        self
    }

    /// 启用 WAL
    pub fn enable_wal(mut self, enable: bool) -> Self {
        self.enable_wal = enable;
        self
    }
}

#[cfg(feature = "cache")]
impl Default for TtlConfig {
    fn default() -> Self {
        Self {
            default_ttl_secs: 3600, // 1小时
            max_ttl_secs: 86400,     // 24小时
            check_interval_secs: 300, // 5分钟
        }
    }
}

#[cfg(feature = "cache")]
impl TtlConfig {
    /// 创建新的 TTL 配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置默认 TTL
    pub fn with_default_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.default_ttl_secs = ttl_secs;
        self
    }

    /// 设置最大 TTL
    pub fn with_max_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.max_ttl_secs = ttl_secs;
        self
    }

    /// 设置检查间隔
    pub fn with_check_interval_secs(mut self, interval_secs: u64) -> Self {
        self.check_interval_secs = interval_secs;
        self
    }
}

#[cfg(feature = "cache")]
impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: CompressionAlgorithm::Zstd,
            threshold_bytes: 1024, // 1KB
        }
    }
}

#[cfg(feature = "cache")]
impl CompressionConfig {
    /// 创建新的压缩配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 启用压缩
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置压缩算法
    pub fn with_algorithm(mut self, algorithm: CompressionAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// 设置压缩阈值
    pub fn with_threshold_bytes(mut self, threshold: usize) -> Self {
        self.threshold_bytes = threshold;
        self
    }
}

impl Default for IdStrategy {
    fn default() -> Self {
        Self::AutoIncrement
    }
}

impl IdStrategy {
    /// 创建 UUID 策略
    pub fn uuid() -> Self {
        Self::Uuid
    }

    /// 创建雪花算法策略
    #[cfg(feature = "snowflake-id")]
    pub fn snowflake(machine_id: u16, datacenter_id: u8) -> Self {
        Self::Snowflake {
            machine_id,
            datacenter_id,
        }
    }

    /// 创建 ObjectId 策略
    pub fn object_id() -> Self {
        Self::ObjectId
    }

    /// 创建自定义策略
    pub fn custom(generator: String) -> Self {
        Self::Custom(generator)
    }
}

impl From<i64> for IdType {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for IdType {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for IdType {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_type_parsing() {
        assert_eq!(DatabaseType::from_str("sqlite").unwrap(), DatabaseType::SQLite);
        assert_eq!(DatabaseType::from_str("postgresql").unwrap(), DatabaseType::PostgreSQL);
        assert_eq!(DatabaseType::from_str("mysql").unwrap(), DatabaseType::MySQL);
        assert_eq!(DatabaseType::from_str("mongodb").unwrap(), DatabaseType::MongoDB);
        
        assert!(DatabaseType::from_str("unknown").is_err());
    }

    #[test]
    fn test_data_value_serialization() {
        let value = DataValue::String("测试".to_string());
        let json = value.to_json_string().unwrap();
        let parsed = DataValue::from_json_string(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_data_value_type_name() {
        assert_eq!(DataValue::Null.type_name(), "null");
        assert_eq!(DataValue::Bool(true).type_name(), "bool");
        assert_eq!(DataValue::String("test".to_string()).type_name(), "string");
    }
}