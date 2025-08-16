//! MongoDB缓存性能对比示例
//!
//! 本示例对比启用缓存和未启用缓存的MongoDB数据库操作性能差异
//! 使用 MongoDB 数据库进行测试，支持 TLS、认证和 ZSTD 压缩

use rat_quickdb::{
    types::*,
    odm::AsyncOdmManager,
    manager::{PoolManager, get_global_pool_manager},
    error::QuickDbResult,
    odm::OdmOperations,
    types::MongoDbConnectionBuilder,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use serde_json::json;
use uuid::Uuid;
use zerg_creep::{info, warn, error, debug};

/// 测试数据结构
#[derive(Debug, Clone)]
struct TestUser {
    id: String,
    name: String,
    email: String,
    age: i32,
    created_at: String,
}

impl TestUser {
    fn new(id: &str, name: &str, email: &str, age: i32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            email: email.to_string(),
            age,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut map = HashMap::new();
        map.insert("_id".to_string(), DataValue::String(self.id.clone())); // MongoDB使用_id
        map.insert("name".to_string(), DataValue::String(self.name.clone()));
        map.insert("email".to_string(), DataValue::String(self.email.clone()));
        map.insert("age".to_string(), DataValue::Int(self.age as i64));
        map.insert("created_at".to_string(), DataValue::String(self.created_at.clone()));
        map
    }
}

/// 性能测试结果
#[derive(Debug, Clone)]
struct PerformanceResult {
    operation: String,
    with_cache: Duration,
    without_cache: Duration,
    improvement_ratio: f64,
    cache_hit_rate: Option<f64>,
}

impl PerformanceResult {
    fn new(operation: String, with_cache: Duration, without_cache: Duration) -> Self {
        let improvement_ratio = if without_cache.as_millis() > 0 {
            with_cache.as_millis() as f64 / without_cache.as_millis() as f64
        } else {
            1.0
        };
        
        Self {
            operation,
            with_cache,
            without_cache,
            improvement_ratio,
            cache_hit_rate: None,
        }
    }

    fn with_cache_hit_rate(mut self, hit_rate: f64) -> Self {
        self.cache_hit_rate = Some(hit_rate);
        self
    }
}

/// 缓存性能测试器
struct CachePerformanceTest {
    /// ODM管理器
    odm: AsyncOdmManager,
    /// 测试结果
    results: Vec<PerformanceResult>,
    /// 表名
    table_name: String,
}

impl CachePerformanceTest {
    /// 创建新的性能测试实例
    async fn new() -> QuickDbResult<Self> {
        // 初始化日志
        zerg_creep::init_logger();
        
        // 创建带缓存的数据库配置
        let cached_config = Self::create_cached_database_config();
        
        // 创建不带缓存的数据库配置
        let non_cached_config = Self::create_non_cached_database_config();
        
        // 获取全局连接池管理器
        let pool_manager = get_global_pool_manager();
        
        // 添加数据库配置
        pool_manager.add_database(cached_config).await?;
        pool_manager.add_database(non_cached_config).await?;
        
        // 创建ODM管理器
        let odm = AsyncOdmManager::new();
        
        // 使用时间戳作为表名后缀，避免重复
        let timestamp = chrono::Utc::now().timestamp_millis();
        let table_name = format!("test_users_{}", timestamp);
        
        Ok(Self {
            odm,
            results: Vec::new(),
            table_name,
        })
    }
    
    /// 创建带缓存的MongoDB数据库配置
    fn create_cached_database_config() -> DatabaseConfig {
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
                options: Some({
                    let mut opts = std::collections::HashMap::new();
                    opts.insert("compressors".to_string(), "zstd".to_string());
                    opts
                }),
            },
            pool: PoolConfig {
                min_connections: 2,
                max_connections: 10,
                connection_timeout: 30,
                idle_timeout: 300,
                max_lifetime: 1800,
            },
            alias: "mongodb_cached".to_string(),
            cache: Some(CacheConfig {
                enabled: true,
                strategy: CacheStrategy::Lru,
                l1_config: L1CacheConfig {
                    max_capacity: 1000,
                    max_memory_mb: 100,
                    enable_stats: true,
                },
                l2_config: Some(L2CacheConfig {
                    storage_path: "/tmp/mongodb_cache_test".to_string(),
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
            id_strategy: IdStrategy::ObjectId,
        }
    }
    
    /// 创建不带缓存的MongoDB数据库配置
    fn create_non_cached_database_config() -> DatabaseConfig {
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
                options: Some({
                    let mut opts = std::collections::HashMap::new();
                    opts.insert("compressors".to_string(), "zstd".to_string());
                    opts
                }),
            },
            pool: PoolConfig {
                min_connections: 2,
                max_connections: 10,
                connection_timeout: 30,
                idle_timeout: 300,
                max_lifetime: 1800,
            },
            alias: "mongodb_non_cached".to_string(),
            cache: None, // 不启用缓存
            id_strategy: IdStrategy::ObjectId,
        }
    }
    
    /// 运行所有性能测试
    async fn run_all_tests(&mut self) -> QuickDbResult<()> {
        info!("开始MongoDB缓存性能对比测试");
        
        // 设置测试数据
        self.setup_test_data().await?;
        
        // 预热缓存
        self.warmup_cache().await?;
        
        // 运行各种测试
        self.test_query_operations().await?;
        self.test_repeated_queries().await?;
        self.test_batch_queries().await?;
        self.test_update_operations().await?;
        
        // 显示结果
        self.display_results();
        
        Ok(())
    }
    
    /// 设置测试数据
    async fn setup_test_data(&mut self) -> QuickDbResult<()> {
        info!("设置MongoDB测试数据");
        
        // 创建测试用户数据
        for i in 1..=100 {
            // 为缓存数据库创建用户
            let cached_user_id = format!("cached_user_{:03}_{}", i, uuid::Uuid::new_v4().simple());
            let cached_user = TestUser::new(
                &cached_user_id,
                &format!("用户{}", i),
                &format!("user{}@example.com", i),
                20 + (i % 50),
            );
            
            // 为非缓存数据库创建用户
            let non_cached_user_id = format!("non_cached_user_{:03}_{}", i, uuid::Uuid::new_v4().simple());
            let non_cached_user = TestUser::new(
                &non_cached_user_id,
                &format!("用户{}", i),
                &format!("user{}@example.com", i),
                20 + (i % 50),
            );
            
            // 插入到两个数据库
            self.odm.create(&self.table_name, cached_user.to_data_map(), Some("mongodb_cached")).await?;
            self.odm.create(&self.table_name, non_cached_user.to_data_map(), Some("mongodb_non_cached")).await?;
        }
        
        info!("MongoDB测试数据设置完成，使用表名: {}", self.table_name);
        Ok(())
    }
    
    /// 预热缓存
    async fn warmup_cache(&mut self) -> QuickDbResult<()> {
        info!("预热MongoDB缓存");
        
        // 执行一些查询来预热缓存
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "_id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("user_{}", i)),
            }];
            
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("mongodb_cached")).await;
        }
        
        info!("MongoDB缓存预热完成");
        Ok(())
    }
    
    /// 测试查询操作性能
    async fn test_query_operations(&mut self) -> QuickDbResult<()> {
        info!("测试MongoDB查询操作性能");
        
        // 测试单条记录查询
        let start = Instant::now();
        for i in 1..=50 {
            let conditions = vec![QueryCondition {
                field: "_id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("user_{}", i)),
            }];
            
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("mongodb_cached")).await?;
        }
        let cached_time = start.elapsed();
        
        let start = Instant::now();
        for i in 1..=50 {
            let conditions = vec![QueryCondition {
                field: "_id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("user_{}", i)),
            }];
            
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("mongodb_non_cached")).await?;
        }
        let non_cached_time = start.elapsed();
        
        self.results.push(PerformanceResult::new(
            "单条记录查询 (50次)".to_string(),
            cached_time,
            non_cached_time,
        ));
        
        Ok(())
    }
    
    /// 测试重复查询性能
    async fn test_repeated_queries(&mut self) -> QuickDbResult<()> {
        info!("测试MongoDB重复查询性能");
        
        let conditions = vec![QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        }];
        
        // 带缓存的重复查询
        let start = Instant::now();
        for _ in 0..20 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(QueryOptions::default()), Some("mongodb_cached")).await?;
        }
        let cached_time = start.elapsed();
        
        // 不带缓存的重复查询
        let start = Instant::now();
        for _ in 0..20 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(QueryOptions::default()), Some("mongodb_non_cached")).await?;
        }
        let non_cached_time = start.elapsed();
        
        self.results.push(PerformanceResult::new(
            "重复查询 (20次相同查询)".to_string(),
            cached_time,
            non_cached_time,
        ));
        
        Ok(())
    }
    
    /// 测试批量查询性能
    async fn test_batch_queries(&mut self) -> QuickDbResult<()> {
        info!("测试MongoDB批量查询性能");
        
        // 带缓存的批量查询
        let start = Instant::now();
        let query_options = QueryOptions {
            pagination: Some(PaginationConfig { skip: 0, limit: 50 }),
            ..Default::default()
        };
        let _ = self.odm.find(&self.table_name, vec![], Some(query_options), Some("mongodb_cached")).await?;
        let cached_time = start.elapsed();
        
        // 不带缓存的批量查询
        let start = Instant::now();
        let query_options = QueryOptions {
            pagination: Some(PaginationConfig { skip: 0, limit: 50 }),
            ..Default::default()
        };
        let _ = self.odm.find(&self.table_name, vec![], Some(query_options), Some("mongodb_non_cached")).await?;
        let non_cached_time = start.elapsed();
        
        self.results.push(PerformanceResult::new(
            "批量查询 (50条记录)".to_string(),
            cached_time,
            non_cached_time,
        ));
        
        Ok(())
    }
    
    /// 测试更新操作性能
    async fn test_update_operations(&mut self) -> QuickDbResult<()> {
        info!("测试MongoDB更新操作性能");
        
        let mut update_data = HashMap::new();
        update_data.insert("age".to_string(), DataValue::Int(30));
        
        // 带缓存的更新操作
        let start = Instant::now();
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "_id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("user_{}", i)),
            }];
            
            let _ = self.odm.update(&self.table_name, conditions, update_data.clone(), Some("mongodb_cached")).await?;
        }
        let cached_time = start.elapsed();
        
        // 不带缓存的更新操作
        let start = Instant::now();
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "_id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("user_{}", i)),
            }];
            
            let _ = self.odm.update(&self.table_name, conditions, update_data.clone(), Some("mongodb_non_cached")).await?;
        }
        let non_cached_time = start.elapsed();
        
        self.results.push(PerformanceResult::new(
            "更新操作 (10次)".to_string(),
            cached_time,
            non_cached_time,
        ));
        
        Ok(())
    }
    
    /// 显示测试结果
    fn display_results(&self) {
        info!("\n=== MongoDB缓存性能对比测试结果 ===");
        info!("连接配置: db0.0ldm0s.net:27017 (TLS + ZSTD + directConnection)");
        info!("数据库: testdb, 认证源: testdb");
        info!("{:-<80}", "");
        info!("{:<25} {:<15} {:<15} {:<15} {:<10}", "操作类型", "带缓存(ms)", "无缓存(ms)", "性能提升", "缓存命中率");
        info!("{:-<80}", "");
        
        for result in &self.results {
            let improvement = if result.improvement_ratio < 1.0 {
                format!("{:.1}x 更快", 1.0 / result.improvement_ratio)
            } else {
                format!("{:.1}x 更慢", result.improvement_ratio)
            };
            
            let hit_rate = result.cache_hit_rate
                .map(|rate| format!("{:.1}%", rate * 100.0))
                .unwrap_or_else(|| "N/A".to_string());
            
            info!(
                "{:<25} {:<15} {:<15} {:<15} {:<10}",
                result.operation,
                result.with_cache.as_millis(),
                result.without_cache.as_millis(),
                improvement,
                hit_rate
            );
        }
        
        info!("{:-<80}", "");
        
        // 计算总体性能提升
        let total_cached: u128 = self.results.iter().map(|r| r.with_cache.as_millis()).sum();
        let total_non_cached: u128 = self.results.iter().map(|r| r.without_cache.as_millis()).sum();
        
        if total_non_cached > 0 {
            let overall_improvement = total_cached as f64 / total_non_cached as f64;
            if overall_improvement < 1.0 {
                info!("总体性能提升: {:.1}x 更快", 1.0 / overall_improvement);
            } else {
                info!("总体性能变化: {:.1}x 更慢", overall_improvement);
            }
        }
        
        info!("\n=== 测试说明 ===");
        info!("• 使用MongoDB数据库进行测试");
        info!("• 启用TLS加密连接");
        info!("• 启用ZSTD压缩");
        info!("• 使用directConnection=true直连模式");
        info!("• 缓存策略: LRU");
        info!("• L1缓存: 内存缓存，最大1000条记录");
        info!("• L2缓存: 磁盘缓存，最大500MB");
        info!("• TTL: 默认5分钟，最大1小时");
        info!("• 压缩: ZSTD算法，阈值1KB");
        
        info!("\n=== 使用场景建议 ===");
        info!("• 频繁查询相同数据时，缓存能显著提升性能");
        info!("• 对于写入密集型应用，缓存的性能提升有限");
        info!("• 网络延迟较高时，缓存的效果更明显");
        info!("• 建议根据实际业务场景调整缓存配置");
    }
}

/// 清理测试文件
async fn cleanup_test_files() {
    let _ = tokio::fs::remove_dir_all("/tmp/mongodb_cache_test").await;
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 清理之前的测试文件
    cleanup_test_files().await;
    
    // 创建并运行性能测试
    let mut test = CachePerformanceTest::new().await?;
    test.run_all_tests().await?;
    
    // 清理测试文件
    cleanup_test_files().await;
    
    info!("MongoDB缓存性能对比测试完成");
    Ok(())
}