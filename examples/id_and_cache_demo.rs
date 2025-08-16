//! ID生成器和缓存功能演示
//! 
//! 本示例演示如何使用rat_quickdb的ID生成器和缓存功能：
//! 1. 配置不同类型的ID生成策略（UUID、雪花算法、MongoDB自增）
//! 2. 配置和使用缓存管理器
//! 3. 演示缓存的自动更新和清理功能

use rat_quickdb::{
    manager::{add_database, get_connection, get_id_generator, get_mongo_auto_increment_generator},
    types::{
        DatabaseConfig, ConnectionConfig, IdStrategy, IdType, DataValue, QueryOptions, PaginationConfig,
        QueryCondition, QueryOperator,
    },
    QuickDbResult,
};

#[cfg(feature = "cache")]
use rat_quickdb::types::{
    CacheConfig, CacheStrategy, L1CacheConfig, L2CacheConfig, TtlConfig,
    CompressionConfig, CompressionAlgorithm
};

#[cfg(feature = "cache")]
use rat_quickdb::manager::{get_cache_manager, get_cache_stats, clear_cache};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: String,
    name: String,
    email: String,
    age: u32,
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    env_logger::init();
    
    println!("=== ID生成器和缓存功能演示 ===");
    
    // 演示UUID ID策略
    demonstrate_uuid_id_strategy().await?;
    
    // 演示雪花算法ID策略
    demonstrate_snowflake_id_strategy().await?;
    
    // 演示MongoDB自增ID
    demonstrate_mongodb_auto_increment().await?;
    
    // 演示缓存功能
    #[cfg(feature = "cache")]
    demonstrate_cache_functionality().await?;
    
    println!("\n=== 演示完成 ===");
    Ok(())
}

