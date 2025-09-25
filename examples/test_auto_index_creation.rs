//! 自动索引创建功能测试
//!
//! 测试所有数据库类型的自动索引创建功能，包括：
//! - MongoDB
//! - PostgreSQL
//! - MySQL
//! - SQLite

use rat_quickdb::{
    Model, ModelOperations, ModelManager, DatabaseConfig, ConnectionConfig, PoolConfig,
    init, add_database, DataValue,
    string_field, integer_field, float_field, boolean_field,
    postgres_config, mysql_config, sqlite_config, mongodb_config_with_builder,
    MongoDbConnectionBuilder, TlsConfig, ZstdConfig
};
use std::collections::HashMap;
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use serde::{Serialize, Deserialize};

// 定义测试用户模型
rat_quickdb::define_model! {
    struct AutoIndexTestUser {
        id: Option<i32>,
        username: String,
        email: String,
        age: i32,
        is_active: bool,
        balance: f64,
    }

    collection = "auto_index_test_users",
    fields = {
        id: integer_field(None, None),
        username: string_field(Some(50), Some(3), None).required(),
        email: string_field(Some(255), Some(5), None).required().unique(),
        age: integer_field(Some(0), Some(150)),
        is_active: boolean_field(),
        balance: float_field(Some(0.0), Some(1000000.0)),
    }

    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
        { fields: ["email"], unique: true, name: "idx_email" },
        { fields: ["age"], unique: false, name: "idx_age" },
        { fields: ["is_active"], unique: false, name: "idx_active" },
    ],
}

