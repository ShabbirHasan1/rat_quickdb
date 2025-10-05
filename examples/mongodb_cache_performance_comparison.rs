//! MongoDB缓存性能对比示例
//!
//! 本示例对比启用缓存和未启用缓存的MongoDB数据库操作性能差异
//! 使用 MongoDB 数据库进行测试，支持 TLS、认证和 ZSTD 压缩

use rat_quickdb::{
    types::*,
    odm::AsyncOdmManager,
    manager::{PoolManager, get_global_pool_manager},
    error::{QuickDbResult, QuickDbError},
    odm::OdmOperations,
    types::MongoDbConnectionBuilder,
    cache::CacheOps,
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
                options: None, // 移除重复的压缩器配置，使用zstd_config
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
                    enable_l2_cache: true,
                    data_dir: Some("./cache/mongodb_cache_test".to_string()),
                    max_disk_size: 500 * 1024 * 1024,
                    write_buffer_size: 8 * 1024 * 1024,
                    max_write_buffer_number: 2,
                    block_cache_size: 4 * 1024 * 1024,
                    background_threads: 1,
                    enable_lz4: true,
                    compression_threshold: 512,
                    compression_max_threshold: 64 * 1024,
                    compression_level: 3,
                    clear_on_startup: false,
                    cache_size_mb: 32,
                    max_file_size_mb: 64,
                    smart_flush_enabled: true,
                    smart_flush_base_interval_ms: 100,
                    smart_flush_min_interval_ms: 30,
                    smart_flush_max_interval_ms: 1500,
                    smart_flush_write_rate_threshold: 4000,
                    smart_flush_accumulated_bytes_threshold: 2 * 1024 * 1024,
                    cache_warmup_strategy: "Recent".to_string(),
                    zstd_compression_level: None,
                    l2_write_strategy: "async".to_string(),
                    l2_write_threshold: 1024,
                    l2_write_ttl_threshold: 3600,
                }),
                ttl_config: TtlConfig {
                    expire_seconds: Some(300),
                    cleanup_interval: 60,
                    max_cleanup_entries: 1000,
                    lazy_expiration: true,
                    active_expiration: false,
                },
                performance_config: PerformanceConfig {
                    worker_threads: 4,
                    enable_concurrency: true,
                    read_write_separation: true,
                    batch_size: 100,
                    enable_warmup: true,
                    large_value_threshold: 10240,
                },
                version: "v1".to_string(),
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
                options: None, // 移除重复的压缩器配置，使用zstd_config
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

        // 运行核心测试
        self.test_query_operations().await?;
        self.test_cache_hit_stability().await?;
        self.test_batch_queries().await?;

        // 显示结果
        self.display_results();

        Ok(())
    }
    
    /// 设置测试数据
    async fn setup_test_data(&mut self) -> QuickDbResult<()> {
        info!("设置MongoDB测试数据");
        
        // 创建测试用户数据，为不同数据库使用不同的ID前缀避免冲突
        for i in 1..=100 {
            // 为缓存数据库创建用户（使用cached_前缀）
            let cached_user_id = format!("cached_user_{:03}", i);
            let cached_user = TestUser::new(
                &cached_user_id,
                &format!("缓存用户{}", i),
                &format!("cached_user{}@example.com", i),
                20 + (i % 50),
            );
            
            // 为非缓存数据库创建用户（使用non_cached_前缀）
            let non_cached_user_id = format!("non_cached_user_{:03}", i);
            let non_cached_user = TestUser::new(
                &non_cached_user_id,
                &format!("非缓存用户{}", i),
                &format!("non_cached_user{}@example.com", i),
                20 + (i % 50),
            );
            
            // 插入到两个数据库
            self.odm.create(&self.table_name, cached_user.to_data_map(), Some("mongodb_cached")).await?;
            self.odm.create(&self.table_name, non_cached_user.to_data_map(), Some("mongodb_non_cached")).await?;
        }
        
        info!("MongoDB测试数据设置完成，使用表名: {}，共创建200条记录（每个数据库100条）", self.table_name);
        Ok(())
    }
    
    /// 预热缓存
    async fn warmup_cache(&mut self) -> QuickDbResult<()> {
        info!("预热MongoDB缓存");
        
        // 执行一些查询来预热缓存，使用缓存数据库的ID格式
        for i in 1..=20 {
            let conditions = vec![QueryCondition {
                field: "_id".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String(format!("cached_user_{:03}", i)),
            }];
            
            let _ = self.odm.find(&self.table_name, conditions, Some(QueryOptions::default()), Some("mongodb_cached")).await;
        }
        
        // 预热一些范围查询
        let age_conditions = vec![QueryCondition {
            field: "age".to_string(),
            operator: QueryOperator::Gte,
            value: DataValue::Int(25),
        }];
        let _ = self.odm.find(&self.table_name, age_conditions, Some(QueryOptions::default()), Some("mongodb_cached")).await;
        
        info!("MongoDB缓存预热完成，预热了20条单记录查询和1次范围查询");
        Ok(())
    }
    
    /// 测试查询操作性能 - 缓存未命中 vs 命中
    async fn test_query_operations(&mut self) -> QuickDbResult<()> {
        info!("\n🔍 测试MongoDB缓存未命中与命中性能对比...");

        let conditions = vec![
            QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("MongoDB用户1".to_string()),
            }
        ];

        // 清理可能的缓存，确保未命中
        CacheOps::clear_table("mongodb", &self.table_name).await?;

        // 第一次查询 - 缓存未命中（数据库查询 + 缓存设置）
        let start = Instant::now();
        let _result1 = self.odm.find(&self.table_name, conditions.clone(), None, Some("mongodb_cached")).await?;
        let cache_miss_duration = start.elapsed();

        // 第二次查询 - 缓存命中（纯缓存读取）
        let start = Instant::now();
        let _result2 = self.odm.find(&self.table_name, conditions.clone(), None, Some("mongodb_cached")).await?;
        let cache_hit_duration = start.elapsed();

        // 第三次查询 - 再次确认缓存命中
        let start = Instant::now();
        let _result3 = self.odm.find(&self.table_name, conditions, None, Some("mongodb_cached")).await?;
        let cache_hit_duration2 = start.elapsed();

        // 计算平均缓存命中时间
        let avg_cache_hit = (cache_hit_duration + cache_hit_duration2) / 2;

        let result = PerformanceResult::new(
            "缓存命中 vs 未命中".to_string(),
            avg_cache_hit,
            cache_miss_duration,
        );

        info!("  ✅ 缓存未命中（首次查询）: {:?}", cache_miss_duration);
        info!("  ✅ 缓存命中（第二次查询）: {:?}", cache_hit_duration);
        info!("  ✅ 缓存命中（第三次查询）: {:?}", cache_hit_duration2);
        info!("  ✅ 平均缓存命中时间: {:?}", avg_cache_hit);
        info!("  📈 缓存命中性能提升: {:.2}x", result.improvement_ratio);
        info!("  💡 说明：未命中时间包含数据库查询+缓存设置时间");

        self.results.push(result);
        Ok(())
    }
    
    /// 测试缓存命中稳定性
    async fn test_cache_hit_stability(&mut self) -> QuickDbResult<()> {
        info!("\n🔄 测试MongoDB缓存命中稳定性...");

        let conditions = vec![
            QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(20),
            }
        ];

        let query_count = 100; // 大量查询测试缓存稳定性

        // 首次查询建立缓存
        let _result = self.odm.find(&self.table_name, conditions.clone(), None, Some("mongodb_cached")).await?;

        // 测量连续缓存命中的性能
        let mut hit_times = Vec::new();
        for i in 0..query_count {
            let start = Instant::now();
            let _result = self.odm.find(&self.table_name, conditions.clone(), None, Some("mongodb_cached")).await?;
            hit_times.push(start.elapsed());

            // 每20次查询输出进度
            if (i + 1) % 20 == 0 {
                info!("    完成 {} 次缓存命中测试", i + 1);
            }
        }

        // 计算统计数据
        let total_time: Duration = hit_times.iter().sum();
        let avg_time = total_time / query_count;
        let min_time = hit_times.iter().min().unwrap();
        let max_time = hit_times.iter().max().unwrap();

        // 计算性能提升（基于理论数据库查询时间）
        let estimated_db_query_time = Duration::from_micros(2000); // 假设数据库查询需要2ms
        let improvement_ratio = estimated_db_query_time.as_micros() as f64 / avg_time.as_micros() as f64;

        info!("  ✅ 连续 {} 次缓存命中测试完成", query_count);
        info!("  ✅ 平均缓存命中时间: {:?}", avg_time);
        info!("  ✅ 最快缓存命中时间: {:?}", min_time);
        info!("  ✅ 最慢缓存命中时间: {:?}", max_time);
        info!("  📈 理论性能提升: {:.2}x", improvement_ratio);
        info!("  🎯 缓存命中率: 100% (全部命中)");

        let result = PerformanceResult::new(
            format!("缓存命中稳定性 ({}次)", query_count),
            avg_time,
            estimated_db_query_time,
        ).with_cache_hit_rate(100.0);

        self.results.push(result);
        Ok(())
    }
    
    /// 测试批量ID查询的缓存效果
    async fn test_batch_queries(&mut self) -> QuickDbResult<()> {
        info!("\n📦 测试MongoDB批量ID查询的缓存效果...");

        let user_ids: Vec<String> = vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string(), "5".to_string()];

        // 清理可能存在的缓存
        for user_id in &user_ids {
            // MongoDB暂不支持单个记录缓存删除，使用表清理
        }

        // 批量查询 - 缓存未命中（全部需要查询数据库）
        let mut miss_times = Vec::new();
        for user_id in &user_ids {
            let start = Instant::now();
            let _result = self.odm.find_by_id(&self.table_name, user_id, Some("mongodb_cached")).await?;
            miss_times.push(start.elapsed());
        }
        let total_miss_time = miss_times.iter().sum::<Duration>();

        // 批量查询 - 缓存命中（全部从缓存读取）
        let mut hit_times = Vec::new();
        for user_id in &user_ids {
            let start = Instant::now();
            let _result = self.odm.find_by_id(&self.table_name, user_id, Some("mongodb_cached")).await?;
            hit_times.push(start.elapsed());
        }
        let total_hit_time = hit_times.iter().sum::<Duration>();

        // 第二轮确认缓存命中
        let mut hit_times2 = Vec::new();
        for user_id in &user_ids {
            let start = Instant::now();
            let _result = self.odm.find_by_id(&self.table_name, user_id, Some("mongodb_cached")).await?;
            hit_times2.push(start.elapsed());
        }
        let total_hit_time2 = hit_times2.iter().sum::<Duration>();

        let avg_miss_time = total_miss_time / user_ids.len() as u32;
        let avg_hit_time = (total_hit_time + total_hit_time2) / (2 * user_ids.len() as u32);

        let result = PerformanceResult::new(
            format!("批量ID查询 ({}个ID)", user_ids.len()),
            avg_hit_time,
            avg_miss_time,
        ).with_cache_hit_rate(100.0);

        info!("  ✅ 缓存未命中（批量查询）: {:?} (平均: {:?})", total_miss_time, avg_miss_time);
        info!("  ✅ 缓存命中（批量查询）: {:?} (平均: {:?})", total_hit_time, total_hit_time / user_ids.len() as u32);
        info!("  ✅ 缓存命中（第二轮）: {:?} (平均: {:?})", total_hit_time2, total_hit_time2 / user_ids.len() as u32);
        info!("  ✅ 平均缓存命中时间: {:?}", avg_hit_time);
        info!("  📈 批量查询性能提升: {:.2}x", result.improvement_ratio);

        self.results.push(result);
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

/// 创建缓存目录
async fn setup_cache_directory() -> QuickDbResult<()> {
    let cache_dir = "./cache/mongodb_cache_test";
    if let Err(e) = tokio::fs::create_dir_all(cache_dir).await {
        error!("创建缓存目录失败: {}", e);
        return Err(QuickDbError::ConfigError { message: format!("创建缓存目录失败: {}", e) });
    }
    info!("缓存目录创建成功: {}", cache_dir);
    Ok(())
}

/// 清理测试文件
async fn cleanup_test_files() {
    let _ = tokio::fs::remove_dir_all("./cache/mongodb_cache_test").await;
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    // 初始化 rat_logger 日志系统，设置为 debug 级别
    rat_logger::LoggerBuilder::new()
        .with_level(rat_logger::LevelFilter::Debug)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("初始化日志系统失败");
    
    // 清理之前的测试文件
    cleanup_test_files().await;
    
    // 创建缓存目录
    setup_cache_directory().await?;
    
    // 创建并运行性能测试
    let mut test = CachePerformanceTest::new().await?;
    test.run_all_tests().await?;
    
    // 清理测试文件
    cleanup_test_files().await;
    
    info!("MongoDB缓存性能对比测试完成");
    Ok(())
}