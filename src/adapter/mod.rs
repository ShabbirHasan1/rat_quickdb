//! 数据库适配器模块
//!
//! 提供统一的数据库操作接口，屏蔽不同数据库的实现差异
//!
//! # 缓存架构重新设计说明
//!
//! **重要变更**：缓存架构已从包装器模式重新设计为内部集成模式。
//!
//! ## 旧模式（已废弃）
//! - 缓存作为 `DatabaseAdapter` 的包装器
//! - 使用 `CachedDatabaseAdapter` 包装真实适配器
//! - 缓存与数据库共享相同的 trait 定义
//!
//! ## 新模式（当前实现）
//! - 缓存提供通用方法供数据库适配器内部使用
//! - 各个数据库适配器在执行操作时自动调用缓存逻辑
//! - 用户使用 `UserCacheOperations` 手动清理缓存
//! - 开发者使用 `CacheOperations` 在适配器内部集成缓存逻辑
//!
//! ## 主要优势
//! 1. **架构清晰**：缓存不再是数据库，职责分离
//! 2. **性能优化**：减少不必要的包装层调用
//! 3. **可见性控制**：严格控制哪些方法可以暴露给用户
//! 4. **弹性设计**：数据库适配器可以灵活选择缓存策略

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::FieldType;
use crate::pool::DatabaseConnection;
use async_trait::async_trait;

use std::collections::HashMap;

// 导入各个数据库适配器
mod sqlite;
mod postgres;
mod mysql;
mod mongodb;
mod query_builder;

pub use sqlite::SqliteAdapter;
pub use postgres::PostgresAdapter;
pub use mysql::MysqlAdapter;
pub use mongodb::MongoAdapter;
pub use query_builder::*;

/// 数据库适配器trait，定义统一的数据库操作接口
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// 创建记录
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<DataValue>;

    /// 根据ID查找记录
    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>>;

    /// 查找记录
    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>>;

    /// 使用条件组合查找记录（支持OR逻辑）
    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>>;

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

    /// 删除表/集合
    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()>;
}

/// 根据数据库类型创建适配器
pub fn create_adapter(db_type: &DatabaseType) -> QuickDbResult<Box<dyn DatabaseAdapter>> {
    match db_type {
        DatabaseType::SQLite => Ok(Box::new(SqliteAdapter)),
        DatabaseType::MySQL => Ok(Box::new(MysqlAdapter::new())),
        DatabaseType::PostgreSQL => Ok(Box::new(PostgresAdapter)),
        DatabaseType::MongoDB => Ok(Box::new(MongoAdapter)),
        _ => Err(QuickDbError::UnsupportedDatabase {
            db_type: format!("{:?}", db_type),
        }),
    }
}