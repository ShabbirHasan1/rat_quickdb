//! Rust主库缓存配置隔离性测试
//!
//! 验证使用同一个DbQueueManager添加多个数据库时，
//! 缓存配置是否会相互影响

use std::collections::HashMap;
use rat_quickdb::{
    manager::PoolManager,
    types::*,
    pool::DatabaseOperation,
};
use rat_logger::{info, warn};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    rat_logger::LoggerBuilder::new().add_terminal_with_config(rat_logger::handler::term::TermConfig::default()).init().expect("日志初始化失败").expect("日志初始化失败");
    
    info!("=== Rust主库缓存配置隔离性测试 ===");
    
    // 创建单一的PoolManager实例（模拟共享Bridge场景）
    let mut manager = PoolManager::new();
    
    // 测试场景1：先添加缓存数据库，再添加非缓存数据库
    info!("\n--- 测试场景1：先缓存后非缓存 ---");
    test_cache_then_non_cache(&mut manager).await?;
    
    // 重新创建manager以避免状态污染
    let mut manager = PoolManager::new();
    
    // 测试场景2：先添加非缓存数据库，再添加缓存数据库
    info!("\n--- 测试场景2：先非缓存后缓存 ---");
    test_non_cache_then_cache(&mut manager).await?;
    
    // 重新创建manager以避免状态污染
    let mut manager = PoolManager::new();
    
    // 测试场景3：同时添加多个缓存和非缓存数据库
    info!("\n--- 测试场景3：混合模式 ---");
    test_mixed_mode(&mut manager).await?;
    
    info!("\n=== 测试完成 ===");
    Ok(())
}

/// 测试场景1：先添加缓存数据库，再添加非缓存数据库
async fn test_cache_then_non_cache(manager: &mut PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    // 创建缓存配置
    let cache_config = CacheConfig::default()
        .enabled(true)
        .with_strategy(CacheStrategy::Lru)
        .with_l1_config(
            L1CacheConfig::new()
                .with_max_capacity(100)
                .with_max_memory_mb(10)
                .enable_stats(true)
        )
        .with_ttl_config(
            TtlConfig::new()
                .with_default_ttl_secs(300)
                .with_max_ttl_secs(3600)
                .with_check_interval_secs(60)
        );
    
    // 添加缓存数据库
    let cached_db_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./test_data/cache_isolation_cached_first.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "test_cached_first".to_string(),
        cache: Some(cache_config),
        id_strategy: IdStrategy::Uuid,
    };
    
    manager.add_database(cached_db_config).await?;
    info!("已添加缓存数据库: test_cached_first");
    
    // 添加非缓存数据库
    let non_cached_db_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./test_data/cache_isolation_non_cached_second.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "test_non_cached_second".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };
    
    manager.add_database(non_cached_db_config).await?;
    info!("已添加非缓存数据库: test_non_cached_second");
    
    // 触发适配器创建并观察日志
    trigger_adapter_creation(manager, "test_cached_first", "test_non_cached_second").await?;
    
    Ok(())
}

/// 测试场景2：先添加非缓存数据库，再添加缓存数据库
async fn test_non_cache_then_cache(manager: &mut PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    // 添加非缓存数据库
    let non_cached_db_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./test_data/cache_isolation_non_cached_first.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "test_non_cached_first".to_string(),
        cache: None,
        id_strategy: IdStrategy::Uuid,
    };
    
    manager.add_database(non_cached_db_config).await?;
    info!("已添加非缓存数据库: test_non_cached_first");
    
    // 创建缓存配置
    let cache_config = CacheConfig::default()
        .enabled(true)
        .with_strategy(CacheStrategy::Lru)
        .with_l1_config(
            L1CacheConfig::new()
                .with_max_capacity(100)
                .with_max_memory_mb(10)
                .enable_stats(true)
        )
        .with_ttl_config(
            TtlConfig::new()
                .with_default_ttl_secs(300)
                .with_max_ttl_secs(3600)
                .with_check_interval_secs(60)
        );
    
    // 添加缓存数据库
    let cached_db_config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
            path: "./test_data/cache_isolation_cached_second.db".to_string(),
            create_if_missing: true,
        },
        pool: PoolConfig::default(),
        alias: "test_cached_second".to_string(),
        cache: Some(cache_config),
        id_strategy: IdStrategy::Uuid,
    };
    
    manager.add_database(cached_db_config).await?;
    info!("已添加缓存数据库: test_cached_second");
    
    // 触发适配器创建并观察日志
    trigger_adapter_creation(manager, "test_non_cached_first", "test_cached_second").await?;
    
    Ok(())
}

