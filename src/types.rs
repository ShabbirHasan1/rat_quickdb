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
        /// SSL 模式
        ssl_mode: Option<String>,
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
    },
    /// MongoDB 连接配置
    MongoDB {
        /// 连接字符串
        uri: String,
        /// 数据库名
        database: String,
        /// 认证数据库
        auth_source: Option<String>,
    },
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