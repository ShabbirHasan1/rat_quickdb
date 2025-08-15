//! # 配置系统使用示例
//!
//! 展示 RatQuickDB 的配置系统功能，包括：
//! - 构建器模式配置
//! - 链式配置API
//! - 配置文件加载和保存
//! - 多数据库配置管理

use rat_quickdb::{
    GlobalConfig, DatabaseConfigBuilder, PoolConfigBuilder, AppConfigBuilder, 
    LoggingConfigBuilder, Environment, LogLevel, ConnectionConfig, DatabaseType,
    sqlite_config, postgres_config, mysql_config, mongodb_config, default_pool_config,
    QuickDbError
};
use std::path::PathBuf;
use zerg_creep::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), QuickDbError> {
    // 初始化日志系统
    rat_quickdb::init();
    
    info!("开始配置系统使用示例");
    
    // 演示构建器模式配置
    demonstrate_builder_pattern().await?;
    
    // 演示链式配置API
    demonstrate_chain_configuration().await?;
    
    // 演示配置文件操作
    demonstrate_config_file_operations().await?;
    
    // 演示多数据库配置
    demonstrate_multi_database_config().await?;
    
    // 演示便捷配置函数
    demonstrate_convenience_functions().await?;
    
    info!("配置系统使用示例完成");
    Ok(())
}

/// 演示构建器模式配置
async fn demonstrate_builder_pattern() -> Result<(), QuickDbError> {
    info!("=== 演示构建器模式配置 ===");
    
    // 1. 创建连接池配置
    let pool_config = PoolConfigBuilder::new()
        .min_connections(2)
        .max_connections(10)
        .connection_timeout(30)
        .idle_timeout(600)
        .max_lifetime(3600)
        .build()?;
    
    info!("连接池配置创建成功: {:?}", pool_config);
    
    // 2. 创建数据库配置
    let db_config = DatabaseConfigBuilder::new()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./example.db".to_string(),
            create_if_missing: true,
        })
        .pool(pool_config)
        .alias("main_db")
        .build()?;
    
    info!("数据库配置创建成功: 别名={}", db_config.alias);
    
    // 3. 创建应用配置
    let app_config = AppConfigBuilder::new()
        .name("RatQuickDB示例应用")
        .version("1.0.0")
        .environment(Environment::Development)
        .debug(true)
        .work_dir(PathBuf::from("./"))
        .build()?;
    
    info!("应用配置创建成功: 名称={}", app_config.name);
    
    // 4. 创建日志配置
    let logging_config = LoggingConfigBuilder::new()
        .level(LogLevel::Info)
        .console(true)
        .file_path(Some(PathBuf::from("./logs/app.log")))
        .max_file_size(10 * 1024 * 1024) // 10MB
        .max_files(5)
        .structured(false)
        .build()?;
    
    info!("日志配置创建成功: 级别={:?}", logging_config.level);
    
    // 5. 创建全局配置
    let global_config = GlobalConfig::builder()
        .add_database(db_config)
        .default_database("main_db")
        .app(app_config)
        .logging(logging_config)
        .build()?;
    
    info!("全局配置创建成功，包含 {} 个数据库", global_config.databases.len());
    
    Ok(())
}

/// 演示链式配置API
async fn demonstrate_chain_configuration() -> Result<(), QuickDbError> {
    info!("=== 演示链式配置API ===");
    
    // 链式创建完整的全局配置
    let global_config = GlobalConfig::builder()
        // 添加SQLite数据库
        .add_database(
            DatabaseConfigBuilder::new()
                .db_type(DatabaseType::SQLite)
                .connection(ConnectionConfig::SQLite {
                    path: "./chain_example.db".to_string(),
                    create_if_missing: true,
                })
                .pool(
                    PoolConfigBuilder::new()
                        .min_connections(1)
                        .max_connections(5)
                        .connection_timeout(30)
                        .idle_timeout(300)
                        .max_lifetime(1800)
                        .build()?
                )
                .alias("sqlite_db")
                .build()?
        )
        // 添加PostgreSQL数据库
        .add_database(
            DatabaseConfigBuilder::new()
                .db_type(DatabaseType::PostgreSQL)
                .connection(ConnectionConfig::PostgreSQL {
                    host: "localhost".to_string(),
                    port: 5432,
                    database: "testdb".to_string(),
                    username: "postgres".to_string(),
                    password: "password".to_string(),
                    ssl_mode: Some("prefer".to_string()),
                    tls_config: None,
                })
                .pool(
                    PoolConfigBuilder::new()
                        .min_connections(2)
                        .max_connections(20)
                        .connection_timeout(30)
                        .idle_timeout(600)
                        .max_lifetime(3600)
                        .build()?
                )
                .alias("postgres_db")
                .build()?
        )
        // 设置默认数据库
        .default_database("sqlite_db")
        // 设置应用配置
        .app(
            AppConfigBuilder::new()
                .name("链式配置示例")
                .version("2.0.0")
                .environment(Environment::Testing)
                .debug(false)
                .work_dir(PathBuf::from("./chain_example"))
                .build()?
        )
        // 设置日志配置
        .logging(
            LoggingConfigBuilder::new()
                .level(LogLevel::Debug)
                .console(true)
                .file_path(Some(PathBuf::from("./logs/chain.log")))
                .max_file_size(5 * 1024 * 1024) // 5MB
                .max_files(3)
                .structured(true)
                .build()?
        )
        .build()?;
    
    info!("链式配置创建成功，默认数据库: {:?}", global_config.default_database);
    
    // 验证配置
    let default_db = global_config.get_default_database()?;
    info!("默认数据库配置: 别名={}, 类型={:?}", default_db.alias, default_db.db_type);
    
    let postgres_db = global_config.get_database("postgres_db")?;
    info!("PostgreSQL数据库配置: 别名={}, 类型={:?}", postgres_db.alias, postgres_db.db_type);
    
    Ok(())
}

