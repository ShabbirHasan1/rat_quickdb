//! TTL过期回退机制测试示例
//! 验证缓存TTL过期后是否正确回退到数据库查询

use rat_quickdb::*;
use rat_quickdb::manager::{get_global_pool_manager, health_check, shutdown};
use std::collections::HashMap;
use chrono::Utc;
use tokio::time::{sleep, Duration};
use rat_logger::{info, debug, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    rat_quickdb::init();
    
    info!("开始TTL过期回退机制测试");
    
    // 创建缓存配置（极短TTL用于测试）
    let cache_config = CacheConfig::default()
        .enabled(true)
        .with_strategy(CacheStrategy::Lru)
        .with_l1_config(
            L1CacheConfig::new()
                .with_max_capacity(100)
                .with_max_memory_mb(10)
                .enable_stats(true)
        )
        .with_l2_config(
            L2CacheConfig::new("./cache/ttl_test".to_string())
                .with_max_disk_mb(50)
                .with_compression_level(1)
                .enable_wal(false)
                .clear_on_startup(true)
        )
        .with_ttl_config(
            TtlConfig::new()
                .with_default_ttl_secs(2) // 极短的TTL，2秒过期
                .with_max_ttl_secs(10)
                .with_check_interval_secs(1) // 每秒检查一次过期
        )
        .with_compression_config(
            CompressionConfig::new()
                .enabled(false)
                .with_algorithm(CompressionAlgorithm::Zstd)
                .with_threshold_bytes(1024)
        );

    // 创建连接池配置
    let pool_config = PoolConfigBuilder::new()
        .min_connections(1)
        .max_connections(2)
        .connection_timeout(5)
        .idle_timeout(10)
        .max_lifetime(60)
        .build()?;

    // 创建数据库配置
    let db_config = DatabaseConfigBuilder::new()
        .db_type(DatabaseType::SQLite)
        .connection(ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .pool(pool_config)
        .alias("test_db")
        .id_strategy(IdStrategy::AutoIncrement)
        .cache(cache_config)
        .build()?;
    
    // 添加数据库配置到全局管理器
    add_database(db_config).await?;
    
    // 2. 开始数据库操作
    println!("\n2. 开始TTL过期回退机制测试...");
    let table_name = "test_ttl_table";
    
    // 插入测试数据
    let mut test_data = HashMap::new();
    test_data.insert("name".to_string(), DataValue::String("test_record".to_string()));
    test_data.insert("value".to_string(), DataValue::Int(42));
    test_data.insert("created_at".to_string(), DataValue::DateTime(Utc::now()));
    
    let created_record = rat_quickdb::create(table_name, test_data, None).await?;
    println!("插入测试数据: {}", created_record);
    
    // 第一次查询 - 应该从数据库查询并缓存
    let conditions = vec![QueryCondition {
        field: "name".to_string(),
        operator: QueryOperator::Eq,
        value: DataValue::String("test_record".to_string()),
    }];
    
    println!("\n=== 第一次查询 (应该从数据库查询并缓存) ===");
    let result1 = rat_quickdb::find(table_name, conditions.clone(), None, None).await?;
    println!("第一次查询结果: {} 条记录", result1.len());
    
    // 第二次查询 - 应该从缓存命中
    println!("\n=== 第二次查询 (应该从缓存命中) ===");
    let result2 = rat_quickdb::find(table_name, conditions.clone(), None, None).await?;
    println!("第二次查询结果: {} 条记录", result2.len());
    
    // 等待TTL过期（3秒）
    println!("\n等待TTL过期（3秒）...");
    sleep(Duration::from_secs(3)).await;
    
    // 第三次查询 - TTL过期后应该回退到数据库查询
    println!("\n=== 第三次查询 (TTL过期，应该回退到数据库) ===");
    let result3 = rat_quickdb::find(table_name, conditions.clone(), None, None).await?;
    println!("第三次查询结果: {} 条记录", result3.len());
    
    // 验证结果
    if result3.is_empty() {
        println!("❌ TTL过期后查询返回空结果，回退机制失败！");
        println!("预期: 1条记录，实际: 0条记录");
    } else {
        println!("✅ TTL过期后成功回退到数据库查询，返回 {} 条记录", result3.len());
    }
    
    // 3. 清理操作
    println!("\n3. 清理操作");
    shutdown().await?;
    println!("\n=== TTL过期回退机制测试完成 ===");
    Ok(())
}