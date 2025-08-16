//! rat_quickdb - 跨数据库ORM库
//! 
//! 提供统一的数据库操作接口，支持SQLite、PostgreSQL、MySQL和MongoDB
//! 通过连接池和无锁队列实现高性能的数据后端无关性

// 导出所有公共模块
pub mod error;
pub mod types;
pub mod pool;
pub mod manager;
pub mod odm;
pub mod model;
pub mod serializer;
pub mod adapter;
pub mod config;
pub mod task_queue;
pub mod table;

// 条件编译的模块
pub mod cache;
pub mod id_generator;

// 重新导出常用类型和函数
pub use error::{QuickDbError, QuickDbResult};
pub use types::*;
pub use pool::{ConnectionPool, DatabaseConnection};
pub use manager::{
    PoolManager, add_database, remove_database, get_connection, release_connection,
    get_aliases, set_default_alias, health_check, shutdown,
    get_id_generator, get_mongo_auto_increment_generator
};

pub use manager::{
    get_cache_manager, get_cache_stats, clear_cache, clear_all_caches
};
pub use odm::{AsyncOdmManager, OdmOperations, get_odm_manager, get_odm_manager_mut};
pub use model::{Model, ModelOperations, ModelManager, FieldType, FieldDefinition, ModelMeta, IndexDefinition};
pub use serializer::{DataSerializer, SerializerConfig, OutputFormat, SerializationResult};
pub use adapter::{DatabaseAdapter, create_adapter};
pub use config::{
    GlobalConfig, GlobalConfigBuilder, DatabaseConfigBuilder, PoolConfigBuilder,
    AppConfig, AppConfigBuilder, LoggingConfig, LoggingConfigBuilder,
    Environment, LogLevel, sqlite_config, postgres_config, mysql_config, 
    mongodb_config, default_pool_config
};
pub use task_queue::{
    TaskQueueManager, get_global_task_queue, initialize_global_task_queue, 
    shutdown_global_task_queue
};
pub use table::{TableManager, TableSchema, ColumnDefinition, ColumnType, IndexType};

// 条件导出缓存相关类型
pub use cache::{CacheManager, CacheStats};

// 导出ID生成器相关类型
pub use id_generator::{IdGenerator, MongoAutoIncrementGenerator};

// 重新导出便捷函数
pub use odm::{create, find_by_id, find, update, update_by_id, delete, delete_by_id, count, exists};

// 初始化日志
use zerg_creep::{info, init_logger};

/// 初始化rat_quickdb库
/// 
/// 这个函数会初始化日志系统
pub fn init() {
    init_logger();
    info!("rat_quickdb库已初始化");
}

/// 库版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 库名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 获取库信息
pub fn get_info() -> String {
    format!("{} v{}", NAME, VERSION)
}