/// 演示配置文件操作
async fn demonstrate_config_file_operations() -> Result<(), QuickDbError> {
    info!("=== 演示配置文件操作 ===");
    
    // 创建示例配置
    let config = GlobalConfig::builder()
        .add_database(
            DatabaseConfigBuilder::new()
                .db_type(DatabaseType::SQLite)
                .connection(ConnectionConfig::SQLite {
                    path: "./file_example.db".to_string(),
                    create_if_missing: true,
                })
                .pool(
                    PoolConfigBuilder::new()
                        .min_connections(1)
                        .max_connections(10)
                        .connection_timeout(30)
                        .idle_timeout(600)
                        .max_lifetime(3600)
                        .build()?
                )
                .alias("file_db")
                .build()?
        )
        .default_database("file_db")
        .app(
            AppConfigBuilder::new()
                .name("文件配置示例")
                .version("1.0.0")
                .environment(Environment::Production)
                .debug(false)
                .work_dir(PathBuf::from("./"))
                .build()?
        )
        .logging(
            LoggingConfigBuilder::new()
                .level(LogLevel::Warn)
                .console(false)
                .file_path(Some(PathBuf::from("./logs/prod.log")))
                .max_file_size(20 * 1024 * 1024) // 20MB
                .max_files(10)
                .structured(true)
                .build()?
        )
        .build()?;
    
    // 保存为JSON文件
    let json_path = "./config_example.json";
    config.save_to_file(json_path)?;
    info!("配置已保存到JSON文件: {}", json_path);
    
    // 保存为TOML文件
    let toml_path = "./config_example.toml";
    config.save_to_file(toml_path)?;
    info!("配置已保存到TOML文件: {}", toml_path);
    
    // 从JSON文件加载配置
    match GlobalConfig::from_file(json_path) {
        Ok(loaded_config) => {
            info!("从JSON文件加载配置成功，应用名称: {}", loaded_config.app.name);
        }
        Err(e) => {
            warn!("从JSON文件加载配置失败: {}", e);
        }
    }
    
    // 从TOML文件加载配置
    match GlobalConfig::from_file(toml_path) {
        Ok(loaded_config) => {
            info!("从TOML文件加载配置成功，应用名称: {}", loaded_config.app.name);
        }
        Err(e) => {
            warn!("从TOML文件加载配置失败: {}", e);
        }
    }
    
    // 清理文件
    let _ = std::fs::remove_file(json_path);
    let _ = std::fs::remove_file(toml_path);
    
    Ok(())
}

