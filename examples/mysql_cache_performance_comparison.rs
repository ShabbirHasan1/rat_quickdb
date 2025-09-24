//! MySQL 缓存性能对比示例
//!
//! 本示例演示了 MySQL 数据库在启用和未启用缓存时的性能差异
//! 测试包括：
//! - 单条记录查询性能
//! - 重复查询性能（缓存命中）
//! - 批量查询性能

use rat_quickdb::{
    types::*,
    odm::AsyncOdmManager,
    manager::{PoolManager, get_global_pool_manager},
    error::{QuickDbResult, QuickDbError},
    odm::OdmOperations,
};
use rat_quickdb::types::PaginationConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::fs;
use rat_logger::{info, warn, error, debug};
use chrono;

/// 测试用户结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestUser {
    id: i64,
    name: String,
    email: String,
    age: i32,
    city: String,
}

impl TestUser {
    /// 转换为数据映射（不包含id，让数据库自动生成）
    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut data = HashMap::new();
        // 不包含id字段，让MySQL自动生成自增主键
        data.insert("name".to_string(), DataValue::String(self.name.clone()));
        data.insert("email".to_string(), DataValue::String(self.email.clone()));
        data.insert("age".to_string(), DataValue::Int(self.age as i64));
        data.insert("city".to_string(), DataValue::String(self.city.clone()));
        data
    }
}

