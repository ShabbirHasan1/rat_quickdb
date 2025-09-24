//! 调用者初始化日志示例
//!
//! 这个示例展示了如何作为调用者来正确初始化日志系统
//! 然后使用rat_quickdb进行数据库操作

use rat_quickdb::{
    Model, ModelManager, ModelOperations, FieldType, DatabaseConfig, ConnectionConfig, PoolConfig, IdStrategy,
    init, add_database, DataValue, DatabaseType, QueryCondition,
    string_field, integer_field
};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig, FormatConfig, LevelStyle};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio;

// 使用宏定义用户模型
rat_quickdb::define_model! {
    struct User {
        id: Option<i32>,
        name: String,
        email: String,
        age: i32,
    }

    collection = "users",
    fields = {
        id: integer_field(None, None),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(200), Some(1), None).required().unique(),
        age: integer_field(Some(0), Some(150)).required(),
    }
}

/// 自定义日志格式配置
fn create_custom_log_format() -> FormatConfig {
    FormatConfig {
        timestamp_format: "%Y-%m-%d %H:%M:%S%.3f".to_string(),
        level_style: LevelStyle {
            error: "ERROR".to_string(),
            warn: "WARN ".to_string(),
            info: "INFO ".to_string(),
            debug: "DEBUG".to_string(),
            trace: "TRACE".to_string(),
        },
        format_template: "[{timestamp}] {level} {target}:{line} - {message}".to_string(),
    }
}

/// 初始化日志系统的推荐方式
fn init_logging_system() -> Result<(), Box<dyn std::error::Error>> {
    // 方式1: 使用默认配置（简单）
    // LoggerBuilder::new()
    //     .add_terminal_with_config(TermConfig::default())
    //     .init()?;

    // 方式2: 自定义配置（推荐）
    let format_config = create_custom_log_format();
    let term_config = TermConfig {
        enable_color: true,
        format: Some(format_config),
        color: None, // 使用默认颜色
    };

    LoggerBuilder::new()
        .with_level(LevelFilter::Info)  // 设置日志级别
        .add_terminal_with_config(term_config)
        .init()?;

    println!("✅ 日志系统初始化成功");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // === 调用者负责初始化日志系统 ===
    println!("=== 调用者初始化日志示例 ===\n");

    // 1. 首先初始化日志系统（调用者责任）
    init_logging_system()?;

    // 2. 然后初始化rat_quickdb库（不自动初始化日志）
    init();

    println!("✅ 所有系统初始化完成\n");

    // 3. 配置数据库连接
    println!("配置数据库连接...");

    // SQLite 配置
    let sqlite_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "main_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };

    // 添加数据库
    add_database(sqlite_config).await?;
    println!("✅ 数据库连接配置完成");

    // 4. 创建模型管理器
    let user_manager = ModelManager::<User>::new();
    println!("✅ 模型管理器创建完成");

    // 5. 数据库操作示例
    println!("\n=== 数据库操作示例 ===");

    // 创建用户
    let mut user = User {
        id: None,
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        age: 25,
    };

    println!("创建用户: {}", user.name);
    let user_id = user.save().await?;
    println!("✅ 用户创建成功，ID: {}", user_id);

    // 查询用户
    println!("查询用户数据...");
    let found_user = ModelManager::<User>::find_by_id(&user_id).await?;
    if let Some(user) = found_user {
        println!("✅ 查询成功: {} ({}岁)", user.name, user.age);

        // 更新用户年龄
        println!("更新用户年龄...");
        let mut updates = HashMap::new();
        updates.insert("age".to_string(), DataValue::Int(26));
        let update_result = user.update(updates).await?;
        println!("✅ 用户更新成功: {}", update_result);
    }

    // 查询所有用户
    println!("查询所有用户...");
    let all_users = ModelManager::<User>::find(vec![], None).await?;
    println!("✅ 找到 {} 个用户", all_users.len());

    // 创建第二个用户
    println!("创建第二个用户...");
    let user2 = User {
        id: None,
        name: "李四".to_string(),
        email: "lisi@example.com".to_string(),
        age: 30,
    };
    let user2_id = user2.save().await?;
    println!("✅ 第二个用户创建成功，ID: {}", user2_id);

    // 再次查询所有用户
    println!("再次查询所有用户...");
    let all_users = ModelManager::<User>::find(vec![], None).await?;
    println!("✅ 现在找到 {} 个用户", all_users.len());

    // 删除第一个用户
    println!("删除第一个用户...");
    if let Some(user) = ModelManager::<User>::find_by_id(&user_id).await? {
        let delete_result = user.delete().await?;
        println!("✅ 用户删除成功: {}", delete_result);
    }

    // 验证删除
    let deleted_user = ModelManager::<User>::find_by_id(&user_id).await?;
    if deleted_user.is_none() {
        println!("✅ 用户删除验证成功");
    }

    // 查询剩余用户
    let remaining_users = ModelManager::<User>::find(vec![], None).await?;
    println!("✅ 删除后剩余 {} 个用户", remaining_users.len());

    println!("\n=== 示例完成 ===");
    println!("这个示例展示了：");
    println!("1. 调用者如何自定义和初始化日志系统");
    println!("2. rat_quickdb库不再自动初始化日志");
    println!("3. 日志系统完全由调用者控制");

    Ok(())
}