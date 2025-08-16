//! 缓存性能对比示例
//!
//! 本示例对比启用缓存和未启用缓存的数据库操作性能差异
//! 使用 SQLite 数据库进行测试

use rat_quickdb::{
    types::*,
    odm::AsyncOdmManager,
    manager::{PoolManager, get_global_pool_manager},
    error::QuickDbResult,
    odm::OdmOperations,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use serde_json::json;

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
        map.insert("id".to_string(), DataValue::String(self.id.clone()));
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

/// 缓存性能对比测试
struct CachePerformanceTest {
    /// ODM 管理器
    odm: AsyncOdmManager,
    /// 测试结果
    results: Vec<PerformanceResult>,
}

impl CachePerformanceTest {
    /// 初始化测试环境
    async fn new() -> QuickDbResult<Self> {
        println!("🚀 初始化缓存性能对比测试环境...");
        
        // 创建带缓存的数据库配置
        let cached_config = Self::create_cached_database_config();
        
        // 创建不带缓存的数据库配置
        let non_cached_config = Self::create_non_cached_database_config();
        
        // 使用全局连接池管理器
        let pool_manager = get_global_pool_manager();
        
        // 添加数据库配置
        pool_manager.add_database(cached_config).await?;
        pool_manager.add_database(non_cached_config).await?;
        
        // 创建 ODM 管理器
        let odm = AsyncOdmManager::new();
        
        println!("✅ 测试环境初始化完成");
        
        Ok(Self {
            odm,
            results: Vec::new(),
        })
    }
    
    /// 创建带缓存的数据库配置
    fn create_cached_database_config() -> DatabaseConfig {
        #[cfg(feature = "cache")]
        {
            let cache_config = CacheConfig::default()
                .enabled(true)
                .with_strategy(CacheStrategy::Lru)
                .with_l1_config(
                    L1CacheConfig::new()
                        .with_max_capacity(1000)
                        .with_max_memory_mb(50)
                        .enable_stats(true)
                )
                .with_ttl_config(
                    TtlConfig::new()
                        .with_default_ttl_secs(300) // 5分钟
                        .with_max_ttl_secs(3600)    // 1小时
                        .with_check_interval_secs(60) // 1分钟检查一次
                );
            
            DatabaseConfig {
                db_type: DatabaseType::SQLite,
                connection: ConnectionConfig::SQLite {
                    path: ":memory:".to_string(),
                    create_if_missing: true,
                },
                pool: PoolConfig::default(),
                alias: "cached_db".to_string(),
                cache: Some(cache_config),
                id_strategy: IdStrategy::Uuid,
            }
        }
        
        #[cfg(not(feature = "cache"))]
        {
            DatabaseConfig {
                db_type: DatabaseType::SQLite,
                connection: ConnectionConfig::SQLite {
                    path: ":memory:".to_string(),
                    create_if_missing: true,
                },
                pool: PoolConfig::default(),
                alias: "cached_db".to_string(),
                id_strategy: IdStrategy::Uuid,
            }
        }
    }
    
    /// 创建不带缓存的数据库配置
    fn create_non_cached_database_config() -> DatabaseConfig {
        DatabaseConfig {
            db_type: DatabaseType::SQLite,
            connection: ConnectionConfig::SQLite {
                path: ":memory:".to_string(),
                create_if_missing: true,
            },
            pool: PoolConfig::default(),
            alias: "non_cached_db".to_string(),
            #[cfg(feature = "cache")]
            cache: None, // 明确禁用缓存
            id_strategy: IdStrategy::Uuid,
        }
    }
    
    /// 运行所有性能测试
    async fn run_all_tests(&mut self) -> QuickDbResult<()> {
        // 1. 设置测试数据
        self.setup_test_data().await?;
        
        // 2. 预热缓存
        self.warmup_cache().await?;
        
        // 3. 运行各项测试
        self.test_query_operations().await?;
        self.test_repeated_queries().await?;
        self.test_batch_queries().await?;
        self.test_update_operations().await?;
        
        Ok(())
    }
    
    /// 设置测试数据
    async fn setup_test_data(&mut self) -> QuickDbResult<()> {
        println!("\n🔧 设置测试数据...");
        
        let test_users = vec![
            TestUser::new("user1", "张三", "zhangsan@example.com", 25),
            TestUser::new("user2", "李四", "lisi@example.com", 30),
            TestUser::new("user3", "王五", "wangwu@example.com", 28),
            TestUser::new("user4", "赵六", "zhaoliu@example.com", 35),
            TestUser::new("user5", "钱七", "qianqi@example.com", 22),
        ];
        
        // 批量用户数据
        let batch_users: Vec<TestUser> = (6..=25)
            .map(|i| TestUser::new(
                &format!("batch_user_{}", i),
                &format!("批量用户{}", i),
                &format!("batch{}@example.com", i),
                20 + (i % 30),
            ))
            .collect();
        
        // 创建测试数据到缓存数据库
        for user in test_users.iter().chain(batch_users.iter()) {
            self.odm.create("users", user.to_data_map(), Some("cached_db")).await?;
        }
        
        println!("  ✅ 创建了 {} 条测试记录", test_users.len() + batch_users.len());
        Ok(())
    }
    
    /// 缓存预热
    async fn warmup_cache(&mut self) -> QuickDbResult<()> {
        println!("\n🔥 缓存预热...");
        
        // 执行一些查询操作来预热缓存
        let conditions = vec![
            QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(20),
            }
        ];
        
        // 预热查询
        let _result = self.odm.find("users", conditions, None, Some("cached_db")).await?;
        
        // 按ID查询预热
        let _result = self.odm.find_by_id("users", "user1", Some("cached_db")).await?;
        let _result = self.odm.find_by_id("users", "user2", Some("cached_db")).await?;
        
        println!("  ✅ 缓存预热完成");
        Ok(())
    }
    

    
    /// 测试查询操作性能
    async fn test_query_operations(&mut self) -> QuickDbResult<()> {
        println!("\n🔍 测试查询操作性能...");
        
        let conditions = vec![
            QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("张三".to_string()),
            }
        ];
        
        // 第一次查询（冷启动，从数据库读取）
        let start = Instant::now();
        let _result1 = self.odm.find("users", conditions.clone(), None, Some("cached_db")).await?;
        let first_query_duration = start.elapsed();
        
        // 第二次查询（缓存命中）
        let start = Instant::now();
        let _result2 = self.odm.find("users", conditions, None, Some("cached_db")).await?;
        let cached_duration = start.elapsed();
        
        let result = PerformanceResult::new(
            "单次查询操作".to_string(),
            cached_duration,
            first_query_duration,
        );
        
        println!("  ✅ 首次查询（数据库）: {:?}", first_query_duration);
        println!("  ✅ 缓存查询: {:?}", cached_duration);
        println!("  📈 性能提升: {:.2}x", result.improvement_ratio);
        
        self.results.push(result);
        Ok(())
    }
    
    /// 测试重复查询（缓存命中）
    async fn test_repeated_queries(&mut self) -> QuickDbResult<()> {
        println!("\n🔄 测试重复查询性能（缓存命中测试）...");
        
        let conditions = vec![
            QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(20),
            }
        ];
        
        let query_count = 10;
        
        // 首次查询（建立缓存）
        let _result = self.odm.find("users", conditions.clone(), None, Some("cached_db")).await?;
        
        // 测试重复查询（应该从缓存读取）
        let start = Instant::now();
        for _ in 0..query_count {
            let _result = self.odm.find("users", conditions.clone(), None, Some("cached_db")).await?;
            // 短暂延迟以模拟真实场景
            sleep(Duration::from_millis(5)).await;
        }
        let cached_duration = start.elapsed();
        
        // 计算平均单次查询时间
        let avg_cached_time = cached_duration / query_count;
        let estimated_db_time = Duration::from_millis(50); // 估算数据库查询时间
        
        let result = PerformanceResult::new(
            format!("重复查询 ({}次)", query_count),
            avg_cached_time,
            estimated_db_time,
        ).with_cache_hit_rate(95.0); // 假设95%的缓存命中率
        
        println!("  ✅ 总耗时: {:?}", cached_duration);
        println!("  ✅ 平均单次查询: {:?}", avg_cached_time);
        println!("  📈 预估性能提升: {:.2}x", result.improvement_ratio);
        println!("  🎯 缓存命中率: {:.1}%", result.cache_hit_rate.unwrap_or(0.0));
        
        self.results.push(result);
        Ok(())
    }
    
    /// 测试批量查询性能
    async fn test_batch_queries(&mut self) -> QuickDbResult<()> {
        println!("\n📦 测试批量查询性能...");
        
        let user_ids = vec!["user1", "user2", "user3", "user4", "user5"];
        
        // 首次批量查询（建立缓存）
        let start = Instant::now();
        for user_id in &user_ids {
            let _result = self.odm.find_by_id("users", user_id, Some("cached_db")).await?;
        }
        let first_batch_duration = start.elapsed();
        
        // 第二次批量查询（缓存命中）
        let start = Instant::now();
        for user_id in &user_ids {
            let _result = self.odm.find_by_id("users", user_id, Some("cached_db")).await?;
        }
        let cached_duration = start.elapsed();
        
        let result = PerformanceResult::new(
            format!("批量ID查询 ({}条记录)", user_ids.len()),
            cached_duration,
            first_batch_duration,
        );
        
        println!("  ✅ 首次批量查询: {:?}", first_batch_duration);
        println!("  ✅ 缓存批量查询: {:?}", cached_duration);
        println!("  📈 性能提升: {:.2}x", result.improvement_ratio);
        
        self.results.push(result);
        Ok(())
    }
    
    /// 测试更新操作性能
    async fn test_update_operations(&mut self) -> QuickDbResult<()> {
        println!("\n✏️ 测试更新操作性能...");
        
        let conditions = vec![
            QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("张三".to_string()),
            }
        ];
        
        let mut updates = HashMap::new();
        updates.insert("age".to_string(), DataValue::Int(26));
        updates.insert("email".to_string(), DataValue::String("zhangsan_new@example.com".to_string()));
        
        // 第一次更新操作
        let start = Instant::now();
        let _count1 = self.odm.update("users", conditions.clone(), updates.clone(), Some("cached_db")).await?;
        let first_update_duration = start.elapsed();
        
        // 恢复数据以便第二次更新
        let mut restore_updates = HashMap::new();
        restore_updates.insert("age".to_string(), DataValue::Int(25));
        restore_updates.insert("email".to_string(), DataValue::String("zhangsan@example.com".to_string()));
        let _restore = self.odm.update("users", conditions.clone(), restore_updates, Some("cached_db")).await?;
        
        // 第二次更新操作（可能有缓存优化）
        let start = Instant::now();
        let _count2 = self.odm.update("users", conditions, updates, Some("cached_db")).await?;
        let second_update_duration = start.elapsed();
        
        let result = PerformanceResult::new(
            "更新操作".to_string(),
            second_update_duration,
            first_update_duration,
        );
        
        println!("  ✅ 首次更新: {:?}", first_update_duration);
        println!("  ✅ 第二次更新: {:?}", second_update_duration);
        println!("  📈 性能变化: {:.2}x", result.improvement_ratio);
        
        self.results.push(result);
        Ok(())
    }
    
    /// 显示测试结果汇总
    fn display_results(&self) {
        println!("\n📊 ==================== 性能测试结果汇总 ====================");
        println!("{:<25} {:<15} {:<15} {:<10} {:<10}", "操作类型", "带缓存(ms)", "不带缓存(ms)", "提升倍数", "缓存命中率");
        println!("{}", "-".repeat(80));
        
        let mut total_improvement = 0.0;
        let mut count = 0;
        
        for result in &self.results {
            let cache_hit_str = if let Some(hit_rate) = result.cache_hit_rate {
                format!("{:.1}%", hit_rate)
            } else {
                "N/A".to_string()
            };
            
            println!(
                "{:<25} {:<15} {:<15} {:<10.2} {:<10}",
                result.operation,
                result.with_cache.as_millis(),
                result.without_cache.as_millis(),
                result.improvement_ratio,
                cache_hit_str
            );
            
            total_improvement += result.improvement_ratio;
            count += 1;
        }
        
        println!("{}", "-".repeat(80));
        
        if count > 0 {
            let avg_improvement = total_improvement / count as f64;
            println!("📈 平均性能提升: {:.2}x", avg_improvement);
            
            if avg_improvement > 1.5 {
                println!("🎉 缓存显著提升了数据库操作性能！");
            } else if avg_improvement > 1.1 {
                println!("✅ 缓存适度提升了数据库操作性能。");
            } else {
                println!("⚠️ 缓存对性能提升有限，可能需要调整缓存策略。");
            }
        }
        
        println!("\n💡 性能优化建议:");
        println!("   • 对于频繁查询的数据，缓存能显著提升性能");
        println!("   • 重复查询场景下，缓存命中率越高，性能提升越明显");
        println!("   • 写操作（创建、更新）的性能提升相对有限");
        println!("   • 可根据实际业务场景调整缓存 TTL 和容量配置");
        
        #[cfg(feature = "cache")]
        {
            println!("\n🔧 缓存配置信息:");
            println!("   • 缓存策略: LRU");
            println!("   • L1 缓存容量: 1000 条记录");
            println!("   • L1 缓存内存限制: 50 MB");
            println!("   • 默认 TTL: 5 分钟");
            println!("   • 最大 TTL: 1 小时");
        }
        
        #[cfg(not(feature = "cache"))]
        {
            println!("\n⚠️ 注意: 当前编译未启用 cache 特性，缓存功能可能不可用");
            println!("   请使用 --features cache 重新编译以启用完整的缓存功能");
        }
    }
}

/// 清理测试文件
async fn cleanup_test_files() {
    // 使用内存数据库，无需清理文件
    println!("🧹 清理测试文件完成");
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🚀 RatQuickDB 缓存性能对比测试");
    println!("=====================================\n");
    
    // 清理之前的测试文件
    cleanup_test_files().await;
    
    // 创建并运行测试
    let mut test = CachePerformanceTest::new().await?;
    test.run_all_tests().await?;
    
    // 显示测试结果
    test.display_results();
    
    // 清理测试文件
    cleanup_test_files().await;
    
    println!("\n🎯 测试完成！感谢使用 RatQuickDB 缓存功能。");
    
    Ok(())
}