/// 演示多数据库配置
async fn demonstrate_multi_database_config() -> Result<(), QuickDbError> {
    info!("=== 演示多数据库配置 ===");
    
    // 创建多个数据库配置
    let mut config_builder = GlobalConfig::builder();
    
    // 添加用户数据库（SQLite）
    let user_db = DatabaseConfigBuilder::new()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: "./users.db".to_string(),
            create_if_missing: true,
        })
        .pool(
            PoolConfigBuilder::new()
                .min_connections(1)
                .max_connections(5)
                .connection_timeout(30)
                .idle_timeout(300)
                .max_lifetime(1800)
                .build()?
        )
        .alias("users")
        .build()?;
    
    config_builder = config_builder.add_database(user_db);
    
    // 添加产品数据库（PostgreSQL）
    let product_db = DatabaseConfigBuilder::new()
        .db_type(DatabaseType::PostgreSQL)
        .connection(ConnectionConfig::PostgreSQL {
            host: "localhost".to_string(),
            port: 5432,
            database: "products".to_string(),
            username: "postgres".to_string(),
            password: "password".to_string(),
            ssl_mode: Some("require".to_string()),
            tls_config: None,
        })
        .pool(
            PoolConfigBuilder::new()
                .min_connections(2)
                .max_connections(15)
                .connection_timeout(30)
                .idle_timeout(600)
                .max_lifetime(3600)
                .build()?
        )
        .alias("products")
        .build()?;
    
    config_builder = config_builder.add_database(product_db);
    
    // 添加订单数据库（MySQL）
    let order_db = DatabaseConfigBuilder::new()
        .db_type(DatabaseType::MySQL)
        .connection(ConnectionConfig::MySQL {
            host: "localhost".to_string(),
            port: 3306,
            database: "orders".to_string(),
            username: "root".to_string(),
            password: "password".to_string(),
            ssl_opts: None,
            tls_config: None,
        })
        .pool(
            PoolConfigBuilder::new()
                .min_connections(3)
                .max_connections(25)
                .connection_timeout(30)
                .idle_timeout(600)
                .max_lifetime(3600)
                .build()?
        )
        .alias("orders")
        .build()?;
    
    config_builder = config_builder.add_database(order_db);
    
    // 添加日志数据库（MongoDB）
    let log_db = DatabaseConfigBuilder::new()
        .db_type(DatabaseType::MongoDB)
        .connection(ConnectionConfig::MongoDB {
            uri: "mongodb://localhost:27017".to_string(),
            database: "logs".to_string(),
            auth_source: None,
            tls_config: None,
            zstd_config: None,
        })
        .pool(
            PoolConfigBuilder::new()
                .min_connections(1)
                .max_connections(10)
                .connection_timeout(30)
                .idle_timeout(600)
                .max_lifetime(3600)
                .build()?
        )
        .alias("logs")
        .build()?;
    
    config_builder = config_builder.add_database(log_db);
    
    // 完成配置
    let global_config = config_builder
        .default_database("users")
        .app(
            AppConfigBuilder::new()
                .name("多数据库示例")
                .version("1.0.0")
                .environment(Environment::Development)
                .debug(true)
                .work_dir(PathBuf::from("./multi_db"))
                .build()?
        )
        .logging(
            LoggingConfigBuilder::new()
                .level(LogLevel::Info)
                .console(true)
                .file_path(Some(PathBuf::from("./logs/multi_db.log")))
                .max_file_size(10 * 1024 * 1024)
                .max_files(5)
                .structured(false)
                .build()?
        )
        .build()?;
    
    info!("多数据库配置创建成功，包含 {} 个数据库:", global_config.databases.len());
    
    for (alias, db_config) in &global_config.databases {
        info!("  - {}: {:?}", alias, db_config.db_type);
    }
    
    // 验证各个数据库配置
    let users_db = global_config.get_database("users")?;
    info!("用户数据库: 类型={:?}, 最大连接数={}", users_db.db_type, users_db.pool.max_connections);
    
    let products_db = global_config.get_database("products")?;
    info!("产品数据库: 类型={:?}, 最大连接数={}", products_db.db_type, products_db.pool.max_connections);
    
    let orders_db = global_config.get_database("orders")?;
    info!("订单数据库: 类型={:?}, 最大连接数={}", orders_db.db_type, orders_db.pool.max_connections);
    
    let logs_db = global_config.get_database("logs")?;
    info!("日志数据库: 类型={:?}, 最大连接数={}", logs_db.db_type, logs_db.pool.max_connections);
    
    Ok(())
}

