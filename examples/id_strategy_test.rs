//! ID策略测试示例
//!
//! 本示例测试不同的ID生成策略是否能正常工作：
//! - AutoIncrement (自增数字)
//! - UUID (字符串)
//! - Snowflake (雪花算法)
//! - ObjectId (MongoDB风格)

use rat_quickdb::*;
use rat_quickdb::types::{DatabaseType, ConnectionConfig, PoolConfig, IdStrategy};
use rat_quickdb::manager::{get_global_pool_manager};
use rat_quickdb::{ModelManager, ModelOperations, string_field, integer_field, datetime_field};
use rat_logger::{LoggerBuilder, handler::term::TermConfig};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::{Utc, DateTime};

// 定义测试模型
define_model! {
    /// 测试用户模型
    struct TestUser {
        id: String,
        username: String,
        email: String,
        created_at: DateTime<Utc>,
    }
    collection = "test_users",
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

impl TestUser {
    /// 创建测试用户（ID为空以触发自动生成）
    fn new(username: &str, email: &str) -> Self {
        Self {
            id: String::new(), // 空ID，测试自动生成
            username: username.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
        }
    }

    /// 创建带有零值ID的用户（测试自增ID）
    fn new_with_zero_id(username: &str, email: &str) -> Self {
        Self {
            id: "0".to_string(), // 零值ID，测试自动生成
            username: username.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
        }
    }

    /// 创建带有无效UUID的用户
    fn new_with_invalid_uuid(username: &str, email: &str) -> Self {
        Self {
            id: "00000000-0000-0000-0000-000000000000".to_string(), // 无效UUID，测试自动生成
            username: username.to_string(),
            email: email.to_string(),
            created_at: Utc::now(),
        }
    }
}

/// 清理测试文件
async fn cleanup_test_files() {
    let test_files = vec![
        "./id_strategy_test.db",
        "./id_strategy_test.db-wal",
        "./id_strategy_test.db-shm",
    ];

    for file in test_files {
        if let Err(e) = tokio::fs::remove_file(file).await {
            if !e.to_string().contains("No such file or directory") {
                eprintln!("警告：无法删除测试文件 {}: {}", file, e);
            }
        }
    }
}

/// 测试自增ID策略
async fn test_auto_increment() -> QuickDbResult<()> {
    println!("🔢 测试 AutoIncrement ID 策略");
    println!("===============================");

    // 配置数据库，使用自增ID
    let db_config = DatabaseConfig {
        alias: "auto_increment_db".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./id_strategy_test.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::AutoIncrement,
        cache: None,
    };

    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;

    // 创建测试用户
    let users = vec![
        TestUser::new_with_zero_id("user1", "user1@test.com"),
        TestUser::new_with_zero_id("user2", "user2@test.com"),
        TestUser::new_with_zero_id("user3", "user3@test.com"),
    ];

    println!("创建3个用户，使用零值ID测试自增ID生成...");
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

    // 验证ID是否是数字且递增
    println!("\n验证ID是否正确生成:");
    for (i, id) in created_ids.iter().enumerate() {
        println!("用户 {} ID: {} (应该是数字且递增)", i + 1, id);
        if let Ok(num_id) = id.parse::<i64>() {
            println!("  ✅ ID是数字: {}", num_id);
        } else {
            println!("  ❌ ID不是数字: {}", id);
        }
    }

    // 清理数据
    let _ = rat_quickdb::delete("test_users", vec![], Some("auto_increment_db")).await;
    println!("✅ AutoIncrement ID 测试完成\n");

    Ok(())
}

/// 测试UUID ID策略
async fn test_uuid() -> QuickDbResult<()> {
    println!("🆔 测试 UUID ID 策略");
    println!("========================");

    // 配置数据库，使用UUID ID
    let db_config = DatabaseConfig {
        alias: "uuid_db".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./id_strategy_test.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::Uuid,
        cache: None,
    };

    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;

    // 创建测试用户
    let users = vec![
        TestUser::new("uuid_user1", "uuid1@test.com"),
        TestUser::new("uuid_user2", "uuid2@test.com"),
        TestUser::new_with_invalid_uuid("uuid_user3", "uuid3@test.com"),
    ];

    println!("创建3个用户，测试UUID自动生成...");
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

    // 验证ID是否是有效的UUID
    println!("\n验证ID是否为有效UUID:");
    for (i, id) in created_ids.iter().enumerate() {
        println!("用户 {} ID: {}", i + 1, id);
        if id.len() == 36 {
            println!("  ✅ ID长度正确 (36字符)");
            if id.contains('-') && id.split('-').count() == 5 {
                println!("  ✅ UUID格式正确");
            } else {
                println!("  ❌ UUID格式错误");
            }
        } else {
            println!("  ❌ ID长度错误: {}", id.len());
        }
    }

    // 清理数据
    let _ = rat_quickdb::delete("test_users", vec![], Some("uuid_db")).await;
    println!("✅ UUID ID 测试完成\n");

    Ok(())
}

/// 测试雪花算法ID策略
async fn test_snowflake() -> QuickDbResult<()> {
    println!("❄️ 测试 Snowflake ID 策略");
    println!("=============================");

    // 配置数据库，使用雪花算法ID
    let db_config = DatabaseConfig {
        alias: "snowflake_db".to_string(),
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./id_strategy_test.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        id_strategy: IdStrategy::snowflake(1, 1),
        cache: None,
    };

    let pool_manager = get_global_pool_manager();
    pool_manager.add_database(db_config).await?;

    // 创建测试用户
    let users = vec![
        TestUser::new("snowflake_user1", "snowflake1@test.com"),
        TestUser::new("snowflake_user2", "snowflake2@test.com"),
        TestUser::new("snowflake_user3", "snowflake3@test.com"),
    ];

    println!("创建3个用户，测试雪花算法ID生成...");
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

    // 验证雪花算法ID
    println!("\n验证雪花算法ID:");
    for (i, id) in created_ids.iter().enumerate() {
        println!("用户 {} ID: {}", i + 1, id);
        if id.parse::<i64>().is_ok() {
            println!("  ✅ 可以解析为数字");
            let num_id = id.parse::<i64>().unwrap();
            if num_id > 0 {
                println!("  ✅ ID为正数");
            } else {
                println!("  ❌ ID不是正数");
            }
        } else {
            println!("  ❌ 无法解析为数字");
        }
    }

    // 清理数据
    let _ = rat_quickdb::delete("test_users", vec![], Some("snowflake_db")).await;
    println!("✅ Snowflake ID 测试完成\n");

    Ok(())
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    LoggerBuilder::new()
        .add_terminal_with_config(TermConfig::default())
        .init()
        .expect("日志初始化失败");

    println!("🧪 RatQuickDB ID策略测试");
    println!("========================\n");

    // 清理之前的测试文件
    cleanup_test_files().await;

    // 测试不同的ID策略
    test_auto_increment().await?;
    test_uuid().await?;
    test_snowflake().await?;

    // 清理测试文件
    cleanup_test_files().await;

    println!("🎉 所有ID策略测试完成！");
    Ok(())
}