//! PostgreSQL缓存性能对比示例
//!
//! 本示例对比启用缓存和未启用缓存的PostgreSQL数据库操作性能差异
//! 使用 PostgreSQL 数据库进行测试，支持 TLS 和 SSL 连接

use rat_quickdb::{
    types::*,
    odm::AsyncOdmManager,
    manager::{PoolManager, get_global_pool_manager},
    error::{QuickDbResult, QuickDbError},
    odm::OdmOperations,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use serde_json::json;
use uuid::Uuid;
use rat_logger::{info, warn, error, debug};

/// 测试数据结构
#[derive(Debug, Clone)]
struct TestUser {
    id: i32,
    name: String,
    email: String,
    age: i32,
    created_at: String,
}

impl TestUser {
    fn new(id: i32, name: &str, email: &str, age: i32) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
            age,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), DataValue::Int(self.id as i64)); // PostgreSQL使用id
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
    
    /// 创建带缓存的PostgreSQL数据库配置
    fn create_cached_database_config() -> DatabaseConfig {
        DatabaseConfig {
            db_type: DatabaseType::PostgreSQL,
            connection: ConnectionConfig::PostgreSQL {
                host: "172.16.0.23".to_string(),
                port: 5432,
                database: "testdb".to_string(),
                username: "testdb".to_string(),
                password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
                ssl_mode: Some("prefer".to_string()),
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
            },
            pool: PoolConfig {
                min_connections: 2,
                max_connections: 10,
                connection_timeout: 30,
                idle_timeout: 300,
                max_lifetime: 1800,
            },
            alias: "pgsql_cached".to_string(),
            cache: Some(CacheConfig {
                enabled: true,
                strategy: CacheStrategy::Lru,
                l1_config: L1CacheConfig {
                    max_capacity: 1000,
                    max_memory_mb: 100,
                    enable_stats: true,
                },
                l2_config: Some(L2CacheConfig {
                    storage_path: "./cache/pgsql_cache_test".to_string(),
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
                version: "v1".to_string(),
            }),
            id_strategy: IdStrategy::AutoIncrement,
        }
    }
    
    /// 创建不带缓存的PostgreSQL数据库配置
    fn create_non_cached_database_config() -> DatabaseConfig {
        DatabaseConfig {
            db_type: DatabaseType::PostgreSQL,
            connection: ConnectionConfig::PostgreSQL {
                host: "172.16.0.23".to_string(),
                port: 5432,
                database: "testdb".to_string(),
                username: "testdb".to_string(),
                password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
                ssl_mode: Some("prefer".to_string()),
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
            },
            pool: PoolConfig {
                min_connections: 2,
                max_connections: 10,
                connection_timeout: 30,
                idle_timeout: 300,
                max_lifetime: 1800,
            },
            alias: "pgsql_non_cached".to_string(),
            cache: None, // 不启用缓存
            id_strategy: IdStrategy::AutoIncrement,
        }
    }
    
    /// 运行所有性能测试
    async fn run_all_tests(&mut self) -> QuickDbResult<()> {
        info!("开始PostgreSQL缓存性能对比测试");
        
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
        info!("设置PostgreSQL测试数据");
        
        // 创建测试用户数据，为不同数据库使用不同的ID范围避免冲突
        for i in 1..=100 {
            // 为缓存数据库创建用户（使用1000+i的ID）
            let cached_user_id = 1000 + i;
            let cached_user = TestUser::new(
                cached_user_id,
                &format!("缓存用户{}", i),
                &format!("cached_user{}@example.com", i),
                20 + (i % 50),
            );
            
            // 为非缓存数据库创建用户（使用2000+i的ID）
            let non_cached_user_id = 2000 + i;
            let non_cached_user = TestUser::new(
                non_cached_user_id,
                &format!("非缓存用户{}", i),
                &format!("non_cached_user{}@example.com", i),
                20 + (i % 50),
            );
            
            // 插入到两个数据库
            self.odm.create(&self.table_name, cached_user.to_data_map(), Some("pgsql_cached")).await?;
            self.odm.create(&self.table_name, non_cached_user.to_data_map(), Some("pgsql_non_cached")).await?;
        }
        
        info!("PostgreSQL测试数据设置完成，使用表名: {}，共创建200条记录（每个数据库100条）", self.table_name);
        Ok(())
    }
    
    /// 预热缓存
    async fn warmup_cache(&mut self) -> QuickDbResult<()> {
        info!("预热PostgreSQL缓存");
        
        // 执行一些查询来预热缓存，使用缓存数据库的ID范围
        for i in 1..=20 {
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int((1000 + i) as i64),
            }];
            
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("pgsql_cached")).await;
        }
        
        // 预热一些范围查询
        let age_conditions = vec![QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        }];
        let _ = self.odm.find(&self.table_name, age_conditions, Some(QueryOptions::default()), Some("pgsql_cached")).await;
        
        info!("PostgreSQL缓存预热完成，预热了20条单记录查询和1次范围查询");
        Ok(())
    }
    
    /// 测试查询操作性能
    async fn test_query_operations(&mut self) -> QuickDbResult<()> {
        info!("测试PostgreSQL查询操作性能");
        
        // 测试缓存数据库的单条记录查询
        let start = Instant::now();
        for i in 1..=50 {
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int((1000 + i) as i64),
            }];
            
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("pgsql_cached")).await?;
        }
        let cached_time = start.elapsed();
        
        // 测试非缓存数据库查询
        let start = Instant::now();
        for i in 1..=50 {
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int((2000 + i) as i64),
            }];
            
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("pgsql_non_cached")).await?;
        }
        let non_cached_time = start.elapsed();
        
        self.results.push(PerformanceResult::new(
            "单条记录查询 (50次)".to_string(),
            cached_time,
            non_cached_time,
        ));
        
        info!("PostgreSQL查询操作性能测试完成 - 缓存: {:?}, 非缓存: {:?}", cached_time, non_cached_time);
        Ok(())
    }
    
    /// 测试重复查询性能
    async fn test_repeated_queries(&mut self) -> QuickDbResult<()> {
        info!("测试PostgreSQL重复查询性能 - 大量重复查询场景");
        
        // 测试缓存数据库的重复查询 - 增加查询次数和多样性
        let cached_conditions_1 = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(1001),
        }];
        
        let cached_conditions_2 = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(1002),
        }];
        
        let cached_conditions_3 = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(1003),
        }];
        
        // 带缓存的重复查询 - 大幅增加查询次数
        let start = Instant::now();
        for i in 0..500 {
            // 重复查询相同的几条记录，确保缓存命中
            match i % 3 {
                0 => { let _ = self.odm.find(&self.table_name, cached_conditions_1.clone(), Some(QueryOptions::default()), Some("pgsql_cached")).await?; }
                1 => { let _ = self.odm.find(&self.table_name, cached_conditions_2.clone(), Some(QueryOptions::default()), Some("pgsql_cached")).await?; }
                _ => { let _ = self.odm.find(&self.table_name, cached_conditions_3.clone(), Some(QueryOptions::default()), Some("pgsql_cached")).await?; }
            }
        }
        let cached_time = start.elapsed();
        
        // 测试非缓存数据库的重复查询
        let non_cached_conditions_1 = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(2001),
        }];
        
        let non_cached_conditions_2 = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(2002),
        }];
        
        let non_cached_conditions_3 = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(2003),
        }];
        
        // 不带缓存的重复查询 - 相同的查询次数
        let start = Instant::now();
        for i in 0..500 {
            match i % 3 {
                0 => { let _ = self.odm.find(&self.table_name, non_cached_conditions_1.clone(), Some(QueryOptions::default()), Some("pgsql_non_cached")).await?; }
                1 => { let _ = self.odm.find(&self.table_name, non_cached_conditions_2.clone(), Some(QueryOptions::default()), Some("pgsql_non_cached")).await?; }
                _ => { let _ = self.odm.find(&self.table_name, non_cached_conditions_3.clone(), Some(QueryOptions::default()), Some("pgsql_non_cached")).await?; }
            }
        }
        let non_cached_time = start.elapsed();
        
        // 获取缓存统计信息
        let cache_stats = match get_global_pool_manager().get_cache_stats("pgsql_cached").await {
            Ok(stats) => {
                info!("缓存统计 - 命中: {}, 未命中: {}, 命中率: {:.1}%", 
                      stats.hits, stats.misses, stats.hit_rate * 100.0);
                Some(stats.hit_rate * 100.0)
            }
            Err(e) => {
                warn!("获取缓存统计失败: {}", e);
                None
            }
        };
        
        let mut result = PerformanceResult::new(
            "重复查询 (500次重复查询，3个不同ID循环)".to_string(),
            cached_time,
            non_cached_time,
        );
        
        if let Some(hit_rate) = cache_stats {
            result = result.with_cache_hit_rate(hit_rate);
        }
        
        self.results.push(result);
        
        info!("PostgreSQL重复查询性能测试完成 - 缓存: {:?}, 非缓存: {:?}", cached_time, non_cached_time);
        Ok(())
    }
    
    /// 测试批量查询性能
    async fn test_batch_queries(&mut self) -> QuickDbResult<()> {
        info!("测试PostgreSQL批量查询性能");
        
        // 带缓存的批量查询（混合ID查询和范围查询）
        let start = Instant::now();
        // 先查询一些具体ID（这些应该命中缓存）
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int((1000 + i) as i64),
            }];
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("pgsql_cached")).await?;
        }
        // 再查询一些年龄范围
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int(20 + (i % 50)),
            }];
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("pgsql_cached")).await?;
        }
        let cached_time = start.elapsed();
        
        // 不带缓存的批量查询（相同的查询模式）
        let start = Instant::now();
        // 查询相同的ID
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int((2000 + i) as i64),
            }];
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("pgsql_non_cached")).await?;
        }
        // 查询相同的年龄范围
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int(20 + (i % 50)),
            }];
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("pgsql_non_cached")).await?;
        }
        let non_cached_time = start.elapsed();
        
        self.results.push(PerformanceResult::new(
            "批量查询 (10次ID查询 + 10次年龄查询)".to_string(),
            cached_time,
            non_cached_time,
        ));
        
        info!("PostgreSQL批量查询性能测试完成 - 缓存: {:?}, 非缓存: {:?}", cached_time, non_cached_time);
        Ok(())
    }
    
    /// 测试更新操作性能
    async fn test_update_operations(&mut self) -> QuickDbResult<()> {
        info!("测试PostgreSQL更新操作性能");
        
        let mut update_data = HashMap::new();
        update_data.insert("age".to_string(), DataValue::Int(30));
        
        // 带缓存的更新操作
        let start = Instant::now();
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int((1000 + i) as i64),
            }];
            
            let _ = self.odm.update(&self.table_name, conditions, update_data.clone(), Some("pgsql_cached")).await?;
        }
        let cached_time = start.elapsed();
        
        // 不带缓存的更新操作
        let start = Instant::now();
        for i in 1..=10 {
            let conditions = vec![QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::Int((2000 + i) as i64),
            }];
            
            let _ = self.odm.update(&self.table_name, conditions, update_data.clone(), Some("pgsql_non_cached")).await?;
        }
        let non_cached_time = start.elapsed();
        
        self.results.push(PerformanceResult::new(
            "更新操作 (10次)".to_string(),
            cached_time,
            non_cached_time,
        ));
        
        info!("PostgreSQL更新操作性能测试完成 - 缓存: {:?}, 非缓存: {:?}", cached_time, non_cached_time);
        Ok(())
    }
    
    /// 显示测试结果
    fn display_results(&self) {
        info!("\n=== PostgreSQL缓存性能测试结果 ===");
        
        let mut total_cached_time = Duration::new(0, 0);
        let mut total_non_cached_time = Duration::new(0, 0);
        
        for result in &self.results {
            total_cached_time += result.with_cache;
            total_non_cached_time += result.without_cache;
            
            let improvement = if result.improvement_ratio < 1.0 {
                format!("快了 {:.1} 倍", 1.0 / result.improvement_ratio)
            } else {
                format!("慢了 {:.1} 倍", result.improvement_ratio)
            };
            
            let cache_info = if let Some(hit_rate) = result.cache_hit_rate {
                format!(" (缓存命中率: {:.1}%)", hit_rate)
            } else {
                String::new()
            };
            
            info!(
                "操作: {} | 缓存: {:?} | 非缓存: {:?} | 性能: {}{}",
                result.operation,
                result.with_cache,
                result.without_cache,
                improvement,
                cache_info
            );
        }
        
        // 计算总体性能提升
        let total_improvement = if total_cached_time.as_millis() > 0 {
            total_non_cached_time.as_millis() as f64 / total_cached_time.as_millis() as f64
        } else {
            1.0
        };
        
        info!("\n=== 总体性能对比 ===");
        info!("总缓存时间: {:?}", total_cached_time);
        info!("总非缓存时间: {:?}", total_non_cached_time);
        info!("总体性能提升: {:.1} 倍", total_improvement);
        
        if total_improvement > 1.0 {
            info!("✅ 缓存显著提升了PostgreSQL数据库操作性能！");
        } else {
            warn!("⚠️  缓存未能提升性能，可能需要调整缓存配置或测试场景");
        }
    }
}

/// 设置缓存目录
async fn setup_cache_directory() -> QuickDbResult<()> {
    let cache_dir = std::path::Path::new("./cache/pgsql_cache_test");
    if !cache_dir.exists() {
        tokio::fs::create_dir_all(cache_dir).await
            .map_err(|e| QuickDbError::ConfigError { message: format!("创建缓存目录失败: {}", e) })?;
    }
    Ok(())
}

/// 清理测试文件
async fn cleanup_test_files() {
    // PostgreSQL不需要清理本地文件，数据在远程数据库中
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志系统
    rat_logger::LoggerBuilder::new().add_terminal_with_config(rat_logger::handler::term::TermConfig::default()).init().expect("日志初始化失败");
    
    info!("开始PostgreSQL缓存性能对比测试");
    
    // 设置缓存目录
    setup_cache_directory().await?;
    
    // 创建并运行性能测试
    let mut test = CachePerformanceTest::new().await?;
    test.run_all_tests().await?;
    
    // 清理测试文件
    cleanup_test_files().await;
    
    info!("PostgreSQL缓存性能对比测试完成");
    Ok(())
}