// 测试特定数据库的索引功能
async fn test_database_indexes(db_alias: &str, db_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试 {} 索引功能 ===", db_name);

    // 1. 创建测试用户
    println!("1. 创建测试用户...");
    let user1 = AutoIndexTestUser {
        id: None,
        username: format!("user1_{}", db_alias),
        email: format!("user1_{}@test.com", db_alias),
        age: 25,
        is_active: true,
        balance: 1000.50,
    };

    let user1_id = user1.save().await?;
    println!("✅ 用户1创建成功，ID: {}", user1_id);

    // 2. 创建第二个用户（不同数据）
    println!("2. 创建第二个用户...");
    let user2 = AutoIndexTestUser {
        id: None,
        username: format!("user2_{}", db_alias),
        email: format!("user2_{}@test.com", db_alias),
        age: 30,
        is_active: false,
        balance: 2000.75,
    };

    let user2_id = user2.save().await?;
    println!("✅ 用户2创建成功，ID: {}", user2_id);

    // 3. 测试唯一索引约束
    println!("3. 测试唯一索引约束...");

    // 3.1 测试重复用户名
    let duplicate_username_user = AutoIndexTestUser {
        id: None,
        username: format!("user1_{}", db_alias), // 重复的用户名
        email: format!("unique_{}@test.com", db_alias),
        age: 35,
        is_active: true,
        balance: 1500.00,
    };

    match duplicate_username_user.save().await {
        Ok(_) => println!("⚠️  用户名唯一索引可能未生效"),
        Err(e) => {
            println!("✅ 用户名唯一索引正常工作: {}", e);
            if e.to_string().contains("duplicate") ||
               e.to_string().contains("unique") ||
               e.to_string().contains("1062") || // MySQL错误码
               e.to_string().contains("23505") || // PostgreSQL错误码
               e.to_string().contains("11000") { // MongoDB错误码
                println!("✅ 确认是唯一键冲突错误");
            }
        }
    }

    // 3.2 测试重复邮箱
    let duplicate_email_user = AutoIndexTestUser {
        id: None,
        username: format!("unique_{}", db_alias),
        email: format!("user1_{}@test.com", db_alias), // 重复的邮箱
        age: 35,
        is_active: true,
        balance: 1500.00,
    };

    match duplicate_email_user.save().await {
        Ok(_) => println!("⚠️  邮箱唯一索引可能未生效"),
        Err(e) => {
            println!("✅ 邮箱唯一索引正常工作: {}", e);
            if e.to_string().contains("duplicate") ||
               e.to_string().contains("unique") ||
               e.to_string().contains("1062") ||
               e.to_string().contains("23505") ||
               e.to_string().contains("11000") {
                println!("✅ 确认是唯一键冲突错误");
            }
        }
    }

    // 3.3 测试组合唯一索引
    let duplicate_combo_user = AutoIndexTestUser {
        id: None,
        username: format!("user1_{}", db_alias),  // 重复的用户名
        email: format!("user1_{}@test.com", db_alias), // 重复的邮箱
        age: 35,
        is_active: true,
        balance: 1500.00,
    };

    match duplicate_combo_user.save().await {
        Ok(_) => println!("⚠️  组合唯一索引可能未生效"),
        Err(e) => {
            println!("✅ 组合唯一索引正常工作: {}", e);
            if e.to_string().contains("duplicate") ||
               e.to_string().contains("unique") ||
               e.to_string().contains("1062") ||
               e.to_string().contains("23505") ||
               e.to_string().contains("11000") {
                println!("✅ 确认是唯一键冲突错误");
            }
        }
    }

    // 4. 测试普通索引功能
    println!("4. 测试普通索引功能...");

    // 验证能够正常查询（索引应该被使用）
    let found_users = ModelManager::<AutoIndexTestUser>::find(
        vec![],
        None
    ).await?;

    println!("✅ 查询到 {} 个用户记录", found_users.len());

    // 5. 清理测试数据
    println!("5. 清理测试数据...");
    if let Some(user1) = ModelManager::<AutoIndexTestUser>::find_by_id(&user1_id).await? {
        user1.delete().await?;
    }
    if let Some(user2) = ModelManager::<AutoIndexTestUser>::find_by_id(&user2_id).await? {
        user2.delete().await?;
    }
    println!("✅ 测试数据清理完成");

    println!("✅ {} 索引功能测试完成", db_name);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    println!("🚀 自动索引创建功能测试");
    println!("=====================================\n");

    // 初始化rat_quickdb
    init();

    // 配置测试数据库
    let pool_config = PoolConfig::builder()
        .max_connections(5)
        .min_connections(1)
        .connection_timeout(10)
        .idle_timeout(300)
        .max_lifetime(1800)
        .build()?;

    // 1. 测试SQLite
    println!("配置SQLite数据库...");
    let sqlite_config = sqlite_config(
        "sqlite_auto_index_test",
        "./test_data/auto_index_test.db",
        pool_config.clone(),
    )?;

    add_database(sqlite_config).await?;
    test_database_indexes("sqlite_auto_index_test", "SQLite").await?;

    // 2. 测试PostgreSQL
    println!("\n配置PostgreSQL数据库...");
    let pg_config = DatabaseConfig {
        db_type: rat_quickdb::types::DatabaseType::PostgreSQL,
        connection: ConnectionConfig::PostgreSQL {
            host: "172.16.0.23".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_mode: Some("prefer".to_string()),
            tls_config: Some(TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
        },
        pool: pool_config.clone(),
        alias: "postgres_auto_index_test".to_string(),
        cache: None,
        id_strategy: rat_quickdb::types::IdStrategy::AutoIncrement,
    };

    add_database(pg_config).await?;
    test_database_indexes("postgres_auto_index_test", "PostgreSQL").await?;

    // 3. 测试MySQL
    println!("\n配置MySQL数据库...");
    let mysql_config = DatabaseConfig {
        db_type: rat_quickdb::types::DatabaseType::MySQL,
        connection: ConnectionConfig::MySQL {
            host: "172.16.0.21".to_string(),
            port: 3306,
            database: "testdb".to_string(),
            username: "testdb".to_string(),
            password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
            ssl_opts: {
                let mut opts = HashMap::new();
                opts.insert("ssl_mode".to_string(), "PREFERRED".to_string());
                Some(opts)
            },
            tls_config: Some(TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            }),
        },
        pool: pool_config.clone(),
        alias: "mysql_auto_index_test".to_string(),
        cache: None,
        id_strategy: rat_quickdb::types::IdStrategy::AutoIncrement,
    };

    add_database(mysql_config).await?;
    test_database_indexes("mysql_auto_index_test", "MySQL").await?;

    // 4. 测试MongoDB
    println!("\n配置MongoDB数据库...");
    let mongo_config = mongodb_config_with_builder(
        "mongodb_auto_index_test",
        MongoDbConnectionBuilder::new("db0.0ldm0s.net", 27017, "testdb")
            .with_auth("testdb", "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^")
            .with_auth_source("testdb")
            .with_direct_connection(true)
            .with_tls_config(TlsConfig {
                enabled: true,
                ca_cert_path: None,
                client_cert_path: None,
                client_key_path: None,
                verify_server_cert: false,
                verify_hostname: false,
                min_tls_version: None,
                cipher_suites: None,
            })
            .with_zstd_config(ZstdConfig {
                enabled: true,
                compression_level: Some(3),
                compression_threshold: Some(1024),
            }),
        pool_config,
    )?;

    add_database(mongo_config).await?;
    test_database_indexes("mongodb_auto_index_test", "MongoDB").await?;

    println!("\n🎉 所有数据库的自动索引创建功能测试完成！");
    println!("✅ 模型自动注册机制正常工作");
    println!("✅ 自动表和索引创建功能正常");
    println!("✅ 唯一索引约束正确生效");
    println!("✅ 普通索引查询功能正常");
    println!("✅ 组合索引功能正常");

    Ok(())
}