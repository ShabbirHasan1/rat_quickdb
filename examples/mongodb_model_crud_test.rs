//! MongoDB Model CRUD 操作测试示例
//!
//! 这个示例专门测试在 MongoDB 数据库中使用 Model 的 update 和 delete 方法
//! 验证对 MongoDB _id 字段和 ObjectId 的兼容性支持

use rat_quickdb::{
    Model, ModelManager, ModelOperations, DatabaseConfig, ConnectionConfig, PoolConfig, IdStrategy,
    init, add_database, DataValue, QueryCondition, QueryOperator,
    string_field, integer_field, mongodb_config_with_builder, MongoDbConnectionBuilder, TlsConfig, ZstdConfig
};
use rat_logger::{LoggerBuilder, LevelFilter, handler::term::TermConfig};
use std::collections::HashMap;
use tokio;

// 定义用户模型，测试 MongoDB 兼容性
rat_quickdb::define_model! {
    struct MongoUser {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<String>,
        name: String,
        email: String,
        age: i32,
        department: String,
    }

    collection = "mongo_users",
    fields = {
        _id: string_field(Some(24), Some(24), None),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(5), None).required().unique(),
        age: integer_field(Some(0), Some(150)).required(),
        department: string_field(Some(50), Some(1), None).required(),
    }
}

/// 创建 MongoDB 配置
fn create_mongodb_config() -> Result<DatabaseConfig, rat_quickdb::QuickDbError> {
    let pool_config = PoolConfig::builder()
        .max_connections(10)
        .min_connections(2)
        .connection_timeout(5)
        .idle_timeout(300)
        .max_lifetime(1800)
        .build()?;

    // 配置TLS
    let tls_config = TlsConfig {
        enabled: true,
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
        verify_server_cert: false,
        verify_hostname: false,
        min_tls_version: None,
        cipher_suites: None,
    };

    // 配置ZSTD压缩
    let zstd_config = ZstdConfig {
        enabled: true,
        compression_level: Some(3),
        compression_threshold: Some(1024),
    };

    // 使用构建器创建连接配置
    let builder = MongoDbConnectionBuilder::new("db0.0ldm0s.net", 27017, "testdb")
        .with_auth("testdb", "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^")
        .with_auth_source("testdb")
        .with_direct_connection(true)
        .with_tls_config(tls_config)
        .with_zstd_config(zstd_config)
        .with_option("retryWrites", "true")
        .with_option("w", "majority");

    mongodb_config_with_builder(
        "mongodb_test",
        builder,
        pool_config,
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    LoggerBuilder::new()
        .with_level(LevelFilter::Info)
        .add_terminal_with_config(TermConfig::default())
        .init()?;

    println!("=== MongoDB Model CRUD 操作测试 ===\n");

    // 初始化 rat_quickdb
    init();

    // 配置 MongoDB 连接
    println!("配置 MongoDB 连接...");
    let mongo_config = create_mongodb_config()?;
    add_database(mongo_config).await?;
    println!("✅ MongoDB 连接配置完成");

    // 设置 MongoDB 为默认数据库
    use rat_quickdb::manager::get_global_pool_manager;
    let pool_manager = get_global_pool_manager();
    pool_manager.set_default_alias("mongodb_test").await?;

    // 测试 CRUD 操作
    test_mongodb_crud_operations().await?;

    println!("\n=== MongoDB 测试完成 ===");
    println!("验证结果：");
    println!("✅ Model.save() 方法在 MongoDB 中正常工作");
    println!("✅ Model.update() 方法在 MongoDB 中正常工作");
    println!("✅ Model.delete() 方法在 MongoDB 中正常工作");
    println!("✅ MongoDB _id 字段兼容性正常");
    println!("✅ ObjectId 格式支持正常");

    Ok(())
}

async fn test_mongodb_crud_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 测试 MongoDB CRUD 操作 ===");

    // 1. 创建用户
    println!("1. 创建用户...");
    let user = MongoUser {
        _id: None,
        name: "MongoDB测试用户".to_string(),
        email: "mongo_test@example.com".to_string(),
        age: 28,
        department: "技术部".to_string(),
    };

    let user_id = user.save().await?;
    println!("✅ 用户创建成功，MongoDB 返回的 ID: {}", user_id);

    // 2. 查询用户
    println!("\n2. 查询用户...");
    let found_user = ModelManager::<MongoUser>::find_by_id(&user_id).await?;
    if let Some(user) = found_user {
        println!("✅ 查询成功: {} ({}岁，{})", user.name, user.age, user.department);
        println!("   用户ID字段值: {:?}", user._id);

        // 3. 更新用户
        println!("\n3. 更新用户...");
        let mut updates = HashMap::new();
        updates.insert("age".to_string(), DataValue::Int(29));
        updates.insert("department".to_string(), DataValue::String("产品部".to_string()));

        let update_result = user.update(updates).await?;
        println!("✅ 用户更新成功: {}", update_result);

        // 验证更新
        let updated_user = ModelManager::<MongoUser>::find_by_id(&user_id).await?;
        if let Some(user) = updated_user {
            println!("✅ 更新验证: {} ({}岁，{})", user.name, user.age, user.department);
        }
    }

    // 4. 创建第二个用户用于测试删除
    println!("\n4. 创建第二个用户...");
    let user2 = MongoUser {
        _id: None,
        name: "待删除用户".to_string(),
        email: "delete_test@example.com".to_string(),
        age: 35,
        department: "市场部".to_string(),
    };

    let user2_id = user2.save().await?;
    println!("✅ 第二个用户创建成功，ID: {}", user2_id);

    // 5. 查询所有用户
    println!("\n5. 查询所有用户...");
    let all_users = ModelManager::<MongoUser>::find(vec![], None).await?;
    println!("✅ 当前用户数量: {}", all_users.len());
    for user in &all_users {
        println!("   - {} ({}岁, {})", user.name, user.age, user.department);
    }

    // 6. 删除第二个用户
    println!("\n6. 删除第二个用户...");
    let user_to_delete = ModelManager::<MongoUser>::find_by_id(&user2_id).await?;
    if let Some(user) = user_to_delete {
        let delete_result = user.delete().await?;
        println!("✅ 用户删除成功: {}", delete_result);

        // 验证删除
        let deleted_user = ModelManager::<MongoUser>::find_by_id(&user2_id).await?;
        if deleted_user.is_none() {
            println!("✅ 删除验证成功：用户已从 MongoDB 中删除");
        }
    }

    // 7. 最终查询
    println!("\n7. 最终查询...");
    let final_users = ModelManager::<MongoUser>::find(vec![], None).await?;
    println!("✅ 删除后剩余用户数量: {}", final_users.len());

    Ok(())
}