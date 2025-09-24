//! MongoDB缓存配置隔离性测试
//!
//! 本测试验证在共享PoolManager实例时，MongoDB数据库的缓存配置是否会相互影响
//! 测试三种场景：
//! 1. 先添加缓存数据库，再添加非缓存数据库
//! 2. 先添加非缓存数据库，再添加缓存数据库
//! 3. 混合模式：交替添加缓存和非缓存数据库

use rat_quickdb::{
    types::*,
    manager::{PoolManager, get_global_pool_manager},
    error::{QuickDbResult, QuickDbError},
    pool::DatabaseOperation,
};
use std::collections::HashMap;
use tokio::sync::oneshot;
use rat_logger::{info, warn, error};

/// 创建带缓存的MongoDB数据库配置
fn create_cached_mongodb_config(alias: &str) -> DatabaseConfig {
    DatabaseConfig {
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
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
            zstd_config: Some(ZstdConfig {
                enabled: true,
                compression_level: Some(3),
                compression_threshold: Some(1024),
            }),
            options: None,
        },
        pool: PoolConfig {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: alias.to_string(),
        cache: Some(CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: true,
            },
            l2_config: Some(L2CacheConfig {
                storage_path: format!("./cache/mongodb_cache_test_{}", alias),
                max_disk_mb: 500,
                compression_level: 6,
                enable_wal: true,
                clear_on_startup: false,
            }),
            ttl_config: TtlConfig {
                default_ttl_secs: 300,
                max_ttl_secs: 3600,
                check_interval_secs: 60,
            },
            compression_config: CompressionConfig {
                enabled: true,
                algorithm: CompressionAlgorithm::Zstd,
                threshold_bytes: 1024,
            },
        }),
        id_strategy: IdStrategy::ObjectId,
    }
}

