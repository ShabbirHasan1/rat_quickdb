//! 手动缓存清理示例
//! 
//! 本示例展示如何使用各种手动缓存清理方法，适用于以下场景：
//! 1. 自动清理不生效时的手动干预
//! 2. 数据字段升级后需要清理旧缓存
//! 3. 内存紧张时的主动清理
//! 4. 调试和监控缓存状态

use rat_quickdb::{
    config::{DatabaseConfigBuilder, GlobalConfig, AppConfigBuilder, LoggingConfigBuilder},
    manager::PoolManager,
    types::*,
    error::QuickDbResult,
};
use rat_quickdb::{Environment, LogLevel};
// 使用 rat_quickdb 的缓存配置类型
// 不再需要直接使用 rat_memcache，因为缓存功能已集成到 rat_quickdb 中
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zerg_creep::{info, warn, error};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Option<IdType>,
    name: String,
    email: String,
    age: i32,
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    zerg_creep::init_logger();
    
    // 初始化数据库
    let db_config = rat_quickdb::config::DatabaseConfigBuilder::new()
        .db_type(rat_quickdb::types::DatabaseType::SQLite)
        .connection(rat_quickdb::types::ConnectionConfig::SQLite {
            path: ":memory:".to_string(),
            create_if_missing: true,
        })
        .pool(rat_quickdb::types::PoolConfig::builder()
            .min_connections(1)
            .max_connections(5)
            .connection_timeout(30)
            .idle_timeout(300)
            .max_lifetime(3600)
            .build()
            .unwrap()
        )
        .alias("main")
        .id_strategy(rat_quickdb::types::IdStrategy::AutoIncrement)
        .cache(rat_quickdb::types::CacheConfig::default()
            .enabled(true)
            .with_strategy(rat_quickdb::types::CacheStrategy::Lru)
            .with_l1_config(rat_quickdb::types::L1CacheConfig::new()
                .with_max_capacity(1000)
                .with_max_memory_mb(50)
                .enable_stats(true))
            .with_ttl_config(rat_quickdb::types::TtlConfig::new()
                .with_default_ttl_secs(300)
                .with_max_ttl_secs(3600)
                .with_check_interval_secs(60)))
        .build()?;

    let config = GlobalConfig::builder()
        .add_database(db_config)
        .default_database("main")
        .app(
            AppConfigBuilder::new()
                .name("缓存清理示例")
                .version("1.0.0")
                .environment(Environment::Development)
                .debug(true)
                .work_dir(std::env::current_dir()?)
                .build()?
        )
        .logging(
            LoggingConfigBuilder::new()
                .level(LogLevel::Info)
                .console(true)
                .file_path(None::<std::path::PathBuf>)
                .max_file_size(10 * 1024 * 1024)
                .max_files(5)
                .structured(false)
                .build()?
        )
        .build()?;
    
    // 初始化rat_quickdb
    rat_quickdb::init();
    
    // 获取全局连接池管理器并添加数据库配置
    let pool_manager = rat_quickdb::manager::get_global_pool_manager();
    let db_config = config.get_default_database()?.clone();
    pool_manager.add_database(db_config).await?;
    
    // 创建测试数据
    let mut user_ids: Vec<String> = Vec::new();
    for i in 1..=5 {
        let data = HashMap::from([
            ("name".to_string(), DataValue::String(format!("用户{}", i))),
            ("age".to_string(), DataValue::Int(20 + i as i64)),
            ("email".to_string(), DataValue::String(format!("user{}@example.com", i))),
            ("created_at".to_string(), DataValue::DateTime(Utc::now())),
        ]);
        
        let id = rat_quickdb::odm::create("users", data, Some("main")).await?;
        info!("创建用户: ID={}", id);
        user_ids.push(id);
    }
    
    // 填充缓存 - 执行一些查询和记录操作
    let query_options = rat_quickdb::types::QueryOptions::new().with_pagination(rat_quickdb::types::PaginationConfig { skip: 0, limit: 10 });
    
    // 查询用户数据（会缓存查询结果）
    let users = rat_quickdb::odm::find("users", vec![], Some(query_options.clone()), Some("main")).await?;
    info!("查询到用户数据: {} 条", users.len());
    
    // 获取单个用户记录（会缓存记录）
    for user_id in &user_ids {
        let user_doc = rat_quickdb::odm::find_by_id("users", user_id, Some("main")).await?;
        if let Some(_) = user_doc {
            info!("缓存了用户记录: {}", user_id);
        }
    }
    

    
    // 创建一些产品数据用于演示
    for i in 1..=3 {
        let product_data = HashMap::from([
            ("name".to_string(), DataValue::String(format!("产品{}", i))),
            ("price".to_string(), DataValue::Float(99.99 + i as f64)),
            ("category".to_string(), DataValue::String("电子产品".to_string())),
        ]);
        
        let _product_id = rat_quickdb::odm::create("products", product_data, Some("main")).await?;
    }
    
    // 查询产品数据
    let products = rat_quickdb::odm::find("products", vec![], Some(query_options), Some("main")).await?;
    info!("查询到产品数据: {} 条", products.len());
    
    // 按条件查询（会缓存查询结果）
    let conditions = vec![
         rat_quickdb::types::QueryCondition {
             field: "age".to_string(),
             operator: rat_quickdb::types::QueryOperator::Gt,
             value: rat_quickdb::types::DataValue::Int(25)
         }
     ];
    let query_options = rat_quickdb::types::QueryOptions::new().with_pagination(rat_quickdb::types::PaginationConfig { skip: 0, limit: 10 });
    let _filtered_users = rat_quickdb::odm::find("users", conditions, Some(query_options), Some("main")).await?;
    info!("按条件查询完成");
    
    // 展示缓存清理功能
    info!("\n=== 缓存清理功能演示 ===");
    
    // 1. 查看当前缓存键
    info!("\n1. 查看当前缓存键：");
    let cache_keys = rat_quickdb::manager::list_cache_keys("main").await?;
    for (table, keys) in &cache_keys {
        info!("表 '{}' 的缓存键数量: {}", table, keys.len());
        for key in keys {
            info!("  - {}", key);
        }
    }
    
    // 2. 查看指定表的缓存键
    info!("\n2. 查看 users 表的缓存键：");
    let user_cache_keys = rat_quickdb::manager::list_table_cache_keys("main", "users").await?;
    info!("users 表缓存键数量: {}", user_cache_keys.len());
    for key in &user_cache_keys {
        info!("  - {}", key);
    }
    
    // 3. 按模式清理缓存
    info!("\n3. 按模式清理缓存（清理所有查询缓存）：");
    let cleared_count = rat_quickdb::manager::clear_cache_by_pattern("main", "*:query:*").await?;
    info!("按模式清理了 {} 个缓存项", cleared_count);
    
    // 验证查询缓存已清理
    let remaining_keys = rat_quickdb::manager::list_table_cache_keys("main", "users").await?;
    info!("清理后剩余缓存键数量: {}", remaining_keys.len());
    
    // 4. 批量清理记录缓存
    info!("\n4. 批量清理指定记录的缓存：");
    let ids_to_clear: Vec<rat_quickdb::types::IdType> = user_ids[0..2].iter().map(|s| rat_quickdb::types::IdType::String(s.clone())).collect(); // 清理前两个用户的缓存
    let cleared_count = rat_quickdb::manager::clear_records_cache_batch("main", "users", &ids_to_clear).await?;
    info!("批量清理了 {} 个记录缓存", cleared_count);
    
    // 5. 清理指定表的查询缓存
    info!("\n5. 清理指定表的查询缓存：");
    // 重新查询以验证缓存已清理
     let query_options = rat_quickdb::types::QueryOptions::new();
     let users = rat_quickdb::odm::find("users", vec![], Some(query_options), Some("main")).await?;
     info!("重新查询到用户数据: {}", users.len());
    let cleared_count = rat_quickdb::manager::clear_cache_by_pattern("main", "users:query:*").await?;
    info!("清理了 {} 个查询缓存", cleared_count);
    
    // 6. 清理指定表的记录缓存
    info!("\n6. 清理指定表的记录缓存：");
    let cleared_count = rat_quickdb::manager::clear_cache_by_pattern("main", "users:record:*").await?;
    info!("清理了 {} 个记录缓存", cleared_count);
    
    // 7. 清理指定表的所有缓存
    info!("\n7. 清理指定表的所有缓存：");
    // 重新填充缓存
    let conditions = vec![];
    let query_options = rat_quickdb::types::QueryOptions::new().with_pagination(rat_quickdb::types::PaginationConfig { skip: 0, limit: 10 });
    let users = rat_quickdb::odm::find("users", conditions, Some(query_options), Some("main")).await?;
    
    for user_id in &user_ids {
        if let Some(user) = rat_quickdb::odm::find_by_id("users", user_id, Some("main")).await? {
            info!("重新查询到用户: ID={}", user_id);
        }
    }
    let query_cleared = rat_quickdb::manager::clear_table_query_cache("main", "users").await?;
     let record_cleared = rat_quickdb::manager::clear_table_record_cache("main", "users").await?;
     let cleared_count = query_cleared + record_cleared;
    info!("清理了 {} 个缓存项（记录+查询）", cleared_count);
    
    // 8. 强制清理过期缓存
    info!("\n8. 强制清理过期缓存：");
    let cleared_count = rat_quickdb::manager::force_cleanup_expired_cache("main").await?;
    info!("强制清理了 {} 个过期缓存项", cleared_count);
    
    // 9. 清理所有缓存
    info!("\n9. 清理所有缓存：");
    rat_quickdb::manager::clear_all_caches().await?;
    info!("已清理所有数据库的缓存");
    
    // 验证缓存已完全清理
    let final_cache_keys = rat_quickdb::manager::list_cache_keys("main").await?;
    info!("\n最终缓存状态：");
    if final_cache_keys.is_empty() {
        info!("✅ 所有缓存已清理完毕");
    } else {
        for (table, keys) in &final_cache_keys {
            info!("表 '{}' 还有 {} 个缓存键", table, keys.len());
        }
    }
    
    info!("\n=== 使用场景示例 ===");
    
    // 场景1：数据字段升级后清理旧缓存
    info!("\n场景1：数据字段升级后清理旧缓存");
    info!("// 假设 User 结构体新增了字段，需要清理旧的缓存数据");
    info!("clear_table_all_cache(\"main\", \"users\").await?;");
    
    // 场景2：内存紧张时的主动清理
    info!("\n场景2：内存紧张时的主动清理");
    info!("// 清理过期缓存释放内存");
    info!("force_cleanup_expired_cache(\"main\").await?;");
    info!("// 或者清理特定模式的缓存");
    info!("clear_cache_by_pattern(\"main\", \"*:query:*\").await?; // 只清理查询缓存");
    
    // 场景3：调试缓存问题
    info!("\n场景3：调试缓存问题");
    info!("// 查看当前缓存状态");
    info!("let cache_keys = list_cache_keys(\"main\").await?;");
    info!("// 查看特定表的缓存");
    info!("let table_keys = list_table_cache_keys(\"main\", \"users\").await?;");
    
    // 场景4：定期维护
    info!("\n场景4：定期维护任务");
    info!("// 可以在定时任务中执行");
    info!("tokio::spawn(async {{");
    info!("    loop {{");
    info!("        tokio::time::sleep(Duration::from_secs(3600)).await; // 每小时");
    info!("        if let Err(e) = force_cleanup_expired_cache(\"main\").await {{");
    info!("            error!(\"清理过期缓存失败: {{}}\", e);");
    info!("        }}");
    info!("    }}");
    info!("}});");
    
    info!("\n✅ 手动缓存清理示例完成！");
    
    Ok(())
}