/// 测试场景3：混合模式
async fn test_mixed_mode(manager: &mut PoolManager) -> Result<(), Box<dyn std::error::Error>> {
    // 创建缓存配置
    let cache_config = CacheConfig::default()
        .enabled(true)
        .with_strategy(CacheStrategy::Lru)
        .with_l1_config(
            L1CacheConfig::new()
                .with_max_capacity(100)
                .with_max_memory_mb(10)
                .enable_stats(true)
        )
        .with_ttl_config(
            TtlConfig::new()
                .with_default_ttl_secs(300)
                .with_max_ttl_secs(3600)
                .with_check_interval_secs(60)
        );
    
    // 同时添加多个数据库
    let databases = vec![
        ("test_mixed_cache_1", Some(cache_config.clone())),
        ("test_mixed_non_cache_1", None),
        ("test_mixed_cache_2", Some(cache_config.clone())),
        ("test_mixed_non_cache_2", None),
    ];
    
    for (alias, cache) in databases {
        let db_config = DatabaseConfig {
            db_type: DatabaseType::SQLite,
            connection: ConnectionConfig::SQLite {
                path: format!("./test_data/cache_isolation_{}.db", alias),
                create_if_missing: true,
            },
            pool: PoolConfig::default(),
            alias: alias.to_string(),
            cache,
            id_strategy: IdStrategy::Uuid,
        };
        
        manager.add_database(db_config).await?;
        info!("已添加数据库: {}", alias);
    }
    
    // 触发所有数据库的适配器创建
    for alias in ["test_mixed_cache_1", "test_mixed_non_cache_1", "test_mixed_cache_2", "test_mixed_non_cache_2"] {
        if let Some(pool) = manager.get_connection_pools().get(alias) {
            let mut test_data = HashMap::new();
            test_data.insert("test_field".to_string(), DataValue::String("test_value".to_string()));
            
            let (tx, rx) = oneshot::channel();
            let operation = DatabaseOperation::Create {
                table: "test_table".to_string(),
                data: test_data,
                response: tx,
            };
            
            if pool.operation_sender.send(operation).is_ok() {
                let _ = rx.await;
                info!("已触发数据库 '{}' 的适配器创建", alias);
            }
        }
    }
    
    Ok(())
}

/// 触发适配器创建
async fn trigger_adapter_creation(
    manager: &mut PoolManager,
    alias1: &str,
    alias2: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut test_data = HashMap::new();
    test_data.insert("test_field".to_string(), DataValue::String("test_value".to_string()));
    
    // 触发第一个数据库的适配器创建
    if let Some(pool1) = manager.get_connection_pools().get(alias1) {
        let (tx, rx) = oneshot::channel();
        let operation = DatabaseOperation::Create {
            table: "test_table".to_string(),
            data: test_data.clone(),
            response: tx,
        };
        
        if pool1.operation_sender.send(operation).is_ok() {
            let _ = rx.await;
            info!("已触发数据库 '{}' 的适配器创建", alias1);
        }
    }
    
    // 触发第二个数据库的适配器创建
    if let Some(pool2) = manager.get_connection_pools().get(alias2) {
        let (tx, rx) = oneshot::channel();
        let operation = DatabaseOperation::Create {
            table: "test_table".to_string(),
            data: test_data,
            response: tx,
        };
        
        if pool2.operation_sender.send(operation).is_ok() {
            let _ = rx.await;
            info!("已触发数据库 '{}' 的适配器创建", alias2);
        }
    }
    
    Ok(())
}