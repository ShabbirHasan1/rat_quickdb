//! 数据库适配器模块
//! 
//! 提供统一的数据库操作接口，屏蔽不同数据库的实现差异

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::FieldType;
use crate::pool::DatabaseConnection;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

// 导入各个数据库适配器
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "postgresql")]
mod postgres;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "mongodb")]
mod mongodb;
mod query_builder;
#[cfg(feature = "cache")]
mod cached;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteAdapter;
#[cfg(feature = "postgresql")]
pub use postgres::PostgresAdapter;
#[cfg(feature = "mysql")]
pub use mysql::MysqlAdapter;
#[cfg(feature = "mongodb")]
pub use mongodb::MongoAdapter;
pub use query_builder::*;
#[cfg(feature = "cache")]
pub use cached::CachedDatabaseAdapter;

/// 数据库适配器trait，定义统一的数据库操作接口
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// 创建记录
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<Value>;

    /// 根据ID查找记录
    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<Value>>;

    /// 查找记录
    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<Value>>;

    /// 更新记录
    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64>;

    /// 根据ID更新记录
    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool>;

    /// 删除记录
    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64>;

    /// 根据ID删除记录
    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool>;

    /// 统计记录数量
    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64>;

    /// 检查记录是否存在
    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool>;

    /// 创建表/集合
    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldType>,
    ) -> QuickDbResult<()>;

    /// 创建索引
    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()>;

    /// 检查表是否存在
    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool>;
}

/// 根据数据库类型创建适配器
pub fn create_adapter(db_type: &DatabaseType) -> QuickDbResult<Box<dyn DatabaseAdapter>> {
    match db_type {
        #[cfg(feature = "sqlite")]
        DatabaseType::SQLite => Ok(Box::new(SqliteAdapter)),
        
        #[cfg(feature = "mysql")]
        DatabaseType::MySQL => Ok(Box::new(MysqlAdapter::new())),
        
        #[cfg(feature = "postgresql")]
        DatabaseType::PostgreSQL => Ok(Box::new(PostgresAdapter)),
        
        #[cfg(feature = "mongodb")]
        DatabaseType::MongoDB => Ok(Box::new(MongoAdapter)),
        
        _ => Err(QuickDbError::UnsupportedDatabase {
            db_type: format!("{:?}", db_type),
        }),
    }
}

/// 根据数据库类型和缓存管理器创建带缓存的适配器
#[cfg(feature = "cache")]
pub fn create_adapter_with_cache(
    db_type: &DatabaseType,
    cache_manager: std::sync::Arc<crate::cache::CacheManager>,
) -> QuickDbResult<Box<dyn DatabaseAdapter>> {
    let base_adapter = create_adapter(db_type)?;
    Ok(Box::new(CachedDatabaseAdapter::new(base_adapter, cache_manager)))
}