/// 缓存性能测试器
struct CachePerformanceTest {
    test_data: Vec<TestUser>,
    odm: AsyncOdmManager,
    results: Vec<PerformanceResult>,
    table_name: String,
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
        let improvement_ratio = if with_cache.as_millis() > 0 {
            without_cache.as_millis() as f64 / with_cache.as_millis() as f64
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

impl CachePerformanceTest {

    /// 创建带缓存的数据库配置
    fn create_cached_database_config() -> DatabaseConfig {
        let pool_config = PoolConfig {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 3600,
        };

        let cache_config = CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: true,
            },
            l2_config: Some(L2CacheConfig {
                storage_path: "./cache/mysql_cache_test".to_string(),
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
        };

        DatabaseConfig {
            alias: "mysql_cached_db".to_string(),
            db_type: DatabaseType::MySQL,
            connection: ConnectionConfig::MySQL {
                host: "172.16.0.21".to_string(),
                port: 3306,
                database: "testdb".to_string(),
                username: "testdb".to_string(),
                password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
                ssl_opts: None,
                tls_config: None,
            },
            pool: pool_config,
            id_strategy: IdStrategy::AutoIncrement,
            cache: Some(cache_config),
        }
    }

    /// 创建不带缓存的数据库配置
    fn create_non_cached_database_config() -> DatabaseConfig {
        let pool_config = PoolConfig {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 3600,
        };

        DatabaseConfig {
            alias: "mysql_non_cached_db".to_string(),
            db_type: DatabaseType::MySQL,
            connection: ConnectionConfig::MySQL {
                host: "172.16.0.21".to_string(),
                port: 3306,
                database: "testdb".to_string(),
                username: "testdb".to_string(),
                password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
                ssl_opts: None,
                tls_config: None,
            },
            pool: pool_config,
            id_strategy: IdStrategy::AutoIncrement,
            cache: None,
        }
    }

    /// 创建新的性能测试器
    async fn new() -> QuickDbResult<Self> {
        // 创建数据库配置
        let cached_config = Self::create_cached_database_config();
        let non_cached_config = Self::create_non_cached_database_config();

        // 获取全局连接池管理器
        let pool_manager = get_global_pool_manager();
        
        // 添加数据库配置
        pool_manager.add_database(cached_config).await?;
        pool_manager.add_database(non_cached_config).await?;

        // 创建ODM管理器
        let odm = AsyncOdmManager::new();

        let test_data = (1..=1000)
            .map(|i| TestUser {
                id: i,
                name: format!("用户{}", i),
                email: format!("user{}@example.com", i),
                age: 20 + (i % 50) as i32,
                city: match i % 5 {
                    0 => "北京".to_string(),
                    1 => "上海".to_string(),
                    2 => "广州".to_string(),
                    3 => "深圳".to_string(),
                    _ => "杭州".to_string(),
                },
            })
            .collect();

        // 使用时间戳作为表名后缀，避免重复
        let timestamp = chrono::Utc::now().timestamp_millis();
        let table_name = format!("test_users_{}", timestamp);
        
        Ok(Self { test_data, odm, results: Vec::new(), table_name })
    }

    /// 运行所有性能测试
    async fn run_all_tests(&mut self) -> QuickDbResult<()> {
        // 准备测试数据
        self.prepare_test_data().await?;

        // 测试单条记录查询
        self.test_single_query_performance().await?;

        // 测试重复查询（缓存命中）
        self.test_repeat_query_performance().await?;

        // 测试批量查询
        self.test_batch_query_performance().await?;

        Ok(())
    }

    /// 准备测试数据
    async fn prepare_test_data(&self) -> QuickDbResult<()> {
        info!("开始准备测试数据...");

        // 清理现有数据（删除所有记录）
        let delete_conditions = vec![];
        let _ = self.odm.delete(&self.table_name, delete_conditions.clone(), Some("mysql_cached_db")).await;
        let _ = self.odm.delete(&self.table_name, delete_conditions, Some("mysql_non_cached_db")).await;

        // 插入测试数据到两个数据库
        for user in &self.test_data {
            let data = user.to_data_map();
            // 插入到缓存数据库
            self.odm.create(&self.table_name, data.clone(), Some("mysql_cached_db")).await?;
            // 插入到非缓存数据库
            self.odm.create(&self.table_name, data, Some("mysql_non_cached_db")).await?;
        }

        info!("测试数据准备完成，共插入 {} 条记录", self.test_data.len());
        Ok(())
    }

    /// 测试单条记录查询性能
    async fn test_single_query_performance(&mut self) -> QuickDbResult<()> {
        info!("开始测试单条记录查询性能...");

        let conditions = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::Int(500),
        }];

        let query_options = QueryOptions {
            conditions: vec![],
            sort: vec![],
            pagination: Some(PaginationConfig {
                skip: 0,
                limit: 1,
            }),
            fields: vec![],
        };

        // 测试带缓存的查询
        let start = Instant::now();
        for _ in 0..100 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(query_options.clone()), Some("mysql_cached_db")).await?;
        }
        let cached_time = start.elapsed();

        // 测试不带缓存的查询
        let start = Instant::now();
        for _ in 0..100 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(query_options.clone()), Some("mysql_non_cached_db")).await?;
        }
        let non_cached_time = start.elapsed();

        let result = PerformanceResult::new(
            "单条记录查询 (100次)".to_string(),
            cached_time,
            non_cached_time,
        );
        
        self.results.push(result);
        Ok(())
    }

    /// 测试重复查询性能（缓存命中）
    async fn test_repeat_query_performance(&mut self) -> QuickDbResult<()> {
        info!("开始测试重复查询性能...");

        let conditions = vec![QueryCondition {
            field: "city".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("北京".to_string()),
        }];

        let query_options = QueryOptions {
            conditions: vec![],
            sort: vec![],
            pagination: None,
            fields: vec![],
        };

        // 预热缓存
        let _ = self.odm.find(&self.table_name, conditions.clone(), Some(query_options.clone()), Some("mysql_cached_db")).await?;

        // 测试带缓存的重复查询
        let start = Instant::now();
        for _ in 0..200 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(query_options.clone()), Some("mysql_cached_db")).await?;
        }
        let cached_time = start.elapsed();

        // 测试不带缓存的重复查询
        let start = Instant::now();
        for _ in 0..200 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(query_options.clone()), Some("mysql_non_cached_db")).await?;
        }
        let non_cached_time = start.elapsed();

        let result = PerformanceResult::new(
            "重复查询 (200次)".to_string(),
            cached_time,
            non_cached_time,
        );
        
        self.results.push(result);
        Ok(())
    }

    /// 测试批量查询性能
    async fn test_batch_query_performance(&mut self) -> QuickDbResult<()> {
        info!("开始测试批量查询性能...");

        let conditions = vec![QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(30),
        }];

        let query_options = QueryOptions {
            conditions: vec![],
            sort: vec![],
            pagination: None,
            fields: vec![],
        };

        // 测试带缓存的批量查询
        let start = Instant::now();
        for _ in 0..50 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(query_options.clone()), Some("mysql_cached_db")).await?;
        }
        let cached_time = start.elapsed();

        // 测试不带缓存的批量查询
        let start = Instant::now();
        for _ in 0..50 {
            let _ = self.odm.find(&self.table_name, conditions.clone(), Some(query_options.clone()), Some("mysql_non_cached_db")).await?;
        }
        let non_cached_time = start.elapsed();

        let result = PerformanceResult::new(
            "批量查询 (50次)".to_string(),
            cached_time,
            non_cached_time,
        );
        
        self.results.push(result);
        Ok(())
    }

    /// 显示测试结果
    fn display_results(&self) {
        info!("\n=== MySQL 缓存性能测试结果 ===");
        let mut total_improvement = 0.0;
        
        for result in &self.results {
            info!(
                "操作: {}\n  带缓存: {}ms\n  不带缓存: {}ms\n  性能提升: {:.1}倍\n",
                result.operation,
                result.with_cache.as_millis(),
                result.without_cache.as_millis(),
                result.improvement_ratio
            );
            total_improvement += result.improvement_ratio;
        }
        
        let avg_improvement = total_improvement / self.results.len() as f64;
        info!("平均性能提升: {:.1}倍", avg_improvement);
        info!("=== MySQL 缓存性能测试完成 ===");
    }
}

/// 设置缓存目录
async fn setup_cache_directory() -> QuickDbResult<()> {
    let cache_dir = "./cache/mysql_cache_test";
    if let Err(e) = fs::create_dir_all(cache_dir).await {
        warn!("创建缓存目录失败: {}, 错误: {}", cache_dir, e);
    } else {
        info!("缓存目录创建成功: {}", cache_dir);
    }
    Ok(())
}

/// 清理测试文件
async fn cleanup_test_files() {
    let _ = fs::remove_dir_all("./cache").await;
    info!("清理测试文件完成");
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化日志
    rat_logger::LoggerBuilder::new().add_terminal_with_config(rat_logger::handler::term::TermConfig::default()).init().expect("日志初始化失败");
    
    info!("=== MySQL 缓存性能对比测试开始 ===");

    // 设置缓存目录
    setup_cache_directory().await?;

    // 创建性能测试器
    let mut test = CachePerformanceTest::new().await?;

    info!("数据库配置添加完成");

    // 运行性能测试
    match test.run_all_tests().await {
        Ok(()) => {
            test.display_results();
        }
        Err(e) => {
            error!("性能测试失败: {:?}", e);
        }
    }

    // 清理测试文件
    cleanup_test_files().await;

    Ok(())
}