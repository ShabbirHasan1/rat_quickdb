//! 测试缓存配置失败时的行为
//!
//! 验证移除容错降级机制后，缓存管理器创建失败时数据库添加操作会直接失败

use rat_quickdb::{
    types::*,
    manager::PoolManager,
    error::{QuickDbResult, QuickDbError},
};
use zerg_creep::{info, error};

/// 测试缓存配置失败时的行为
#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    zerg_creep::init_logger().expect("日志初始化失败");
    
    info!("开始测试缓存配置失败时的行为");
    
    // 创建连接池管理器
    let pool_manager = PoolManager::new();
    
    // 创建数据库配置，使用不存在的缓存目录
    // 这应该会导致缓存管理器创建失败
    let db_config = DatabaseConfig {
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
        alias: "test_db".to_string(),
        cache: Some(CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 50,
                enable_stats: true,
            },
            l2_config: Some(L2CacheConfig {
                storage_path: "/non_existent_directory/cache".to_string(),
                max_disk_mb: 100,
                compression_level: 3,
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
        id_strategy: IdStrategy::AutoIncrement,
    };
    
    info!("尝试添加带有失败缓存配置的数据库...");
    
    // 尝试添加数据库 - 这应该会失败，因为缓存目录不存在
    match pool_manager.add_database(db_config).await {
        Ok(_) => {
            error!("❌ 预期失败但成功了！这表明容错机制仍然存在。");
            return Err(QuickDbError::ConfigError {
                message: "测试失败：缓存配置失败时应该直接报错，而不是降级".to_string(),
            });
        }
        Err(e) => {
            info!("✅ 测试成功：缓存配置失败时正确报错: {}", e);
        }
    }
    
    // 验证数据库别名列表为空（没有成功添加任何数据库）
    let aliases = pool_manager.get_aliases();
    if aliases.is_empty() {
        info!("✅ 验证成功：数据库别名列表为空，确认没有添加任何数据库");
    } else {
        error!("❌ 验证失败：数据库别名列表不为空: {:?}", aliases);
        return Err(QuickDbError::ConfigError {
            message: "验证失败：期望空列表但实际有数据".to_string(),
        });
    }
    
    info!("🎉 测试完成：缓存初始化失败时的容错机制已成功移除");
    Ok(())
}