/// 演示UUID ID策略
async fn demonstrate_uuid_id_strategy() -> QuickDbResult<()> {
    println!("\n--- UUID ID策略演示 ---");
    
    // 配置使用UUID的数据库
    let config = DatabaseConfig::builder()
        .connection(ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .id_strategy(IdStrategy::Uuid)
        .build()?;
    
    add_database(config).await?;
    
    // 获取ID生成器并生成UUID
    let id_generator = get_id_generator("uuid_db")?;
    
    for i in 1..=5 {
        let id = id_generator.generate().await?;
        println!("生成的UUID {}: {}", i, id);
        
        // 验证ID格式
        if id_generator.validate_id(&id) {
            println!("  ✓ ID格式验证通过");
        } else {
            println!("  ✗ ID格式验证失败");
        }
    }
    
    Ok(())
}

/// 演示雪花算法ID策略
#[cfg(feature = "snowflake-id")]
async fn demonstrate_snowflake_id_strategy() -> QuickDbResult<()> {
    println!("\n--- 雪花算法ID策略演示 ---");
    
    // 配置使用雪花算法的数据库
    let config = DatabaseConfig::builder()
        .connection(ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .id_strategy(IdStrategy::Snowflake { machine_id: 1, datacenter_id: 1 }) // 机器ID=1, 数据中心ID=1
        .build()?;
    
    add_database(config).await?;
    
    // 获取ID生成器并生成雪花ID
    let id_generator = get_id_generator("snowflake_db")?;
    
    for i in 1..=5 {
        let id = id_generator.generate().await?;
        println!("生成的雪花ID {}: {}", i, id);
        
        // 验证ID格式
        if id_generator.validate_id(&id) {
            println!("  ✓ ID格式验证通过");
        } else {
            println!("  ✗ ID格式验证失败");
        }
    }
    
    Ok(())
}

#[cfg(not(feature = "snowflake-id"))]
async fn demonstrate_snowflake_id_strategy() -> QuickDbResult<()> {
    println!("\n--- 雪花算法ID策略演示 ---");
    println!("注意: snowflake-id 特性未启用，跳过雪花算法演示");
    Ok(())
}

/// 演示MongoDB自增ID
#[cfg(feature = "mongodb")]
async fn demonstrate_mongodb_auto_increment() -> QuickDbResult<()> {
    println!("\n--- MongoDB自增ID演示 ---");
    
    // 配置MongoDB数据库
    let config = DatabaseConfig::builder()
        .connection(ConnectionConfig::MongoDB {
            connection_string: "mongodb://localhost:27017".to_string(),
            database_name: "test_db".to_string(),
            max_pool_size: Some(10),
            min_pool_size: Some(1),
            max_idle_time: None,
            tls_config: None,
            zstd_config: None,
        })
        .id_strategy(IdStrategy::AutoIncrement)
        .build()?;
    
    add_database(config).await?;
    
    // 获取MongoDB自增ID生成器
    let mongo_generator = get_mongo_auto_increment_generator("mongo_db")?;
    
    for i in 1..=5 {
        let id = mongo_generator.next_id("users").await?;
        println!("生成的MongoDB自增ID {}: {}", i, id);
    }
    
    // 重置计数器演示
    mongo_generator.reset_counter("users", 100).await?;
    let reset_id = mongo_generator.next_id("users").await?;
    println!("重置计数器后的ID: {}", reset_id);
    
    Ok(())
}

#[cfg(not(feature = "mongodb"))]
async fn demonstrate_mongodb_auto_increment() -> QuickDbResult<()> {
    println!("\n--- MongoDB自增ID演示 ---");
    println!("注意: mongodb 特性未启用，跳过MongoDB自增ID演示");
    Ok(())
}

/// 演示缓存功能
#[cfg(feature = "cache")]
async fn demonstrate_cache_functionality() -> QuickDbResult<()> {
    println!("\n--- 缓存功能演示 ---");
    
    // 配置带缓存的数据库
    let cache_config = CacheConfig {
        enabled: true,
        strategy: CacheStrategy::Lru,
        l1_config: L1CacheConfig {
            max_capacity: 1000,
            max_memory_mb: 100,
            enable_stats: true,
        },
        l2_config: None,
        ttl_config: TtlConfig {
            default_ttl_secs: 3600,
            max_ttl_secs: 86400,
            check_interval_secs: 300,
        },
        compression_config: CompressionConfig {
            enabled: true,
            algorithm: CompressionAlgorithm::Lz4,
            threshold_bytes: 1024,
        },
    };
    
    let config = DatabaseConfig::builder()
        .connection(ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .id_strategy(IdStrategy::Uuid)
        .cache(cache_config)
        .build()?;
    
    add_database(config).await?;
    
    // 获取缓存管理器
    let cache_manager = get_cache_manager("cache_db")?;
    
    // 演示缓存操作
    let user = User {
        id: "user_001".to_string(),
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        age: 25,
    };
    
    // 存储用户到缓存
    let user_data = DataValue::Object({
        let mut map = std::collections::HashMap::new();
        map.insert("id".to_string(), DataValue::String(user.id.clone()));
        map.insert("name".to_string(), DataValue::String(user.name.clone()));
        map.insert("email".to_string(), DataValue::String(user.email.clone()));
        map.insert("age".to_string(), DataValue::Int(user.age as i64));
        map
    });
    let user_id = IdType::String(user.id.clone());
    cache_manager.cache_record("users", &user_id, &user_data).await?;
    println!("用户已存储到缓存: {}", user.name);
    
    // 从缓存获取用户
    if let Some(cached_user_data) = cache_manager.get_cached_record("users", &user_id).await? {
        if let DataValue::Object(ref map) = cached_user_data {
            if let Some(DataValue::String(name)) = map.get("name") {
                println!("从缓存获取用户: {}", name);
            }
        }
    }
    
    // 演示查询结果缓存
    let query_options = QueryOptions {
        conditions: vec![QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gt,
            value: DataValue::Int(20),
        }],
        sort: vec![],
        pagination: None,
        fields: vec![],
    };
    let query_result = vec![user_data.clone()];
    
    cache_manager.cache_query_result("users", &query_options, &query_result).await?;
    println!("查询结果已缓存");
    
    if let Some(cached_result) = cache_manager.get_cached_query_result("users", &query_options).await? {
        println!("从缓存获取查询结果，用户数量: {}", cached_result.len());
    }
    
    // 获取缓存统计信息
    let stats = get_cache_stats("cache_db").await?;
    println!("缓存统计信息:");
    println!("  命中次数: {}", stats.hits);
    println!("  未命中次数: {}", stats.misses);
    println!("  命中率: {:.2}%", (stats.hits as f64 / (stats.hits + stats.misses) as f64) * 100.0);
    println!("  缓存大小: {}", stats.entries);
    
    // 演示缓存失效
    cache_manager.invalidate_record("users", &user_id).await?;
    println!("用户缓存已失效");
    
    // 验证缓存已失效
    if cache_manager.get_cached_record("users", &user_id).await?.is_none() {
        println!("✓ 确认用户缓存已被清除");
    }
    
    // 清理所有缓存
    clear_cache("cache_db").await?;
    println!("所有缓存已清理");
    
    Ok(())
}

#[cfg(not(feature = "cache"))]
async fn demonstrate_cache_functionality() -> QuickDbResult<()> {
    println!("\n--- 缓存功能演示 ---");
    println!("注意: cache 特性未启用，跳过缓存功能演示");
    Ok(())
}