//! 测试缓存配置失败时的错误处理
//! 
//! 验证移除容错机制后，缓存初始化失败时会直接报错而不是降级为普通适配器

use rat_quickdb::{
    manager::PoolManager,
    types::*,
    error::QuickDbResult,
};
use std::path::Path;
use tokio::fs;
use zerg_creep::{info, error};

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初志化日志
    zerg_creep::init_logger().expect("日志初始化失败");
    
    info!("开始测试缓存配置失败时的错误处理");
    
    // 测试1: 使用无效的缓存路径（只读目录）
    info!("\n=== 测试1: 使用无效的缓存路径 ===");
    test_invalid_cache_path().await;
    
    // 测试2: 使用不存在且无法创建的缓存路径
    info!("\n=== 测试2: 使用不存在且无法创建的缓存路径 ===");
    test_uncreatable_cache_path().await;
    
    info!("\n=== 测试完成 ===");
    Ok(())
}

/// 测试使用无效缓存路径（只读目录）
async fn test_invalid_cache_path() -> QuickDbResult<()> {
    info!("测试场景1：使用只读目录作为缓存路径");
    
    // 创建一个只读目录
    let readonly_dir = "/tmp/readonly_cache_test";
    fs::create_dir_all(readonly_dir).await?;
    
    // 设置为只读
    let mut perms = fs::metadata(readonly_dir).await?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(readonly_dir, perms).await?;
    
    let pool_manager = PoolManager::new();
    
    // 尝试创建带缓存的数据库配置
    let config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
             path: ":memory:".to_string(),
             create_if_missing: true,
         },
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: "test_readonly_cache".to_string(),
        cache: Some(CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: true,
            },
            l2_config: Some(L2CacheConfig {
                storage_path: readonly_dir.to_string(),
                max_disk_mb: 500,
                compression_level: 6,
                enable_wal: true,
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
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    // 这里应该失败并返回错误
    match pool_manager.add_database(config).await {
        Ok(_) => {
            error!("预期失败但成功了！缓存降级机制仍然存在");
            return Err(rat_quickdb::error::QuickDbError::ConfigError {
                message: "缓存降级机制未被移除".to_string()
            });
        }
        Err(e) => {
            info!("✓ 正确失败：{}", e);
        }
    }
    
    // 清理
    let _ = fs::remove_dir_all(readonly_dir).await;
    
    Ok(())
}

/// 测试使用不存在且无法创建的缓存路径
async fn test_uncreatable_cache_path() -> QuickDbResult<()> {
    info!("测试场景2：使用无法创建的缓存路径");
    
    let pool_manager = PoolManager::new();
    
    // 使用一个无法创建的路径（包含非法字符）
    let invalid_path = "/dev/null/invalid_cache_path";
    
    let config = DatabaseConfig {
        db_type: DatabaseType::SQLite,
        connection: ConnectionConfig::SQLite {
             path: ":memory:".to_string(),
             create_if_missing: true,
         },
        pool: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: 30,
            idle_timeout: 300,
            max_lifetime: 1800,
        },
        alias: "test_uncreatable_cache".to_string(),
        cache: Some(CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: true,
            },
            l2_config: Some(L2CacheConfig {
                storage_path: invalid_path.to_string(),
                max_disk_mb: 500,
                compression_level: 6,
                enable_wal: true,
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
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    // 这里应该失败并返回错误
    match pool_manager.add_database(config).await {
        Ok(_) => {
            error!("预期失败但成功了！缓存降级机制仍然存在");
            return Err(rat_quickdb::error::QuickDbError::ConfigError {
                message: "缓存降级机制未被移除".to_string()
            });
        }
        Err(e) => {
            info!("✓ 正确失败：{}", e);
        }
    }
    
    Ok(())
}