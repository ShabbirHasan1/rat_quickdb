//! 数组字段示例
//! 演示如何使用 array_field 和 list_field 便捷函数
//! 在 MongoDB 中使用原生数组，在 SQL 数据库中使用 JSON 存储

use rat_quickdb::{
    array_field, list_field, string_field, integer_field, float_field, boolean_field, datetime_field, dict_field,
    Model, ModelManager, ModelOperations, FieldType, DatabaseConfig, ConnectionConfig, PoolConfig, IdStrategy,
    init, add_database, DataValue, DatabaseType, QueryCondition
};
use rat_logger;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio;

rat_quickdb::define_model! {
    struct UserModel {
        id: Option<i64>,
        name: String,
        email: String,
        age: i32,
        tags: Vec<String>,
        hobbies: Vec<String>,
        scores: Vec<i32>,
        grades: Vec<f64>,
        is_active: Vec<bool>,
        #[serde(with = "rat_quickdb::types::serde_helpers::hashmap_datavalue")]
        metadata: HashMap<String, DataValue>,
    }
    
    collection = "users",
    fields = {
        id: integer_field(None, None),
        name: string_field(Some(100), Some(1), None).required(),
        email: string_field(Some(255), Some(5), None).required().unique(),
        age: integer_field(Some(0), Some(150)).required(),
        tags: array_field(
            FieldType::String {
                max_length: Some(50),
                min_length: Some(1),
                regex: None,
            },
            Some(10), // 最多10个标签
            Some(1),  // 至少1个标签
        ),
        hobbies: list_field(
            FieldType::String {
                max_length: Some(100),
                min_length: Some(1),
                regex: None,
            },
            Some(5), // 最多5个爱好
            None,    // 可以没有爱好
        ),
        scores: array_field(
            FieldType::Integer {
                min_value: Some(0),
                max_value: Some(100),
            },
            Some(10), // 最多10个分数
            None,     // 可以没有分数
        ),
        grades: array_field(
            FieldType::Float {
                min_value: Some(0.0),
                max_value: Some(4.0),
            },
            Some(10), // 最多10个成绩
            None,     // 可以没有成绩
        ),
        is_active: array_field(
            FieldType::Boolean,
            Some(5),  // 最多5个布尔值
            None,     // 可以没有
        ),
        metadata: dict_field({
            let mut fields = HashMap::new();
            fields.insert("department".to_string(), string_field(Some(100), None, None));
            fields.insert("level".to_string(), integer_field(Some(1), Some(10)));
            fields.insert("rating".to_string(), float_field(Some(0.0), Some(5.0)));
            fields.insert("verified".to_string(), boolean_field());
            fields
        }),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志为debug级别
    rat_logger::LoggerBuilder::new()
        .with_level(rat_logger::LevelFilter::Debug)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .unwrap();

    // 初始化rat_quickdb
    init();
    
    println!("=== 数组字段示例 ===");
    
    // 配置 SQLite 数据库连接
    let sqlite_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "sqlite_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    // 添加 SQLite 数据库连接
    add_database(sqlite_config).await?;
    
    // 设置 SQLite 为默认数据库
    use rat_quickdb::manager::get_global_pool_manager;
    let pool_manager = get_global_pool_manager();
    pool_manager.set_default_alias("sqlite_db").await?;
    
    // 测试 SQLite 数据库
    println!("\n--- 测试 SQLite 数据库 ---");
    test_array_fields(Some("sqlite_db")).await?;
    
    // 配置 MongoDB 数据库连接（如果可用）
    let mongo_config = DatabaseConfig {
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "localhost".to_string(),
            port: 27017,
            database: "test_array".to_string(),
            username: None,
            password: None,
            auth_source: None,
            direct_connection: false,
            tls_config: None,
            zstd_config: None,
            options: None,
        },
        pool: PoolConfig::default(),
        alias: "mongo_db".to_string(),
        cache: None,
        id_strategy: IdStrategy::ObjectId,
    };
    
    // 如果 MongoDB 可用，也测试 MongoDB
    if let Ok(_) = add_database(mongo_config).await {
        println!("\n--- 测试 MongoDB 数据库 ---");
        test_array_fields(Some("mongo_db")).await?;
    } else {
        println!("\n--- MongoDB 不可用，跳过测试 ---");
    }
    
    Ok(())
}

async fn test_array_fields(db_alias: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // 通过PoolManager获取连接池来删除可能存在的残留表，确保测试环境干净
    println!("清理测试环境，删除可能存在的残留表...");
    let pool_manager = rat_quickdb::manager::get_global_pool_manager();
    let pools = pool_manager.get_connection_pools();
    let alias = db_alias.unwrap_or("default");
    if let Some(pool) = pools.get(alias) {
        let _ = pool.drop_table("users").await; // 忽略错误，表可能不存在
    }
    println!("测试环境清理完成");
    // 创建用户
    let mut metadata = HashMap::new();
    metadata.insert("department".to_string(), DataValue::String("工程部".to_string()));
    metadata.insert("level".to_string(), DataValue::Int(5));
    metadata.insert("rating".to_string(), DataValue::Float(4.2));
    metadata.insert("verified".to_string(), DataValue::Bool(true));
    
    let user = UserModel {
        id: None,
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        age: 25,
        tags: vec!["rust".to_string(), "programming".to_string()],
        hobbies: vec!["编程".to_string(), "阅读".to_string()],
        scores: vec![95, 87, 92],
        grades: vec![3.9, 3.6, 4.0],
        is_active: vec![true, false, true, true],
        metadata,
    };
    
    // 保存用户
    let user_id = user.save().await?;
    println!("用户ID: {}", user_id);
    
    // 查询数据
    println!("查询用户数据...");
    let users = ModelManager::<UserModel>::find(vec![], None).await?;
    println!("查询到 {} 个用户", users.len());
    
    for user in users {
        println!("用户: {:?}", user);
        println!("  标签: {:?}", user.tags);
        println!("  爱好: {:?}", user.hobbies);
        println!("  分数: {:?}", user.scores);
        println!("  成绩: {:?}", user.grades);
        println!("  活跃状态: {:?}", user.is_active);
        println!("  元数据: {:?}", user.metadata);
        
        // 验证数据正确性
        assert_eq!(user.tags, vec!["rust".to_string(), "programming".to_string()]);
        assert_eq!(user.hobbies, vec!["编程".to_string(), "阅读".to_string()]);
        assert_eq!(user.scores, vec![95, 87, 92]);
        assert_eq!(user.grades, vec![3.9, 3.6, 4.0]);
        assert_eq!(user.is_active, vec![true, false, true, true]);
        
        // 验证字典内容
        assert_eq!(user.metadata.get("department"), Some(&DataValue::String("工程部".to_string())));
        assert_eq!(user.metadata.get("level"), Some(&DataValue::Int(5)));
        assert_eq!(user.metadata.get("rating"), Some(&DataValue::Float(4.2)));
        assert_eq!(user.metadata.get("verified"), Some(&DataValue::Bool(true)));
        
        println!("✅ 数组字段和字典字段验证通过！");
    }
    
    println!("数据库 {:?} 测试完成", db_alias);
    
    Ok(())
}