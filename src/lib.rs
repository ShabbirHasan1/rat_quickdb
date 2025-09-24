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

// Python API 模块（仅在启用 python-bindings 特性时编译）
#[cfg(feature = "python-bindings")]
pub mod python_api;

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
pub use model::{
    Model, ModelOperations, ModelManager, FieldType, FieldDefinition, ModelMeta, IndexDefinition,
    array_field, list_field, string_field, integer_field, float_field, boolean_field,
    datetime_field, uuid_field, json_field, dict_field, reference_field
};
pub use serializer::{DataSerializer, SerializerConfig, OutputFormat, SerializationResult};
pub use adapter::{DatabaseAdapter, create_adapter};
pub use config::{
    GlobalConfig, GlobalConfigBuilder, DatabaseConfigBuilder, PoolConfigBuilder,
    AppConfig, AppConfigBuilder, LoggingConfig, LoggingConfigBuilder,
    Environment, LogLevel, sqlite_config, postgres_config, mysql_config,
    mongodb_config
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

// 重新导出 ODM 操作函数（底层 DataValue API）
// 注意：这些函数返回 DataValue，主要用于内部使用
// 对于应用开发，推荐使用 ModelManager<T> 获取结构化数据
pub use odm::{create, find_by_id, find, update, update_by_id, delete, delete_by_id, count, exists};

// Python API 导出（仅在启用 python-bindings 特性时）
// 注意：Python绑定相关的导出已移至专门的Python绑定库中

// 日志系统导入
use rat_logger::{LoggerBuilder, LevelFilter, info};


/// 初始化rat_quickdb库
/// 
/// 这个函数会初始化rat_quickdb库，可以选择性地初始化日志系统
/// 
/// 参数:
/// - init_logging: 是否初始化日志系统，设为false时由上层应用显式初始化
pub fn init(init_logging: bool) {
    if init_logging {
        LoggerBuilder::new()
            .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
            .init()
            .expect("日志初始化失败");
        info!("rat_quickdb库已初始化（包含日志系统）");
    }
    // 不初始化日志时不输出任何信息，由上层应用管理
}

/// 初始化rat_quickdb库并设置日志级别
/// 
/// 参数:
/// - level: 日志级别过滤器
pub fn init_with_log_level(level: rat_logger::LevelFilter) {
    LoggerBuilder::new()
        .with_level(level)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("日志初始化失败");
    info!("rat_quickdb库已初始化，日志级别: {:?}", level);
}

/// 库版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 库名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 获取库信息
pub fn get_info() -> String {
    format!("{} v{}", NAME, VERSION)
}