/// 演示便捷配置函数
async fn demonstrate_convenience_functions() -> Result<(), QuickDbError> {
    info!("=== 演示便捷配置函数 ===");
    
    // 创建默认连接池配置
    let pool_config = default_pool_config(2, 10)?;
    info!("默认连接池配置: 最小连接数={}, 最大连接数={}", 
          pool_config.min_connections, pool_config.max_connections);
    
    // 使用便捷函数创建SQLite配置
    let sqlite_config = sqlite_config("convenience_sqlite", "./convenience.db", pool_config.clone())?;
    info!("SQLite配置: 别名={}, 类型={:?}", sqlite_config.alias, sqlite_config.db_type);
    
    // 使用便捷函数创建PostgreSQL配置
    let postgres_config = postgres_config(
        "convenience_postgres",
        "localhost",
        5432,
        "testdb",
        "postgres",
        "password",
        pool_config.clone(),
    )?;
    info!("PostgreSQL配置: 别名={}, 类型={:?}", postgres_config.alias, postgres_config.db_type);
    
    // 使用便捷函数创建MySQL配置
    let mysql_config = mysql_config(
        "convenience_mysql",
        "localhost",
        3306,
        "testdb",
        "root",
        "password",
        pool_config.clone(),
    )?;
    info!("MySQL配置: 别名={}, 类型={:?}", mysql_config.alias, mysql_config.db_type);
    
    // 使用便捷函数创建MongoDB配置
    let mongodb_config = mongodb_config(
        "convenience_mongodb",
        "mongodb://localhost:27017",
        "testdb",
        pool_config,
    )?;
    info!("MongoDB配置: 别名={}, 类型={:?}", mongodb_config.alias, mongodb_config.db_type);
    
    // 创建包含所有便捷配置的全局配置
    let global_config = GlobalConfig::builder()
        .add_database(sqlite_config)
        .add_database(postgres_config)
        .add_database(mysql_config)
        .add_database(mongodb_config)
        .default_database("convenience_sqlite")
        .app(
            AppConfigBuilder::new()
                .name("便捷函数示例")
                .version("1.0.0")
                .environment(Environment::Development)
                .debug(true)
                .work_dir(PathBuf::from("./convenience"))
                .build()?
        )
        .logging(
            LoggingConfigBuilder::new()
                .level(LogLevel::Debug)
                .console(true)
                .file_path::<String>(None)
                .max_file_size(1024 * 1024)
                .max_files(1)
                .structured(false)
                .build()?
        )
        .build()?;
    
    info!("便捷函数配置创建成功，包含 {} 个数据库", global_config.databases.len());
    
    // 演示配置验证
    demonstrate_config_validation().await?;
    
    Ok(())
}

/// 演示配置验证
async fn demonstrate_config_validation() -> Result<(), QuickDbError> {
    info!("=== 演示配置验证 ===");
    
    // 测试无效的连接池配置
    info!("测试无效的连接池配置...");
    
    match PoolConfigBuilder::new()
        .min_connections(10)
        .max_connections(5) // 最小连接数大于最大连接数
        .connection_timeout(30)
        .idle_timeout(600)
        .max_lifetime(3600)
        .build() {
        Ok(_) => error!("应该失败但成功了"),
        Err(e) => info!("预期的验证错误: {}", e),
    }
    
    // 测试零超时时间
    match PoolConfigBuilder::new()
        .min_connections(1)
        .max_connections(5)
        .connection_timeout(0) // 零超时时间
        .idle_timeout(600)
        .max_lifetime(3600)
        .build() {
        Ok(_) => error!("应该失败但成功了"),
        Err(e) => info!("预期的验证错误: {}", e),
    }
    
    // 测试数据库类型与连接配置不匹配
    match DatabaseConfigBuilder::new()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::PostgreSQL {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "postgres".to_string(),
            password: "password".to_string(),
            ssl_mode: None,
            tls_config: None,
        })
        .pool(default_pool_config(1, 5)?)
        .alias("mismatch_test")
        .build() {
        Ok(_) => error!("应该失败但成功了"),
        Err(e) => info!("预期的验证错误: {}", e),
    }
    
    // 测试缺少必需配置项
    match DatabaseConfigBuilder::new()
        .db_type(DatabaseType::SQLite)
        // 缺少连接配置
        .pool(default_pool_config(1, 5)?)
        .alias("incomplete_test")
        .build() {
        Ok(_) => error!("应该失败但成功了"),
        Err(e) => info!("预期的验证错误: {}", e),
    }
    
    // 测试无效的默认数据库
    match GlobalConfig::builder()
        .add_database(sqlite_config("test_db", "./test.db", default_pool_config(1, 5)?)?)
        .default_database("nonexistent_db") // 不存在的数据库别名
        .app(
            AppConfigBuilder::new()
                .name("测试应用")
                .version("1.0.0")
                .environment(Environment::Development)
                .debug(true)
                .work_dir(PathBuf::from("./"))
                .build()?
        )
        .logging(
            LoggingConfigBuilder::new()
                .level(LogLevel::Info)
                .console(true)
                .file_path::<String>(None)
                .max_file_size(1024)
                .max_files(1)
                .structured(false)
                .build()?
        )
        .build() {
        Ok(_) => error!("应该失败但成功了"),
        Err(e) => info!("预期的验证错误: {}", e),
    }
    
    info!("配置验证测试完成");
    Ok(())
}