/// 创建不带缓存的MongoDB数据库配置
fn create_non_cached_mongodb_config(alias: &str) -> DatabaseConfig {
    DatabaseConfig {
        db_type: DatabaseType::MongoDB,
        connection: ConnectionConfig::MongoDB {
            host: "db0.0ldm0s.net".to_string(),
            port: 27017,
            database: "testdb".to_string(),
            username: Some("testdb".to_string()),
            password: Some("yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string()),
            auth_source: Some("testdb".to_string()),
            direct_connection: true,
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
            zstd_config: Some(ZstdConfig {
                enabled: true,
                compression_level: Some(3),
                compression_threshold: Some(1024),
            }),
            options: None,
        },
        pool: PoolConfig {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: alias.to_string(),
        cache: None, // 不启用缓存
        id_strategy: IdStrategy::ObjectId,
    }
}

/// 创建所有需要的缓存目录
async fn setup_cache_directories() -> QuickDbResult<()> {
    let cache_dirs = vec![
        "./cache/mongodb_cache_test_mongodb_cached_first",
        "./cache/mongodb_cache_test_mongodb_cached_second", 
        "./cache/mongodb_cache_test_mongodb_mixed_cache_1",
        "./cache/mongodb_cache_test_mongodb_mixed_cache_2",
    ];
    
    for cache_dir in cache_dirs {
        if let Err(e) = tokio::fs::create_dir_all(cache_dir).await {
            error!("创建缓存目录失败: {}, 错误: {}", cache_dir, e);
            return Err(QuickDbError::ConfigError { message: format!("创建缓存目录失败: {}", e) });
        } else {
            info!("缓存目录创建成功: {}", cache_dir);
        }
    }
    Ok(())
}

/// 触发适配器创建的辅助函数
async fn trigger_adapter_creation(pool_manager: &PoolManager, db_alias: &str) -> QuickDbResult<()> {
    // 获取连接池
        let pools = pool_manager.get_connection_pools();
        if let Some(pool) = pools.get(db_alias) {
        // 创建一个简单的测试数据
        let mut test_data = HashMap::new();
        test_data.insert("_id".to_string(), DataValue::String(format!("test_id_{}", db_alias)));
        test_data.insert("name".to_string(), DataValue::String("测试用户".to_string()));
        test_data.insert("email".to_string(), DataValue::String("test@example.com".to_string()));
        
        // 发送创建操作
        let (tx, rx) = oneshot::channel();
        let operation = DatabaseOperation::Create {
            table: "test_table".to_string(),
            data: test_data,
            response: tx,
        };
        
        if let Err(e) = pool.operation_sender.send(operation) {
            warn!("发送数据库操作失败: {}", e);
        } else {
            // 等待响应
            match rx.await {
                Ok(_) => info!("已触发数据库 '{}' 的适配器创建", db_alias),
                Err(e) => warn!("等待数据库操作响应失败: {}", e),
            }
        }
    } else {
        warn!("未找到数据库 '{}' 的连接池", db_alias);
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    rat_logger::LoggerBuilder::new().add_terminal_with_config(rat_logger::handler::term::TermConfig::default()).init().expect("日志初始化失败").expect("日志初始化失败");
    
    info!("=== MongoDB缓存配置隔离性测试 ===");
    
    // 创建所有需要的缓存目录
    setup_cache_directories().await?;
    info!("所有缓存目录创建完成");
    
    // 测试场景1：先缓存后非缓存
    {
        info!("创建连接池管理器");
        let pool_manager = PoolManager::new();
        
        info!("");
        info!("--- 测试场景1：先缓存后非缓存 ---");
        
        // 添加缓存数据库
        let cached_config = create_cached_mongodb_config("mongodb_cached_first");
        pool_manager.add_database(cached_config).await?;
        info!("已添加缓存数据库: mongodb_cached_first");
        
        // 添加非缓存数据库
        let non_cached_config = create_non_cached_mongodb_config("mongodb_non_cached_second");
        pool_manager.add_database(non_cached_config).await?;
        info!("已添加非缓存数据库: mongodb_non_cached_second");
        
        // 触发适配器创建
        trigger_adapter_creation(&pool_manager, "mongodb_cached_first").await?;
        trigger_adapter_creation(&pool_manager, "mongodb_non_cached_second").await?;
    }
    
    // 测试场景2：先非缓存后缓存
    {
        info!("创建连接池管理器");
        let pool_manager = PoolManager::new();
        
        info!("");
        info!("--- 测试场景2：先非缓存后缓存 ---");
        
        // 添加非缓存数据库
        let non_cached_config = create_non_cached_mongodb_config("mongodb_non_cached_first");
        pool_manager.add_database(non_cached_config).await?;
        info!("已添加非缓存数据库: mongodb_non_cached_first");
        
        // 添加缓存数据库
        let cached_config = create_cached_mongodb_config("mongodb_cached_second");
        pool_manager.add_database(cached_config).await?;
        info!("已添加缓存数据库: mongodb_cached_second");
        
        // 触发适配器创建
        trigger_adapter_creation(&pool_manager, "mongodb_non_cached_first").await?;
        trigger_adapter_creation(&pool_manager, "mongodb_cached_second").await?;
    }
    
    // 测试场景3：混合模式
    {
        info!("创建连接池管理器");
        let pool_manager = PoolManager::new();
        
        info!("");
        info!("--- 测试场景3：混合模式 ---");
        
        // 交替添加缓存和非缓存数据库
        let cached_config_1 = create_cached_mongodb_config("mongodb_mixed_cache_1");
        pool_manager.add_database(cached_config_1).await?;
        info!("已添加数据库: mongodb_mixed_cache_1");
        
        let non_cached_config_1 = create_non_cached_mongodb_config("mongodb_mixed_non_cache_1");
        pool_manager.add_database(non_cached_config_1).await?;
        info!("已添加数据库: mongodb_mixed_non_cache_1");
        
        let cached_config_2 = create_cached_mongodb_config("mongodb_mixed_cache_2");
        pool_manager.add_database(cached_config_2).await?;
        info!("已添加数据库: mongodb_mixed_cache_2");
        
        let non_cached_config_2 = create_non_cached_mongodb_config("mongodb_mixed_non_cache_2");
        pool_manager.add_database(non_cached_config_2).await?;
        info!("已添加数据库: mongodb_mixed_non_cache_2");
        
        // 触发所有适配器创建
        trigger_adapter_creation(&pool_manager, "mongodb_mixed_cache_1").await?;
        trigger_adapter_creation(&pool_manager, "mongodb_mixed_non_cache_1").await?;
        trigger_adapter_creation(&pool_manager, "mongodb_mixed_cache_2").await?;
        trigger_adapter_creation(&pool_manager, "mongodb_mixed_non_cache_2").await?;
    }
    
    info!("");
    info!("=== 测试完成 ===");
    
    Ok(())
}