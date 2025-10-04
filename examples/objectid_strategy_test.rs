//! ObjectId策略测试示例
//!
//! 本示例测试ObjectId策略在不同数据库下的工作情况：
//! - SQLite (SQL数据库) - 使用MongoDB ObjectId生成器生成ObjectId字符串
//! - MongoDB (NoSQL数据库) - 让系统自己处理ObjectId生成

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, IdStrategy};
use rat_quickdb::manager::{get_global_pool_manager};
use rat_quickdb::{ModelManager, ModelOperations, string_field, integer_field, datetime_field};
use rat_logger::{LoggerBuilder, handler::term::TermConfig};
use serde::{Serialize, Deserialize};
use chrono::{Utc, DateTime};

// 定义测试模型
define_model! {
    /// 测试用户模型
    struct ObjectIdTestUser {
        id: String,
        username: String,
        email: String,
        created_at: DateTime<Utc>,
    }
    collection = "objectid_test_users",
    fields = {
        id: string_field(None, None, None).required().unique(),
        username: string_field(None, None, None).required(),
        email: string_field(None, None, None).required(),
        created_at: datetime_field().required(),
    }
    indexes = [
        { fields: ["username"], unique: true, name: "idx_username" },
    ],
}

impl ObjectIdTestUser {
    /// 创建测试用户（ID为空以触发自动生成）
    fn new(username: &str, email: &str) -> Self {
        Self {
            id: String::new(), // 空ID，测试自动生成
            username: username.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
        }
    }
}

/// 清理测试文件
async fn cleanup_test_files() {
    let test_files = vec![
        "./objectid_test.db",
        "./objectid_test.db-wal",
        "./objectid_test.db-shm",
    ];

    for file in test_files {
        if let Err(e) = tokio::fs::remove_file(file).await {
            if !e.to_string().contains("No such file or directory") {
                eprintln!("警告：无法删除测试文件 {}: {}", file, e);
            }
        }
    }
}

/// 测试SQLite数据库的ObjectId策略
async fn test_sqlite_objectid() -> QuickDbResult<()> {
    println!("🗄️ 测试 SQLite + ObjectId 策略");
    println!("===============================");

    // 配置SQLite数据库，使用ObjectId策略
    let db_config = DatabaseConfig {
        alias: "sqlite_objectid".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./objectid_test.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::ObjectId,
        cache: None,
    };

    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;

    // 创建测试用户
    let users = vec![
        ObjectIdTestUser::new("sqlite_user1", "sqlite1@test.com"),
        ObjectIdTestUser::new("sqlite_user2", "sqlite2@test.com"),
        ObjectIdTestUser::new("sqlite_user3", "sqlite3@test.com"),
    ];

    println!("创建3个用户，测试ObjectId自动生成...");
    let mut created_ids = Vec::new();

    for (i, user) in users.iter().enumerate() {
        match user.save().await {
            Ok(id) => {
                println!("✅ 用户 {} 创建成功，生成的ID: {}", i + 1, id);
                created_ids.push(id);
            },
            Err(e) => {
                println!("❌ 用户 {} 创建失败: {}", i + 1, e);
                return Err(e);
            }
        }
    }

    // 验证ID是否是有效的ObjectId格式
    println!("\n验证ID是否为有效ObjectId:");
    for (i, id) in created_ids.iter().enumerate() {
        println!("用户 {} ID: {}", i + 1, id);
        if id.len() == 24 {
            println!("  ✅ ID长度正确 (24字符)");
            if id.chars().all(|c| c.is_ascii_hexdigit()) {
                println!("  ✅ ObjectId格式正确 (16进制字符串)");
            } else {
                println!("  ❌ ObjectId格式错误 (包含非16进制字符)");
            }
        } else {
            println!("  ❌ ID长度错误: {}", id.len());
        }
    }

    // 清理数据
    let _ = rat_quickdb::delete("objectid_test_users", vec![], Some("sqlite_objectid")).await;
    println!("✅ SQLite ObjectId 测试完成\n");

    Ok(())
}

/// 测试MongoDB数据库的ObjectId策略
async fn test_mongodb_objectid() -> QuickDbResult<()> {
    println!("🍃 测试 MongoDB + ObjectId 策略");
    println!("===============================");

    // 配置MongoDB数据库，使用ObjectId策略
    let db_config = DatabaseConfig {
        alias: "mongodb_objectid".to_string(),
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "localhost".to_string(),
            port: 27017,
            database: "objectid_test".to_string(),
            username: None,
            password: None,
            auth_source: None,
            direct_connection: false,
            tls_config: None,
            zstd_config: None,
            options: None,
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::ObjectId,
        cache: None,
    };

    let pool_manager = get_global_pool_manager();

    // 尝试连接MongoDB，如果失败则跳过测试
    match pool_manager.add_database(db_config).await {
        Ok(_) => {
            println!("✅ MongoDB连接成功");

            // 创建测试用户
            let users = vec![
                ObjectIdTestUser::new("mongo_user1", "mongo1@test.com"),
                ObjectIdTestUser::new("mongo_user2", "mongo2@test.com"),
                ObjectIdTestUser::new("mongo_user3", "mongo3@test.com"),
            ];

            println!("创建3个用户，测试MongoDB系统自生成ObjectId...");
            let mut created_ids = Vec::new();

            for (i, user) in users.iter().enumerate() {
                match user.save().await {
                    Ok(id) => {
                        println!("✅ 用户 {} 创建成功，系统生成的ID: {}", i + 1, id);
                        created_ids.push(id);
                    },
                    Err(e) => {
                        println!("❌ 用户 {} 创建失败: {}", i + 1, e);
                        return Err(e);
                    }
                }
            }

            // 验证MongoDB系统生成的ObjectId
            println!("\n验证MongoDB系统生成的ObjectId:");
            for (i, id) in created_ids.iter().enumerate() {
                println!("用户 {} ID: {}", i + 1, id);
                if id.len() == 24 {
                    println!("  ✅ ID长度正确 (24字符)");
                    if id.chars().all(|c| c.is_ascii_hexdigit()) {
                        println!("  ✅ ObjectId格式正确 (16进制字符串)");
                    } else {
                        println!("  ❌ ObjectId格式错误 (包含非16进制字符)");
                    }
                } else {
                    println!("  ❌ ID长度错误: {}", id.len());
                }
            }

            // 清理数据
            let _ = rat_quickdb::delete("objectid_test_users", vec![], Some("mongodb_objectid")).await;
            println!("✅ MongoDB ObjectId 测试完成\n");
        },
        Err(e) => {
            println!("⚠️ MongoDB连接失败，跳过MongoDB测试: {}", e);
            println!("这是正常的，如果没有运行MongoDB服务器的话。\n");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🧪 RatQuickDB ObjectId策略测试");
    println!("=============================\n");

    // 清理之前的测试文件
    cleanup_test_files().await;

    // 测试不同数据库的ObjectId策略
    test_sqlite_objectid().await?;
    test_mongodb_objectid().await?;

    // 清理测试文件
    cleanup_test_files().await;

    println!("🎉 所有ObjectId策略测试完成！");
    Ok(())
}