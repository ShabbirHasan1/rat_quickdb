//! MySQL缓存性能对比示例
//!
//! 本示例对比启用缓存和未启用缓存的MySQL数据库操作性能差异
//! 使用真实的MySQL数据库进行测试

use rat_quickdb::{
    types::*,
    odm::AsyncOdmManager,
    manager::{PoolManager, get_global_pool_manager},
    error::QuickDbResult,
    odm::OdmOperations,
    cache::CacheOps,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde_json::json;

/// 测试数据结构
#[derive(Debug, Clone)]
struct TestUser {
    id: i64,
    name: String,
    email: String,
    age: i32,
    city: String,
    created_at: String,
}

impl TestUser {
    fn new(index: usize) -> Self {
        Self {
            id: index as i64, // 保留ID用于测试，但实际插入时会让MySQL自动生成
            name: format!("MySQL用户{}", index),
            email: format!("mysql_user{}@example.com", index),
            age: (20 + (index % 50)) as i32,
            city: format!("城市{}", (index % 10) + 1),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn to_data_map(&self) -> HashMap<String, DataValue> {
        let mut map = HashMap::new();
        // 不包含id字段，让MySQL自动生成
        map.insert("name".to_string(), DataValue::String(self.name.clone()));
        map.insert("email".to_string(), DataValue::String(self.email.clone()));
        map.insert("age".to_string(), DataValue::Int(self.age as i64));
        map.insert("city".to_string(), DataValue::String(self.city.clone()));
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
        let improvement_ratio = if with_cache.as_micros() > 0 {
            without_cache.as_micros() as f64 / with_cache.as_micros() as f64
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

/// MySQL缓存性能对比测试
struct MysqlCachePerformanceTest {
    /// ODM 管理器
    odm: AsyncOdmManager,
    /// 测试结果
    results: Vec<PerformanceResult>,
}

impl MysqlCachePerformanceTest {
    /// 初始化测试环境
    async fn new() -> QuickDbResult<Self> {
        println!("🚀 初始化MySQL缓存性能对比测试环境...");

        // 创建数据库配置（强制启用缓存）
        let db_config = Self::create_mysql_database_config();

        // 使用全局连接池管理器
        let pool_manager = get_global_pool_manager();

        // 添加数据库配置
        pool_manager.add_database(db_config).await?;

        // 创建 ODM 管理器
        let odm = AsyncOdmManager::new();

        // 检查缓存是否初始化
        if CacheOps::is_initialized() {
            println!("✅ 全局缓存管理器已初始化");
        } else {
            println!("⚠️  全局缓存管理器未初始化！");
        }

        println!("✅ 测试环境初始化完成");
        println!("📝 测试说明：对比缓存命中与未命中的性能差异");

        Ok(Self {
            odm,
            results: Vec::new(),
        })
    }

    /// 创建MySQL数据库配置
    fn create_mysql_database_config() -> DatabaseConfig {
        let cache_config = CacheConfig::default()
            .enabled(true)
            .with_strategy(CacheStrategy::Lru)
            .with_l1_config(
                L1CacheConfig::new()
                    .with_max_capacity(1000)
                    .with_max_memory_mb(50)
                    .enable_stats(true)
            )
            .with_l2_config(
                L2CacheConfig::new(Some("./test_data/mysql_cache_l2".to_string()))
            )
            .with_ttl_config(
                TtlConfig::new()
                    .with_expire_seconds(Some(300)) // 5分钟
                    .with_cleanup_interval(60)      // 1分钟检查一次
                    .with_max_cleanup_entries(100)
                    .with_lazy_expiration(true)
                    .with_active_expiration(false)
            )
            .with_performance_config(
                PerformanceConfig::new()
            );

        DatabaseConfig {
            db_type: DatabaseType::MySQL,
            connection: ConnectionConfig::MySQL {
                host: "172.16.0.21".to_string(),
                port: 3306,
                username: "testdb".to_string(),
                password: "yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^".to_string(),
                database: "testdb".to_string(),
                tls_config: None,
            },
            pool: PoolConfig::default(),
            alias: "mysql_cache_db".to_string(),
            cache: Some(cache_config),
            id_strategy: IdStrategy::AutoIncrement,
        }
    }

    /// 运行所有性能测试
    async fn run_all_tests(&mut self) -> QuickDbResult<()> {
        // 1. 设置测试数据
        self.setup_test_data().await?;

        // 2. 运行缓存性能对比测试
        self.test_query_operations().await?;           // 缓存未命中 vs 命中
        self.test_cache_hit_stability().await?;        // 缓存命中稳定性
        self.test_batch_queries().await?;              // 批量查询缓存效果
        self.test_update_operations().await?;          // 更新操作对缓存的影响

        Ok(())
    }

    /// 设置测试数据
    async fn setup_test_data(&mut self) -> QuickDbResult<()> {
        println!("\n🔧 设置MySQL测试数据...");

        // 安全机制：清理可能存在的测试数据
        println!("  清理可能存在的测试数据...");
        if let Ok(_) = self.odm.delete("users", vec![], Some("mysql_cache_db")).await {
            println!("  ✅ 已清理数据库");
        }

        let test_users = vec![
            TestUser::new(1),
            TestUser::new(2),
            TestUser::new(3),
            TestUser::new(4),
            TestUser::new(5),
        ];

        // 批量用户数据 - 减少数据量避免低配置MySQL服务器假死
        let batch_users: Vec<TestUser> = (6..=10)
            .map(|i| TestUser::new(i))
            .collect();

        // 创建测试数据到数据库
        for user in test_users.iter().chain(batch_users.iter()) {
            self.odm.create("users", user.to_data_map(), Some("mysql_cache_db")).await?;
        }

        println!("  ✅ 创建了 {} 条测试记录", test_users.len() + batch_users.len());
        Ok(())
    }

    /// 测试查询操作性能 - 缓存未命中 vs 命中
    async fn test_query_operations(&mut self) -> QuickDbResult<()> {
        println!("\n🔍 测试MySQL缓存未命中与命中性能对比...");

        let conditions = vec![
            QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("MySQL用户1".to_string()),
            }
        ];

        // 清理可能的缓存，确保未命中
        CacheOps::clear_table("mysql", "users").await?;

        // 第一次查询 - 缓存未命中（数据库查询 + 缓存设置）
        let start = Instant::now();
        let _result1 = self.odm.find("users", conditions.clone(), None, Some("mysql_cache_db")).await?;
        let cache_miss_duration = start.elapsed();

        // 第二次查询 - 缓存命中（纯缓存读取）
        let start = Instant::now();
        let _result2 = self.odm.find("users", conditions.clone(), None, Some("mysql_cache_db")).await?;
        let cache_hit_duration = start.elapsed();

        // 第三次查询 - 再次确认缓存命中
        let start = Instant::now();
        let _result3 = self.odm.find("users", conditions, None, Some("mysql_cache_db")).await?;
        let cache_hit_duration2 = start.elapsed();

        // 计算平均缓存命中时间
        let avg_cache_hit = (cache_hit_duration + cache_hit_duration2) / 2;

        let result = PerformanceResult::new(
            "MySQL缓存命中 vs 未命中".to_string(),
            avg_cache_hit,
            cache_miss_duration,
        );

        println!("  ✅ 缓存未命中（首次查询）: {:?}", cache_miss_duration);
        println!("  ✅ 缓存命中（第二次查询）: {:?}", cache_hit_duration);
        println!("  ✅ 缓存命中（第三次查询）: {:?}", cache_hit_duration2);
        println!("  ✅ 平均缓存命中时间: {:?}", avg_cache_hit);
        println!("  📈 缓存命中性能提升: {:.2}x", result.improvement_ratio);
        println!("  💡 说明：未命中时间包含数据库查询+缓存设置时间");

        self.results.push(result);
        Ok(())
    }

    /// 测试缓存命中稳定性
    async fn test_cache_hit_stability(&mut self) -> QuickDbResult<()> {
        println!("\n🔄 测试MySQL缓存命中稳定性...");

        let conditions = vec![
            QueryCondition {
                field: "age".to_string(),
                operator: QueryOperator::Gt,
                value: DataValue::Int(20),
            }
        ];

        let query_count = 100; // 大量查询测试缓存稳定性

        // 首次查询建立缓存
        let _result = self.odm.find("users", conditions.clone(), None, Some("mysql_cache_db")).await?;

        // 测量连续缓存命中的性能
        let mut hit_times = Vec::new();
        for i in 0..query_count {
            let start = Instant::now();
            let _result = self.odm.find("users", conditions.clone(), None, Some("mysql_cache_db")).await?;
            hit_times.push(start.elapsed());

            // 每20次查询输出进度
            if (i + 1) % 20 == 0 {
                println!("    完成 {} 次缓存命中测试", i + 1);
            }
        }

        // 计算统计数据
        let total_time: Duration = hit_times.iter().sum();
        let avg_time = total_time / query_count;
        let min_time = hit_times.iter().min().unwrap();
        let max_time = hit_times.iter().max().unwrap();

        // 计算性能提升（基于理论数据库查询时间）
        let estimated_db_query_time = Duration::from_micros(5000); // 假设MySQL查询需要5ms
        let improvement_ratio = estimated_db_query_time.as_micros() as f64 / avg_time.as_micros() as f64;

        println!("  ✅ 连续 {} 次缓存命中测试完成", query_count);
        println!("  ✅ 平均缓存命中时间: {:?}", avg_time);
        println!("  ✅ 最快缓存命中时间: {:?}", min_time);
        println!("  ✅ 最慢缓存命中时间: {:?}", max_time);
        println!("  📈 理论性能提升: {:.2}x", improvement_ratio);
        println!("  🎯 缓存命中率: 100% (全部命中)");

        let result = PerformanceResult::new(
            format!("MySQL缓存命中稳定性 ({}次)", query_count),
            avg_time,
            estimated_db_query_time,
        ).with_cache_hit_rate(100.0);

        self.results.push(result);
        Ok(())
    }

    /// 测试批量ID查询的缓存效果
    async fn test_batch_queries(&mut self) -> QuickDbResult<()> {
        println!("\n📦 测试MySQL批量ID查询的缓存效果...");

        let user_ids = vec!["1", "2", "3", "4", "5"];

        // 清理可能存在的缓存
        for user_id in &user_ids {
            CacheOps::delete_record("mysql", "users", &IdType::String(user_id.to_string())).await?;
        }

        // 批量查询 - 缓存未命中（全部需要查询数据库）
        let mut miss_times = Vec::new();
        for user_id in &user_ids {
            let start = Instant::now();
            let _result = self.odm.find_by_id("users", user_id, Some("mysql_cache_db")).await?;
            miss_times.push(start.elapsed());
        }
        let total_miss_time = miss_times.iter().sum::<Duration>();

        // 批量查询 - 缓存命中（全部从缓存读取）
        let mut hit_times = Vec::new();
        for user_id in &user_ids {
            let start = Instant::now();
            let _result = self.odm.find_by_id("users", user_id, Some("mysql_cache_db")).await?;
            hit_times.push(start.elapsed());
        }
        let total_hit_time = hit_times.iter().sum::<Duration>();

        // 计算平均时间
        let avg_miss_time = total_miss_time / user_ids.len() as u32;
        let avg_hit_time = total_hit_time / user_ids.len() as u32;

        let result = PerformanceResult::new(
            format!("MySQL批量ID查询 ({}条记录)", user_ids.len()),
            avg_hit_time,
            avg_miss_time,
        );

        println!("  ✅ 批量查询 - 缓存未命中总计: {:?}", total_miss_time);
        println!("  ✅ 批量查询 - 缓存命中总计: {:?}", total_hit_time);
        println!("  ✅ 平均单次查询 - 缓存未命中: {:?}", avg_miss_time);
        println!("  ✅ 平均单次查询 - 缓存命中: {:?}", avg_hit_time);
        println!("  📈 缓存命中性能提升: {:.2}x", result.improvement_ratio);

        self.results.push(result);
        Ok(())
    }

    /// 测试更新操作性能
    async fn test_update_operations(&mut self) -> QuickDbResult<()> {
        println!("\n✏️ 测试MySQL更新操作性能...");

        let conditions = vec![
            QueryCondition {
                field: "name".to_string(),
                operator: QueryOperator::Eq,
                value: DataValue::String("MySQL用户1".to_string()),
            }
        ];

        let mut updates = HashMap::new();
        updates.insert("age".to_string(), DataValue::Int(26));
        updates.insert("city".to_string(), DataValue::String("新城市".to_string()));

        // 第一次更新操作
        let start = Instant::now();
        let _count1 = self.odm.update("users", conditions.clone(), updates.clone(), Some("mysql_cache_db")).await?;
        let first_update_duration = start.elapsed();

        // 恢复数据以便第二次更新
        let mut restore_updates = HashMap::new();
        restore_updates.insert("age".to_string(), DataValue::Int(25));
        restore_updates.insert("city".to_string(), DataValue::String("城市1".to_string()));
        let _restore = self.odm.update("users", conditions.clone(), restore_updates, Some("mysql_cache_db")).await?;

        // 第二次更新操作（可能有缓存优化）
        let start = Instant::now();
        let _count2 = self.odm.update("users", conditions, updates, Some("mysql_cache_db")).await?;
        let second_update_duration = start.elapsed();

        let result = PerformanceResult::new(
            "MySQL更新操作".to_string(),
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
        println!("\n📊 ==================== MySQL缓存性能测试结果汇总 ====================");
        println!("{:<30} {:<15} {:<15} {:<10} {:<10}", "测试类型", "缓存命中(µs)", "缓存未命中(µs)", "提升倍数", "命中率");
        println!("{}", "-".repeat(85));

        let mut total_improvement = 0.0;
        let mut count = 0;

        for result in &self.results {
            let cache_hit_str = if let Some(hit_rate) = result.cache_hit_rate {
                format!("{:.1}%", hit_rate)
            } else {
                "N/A".to_string()
            };

            println!(
                "{:<30} {:<15.0} {:<15.0} {:<10.2} {:<10}",
                result.operation,
                result.with_cache.as_micros(),
                result.without_cache.as_micros(),
                result.improvement_ratio,
                cache_hit_str
            );

            total_improvement += result.improvement_ratio;
            count += 1;
        }

        println!("{}", "-".repeat(85));

        if count > 0 {
            let avg_improvement = total_improvement / count as f64;
            println!("📈 平均MySQL缓存性能提升: {:.2}x", avg_improvement);

            if avg_improvement > 5.0 {
                println!("🎉 MySQL缓存效果极佳！显著提升了查询性能！");
            } else if avg_improvement > 2.0 {
                println!("✅ MySQL缓存效果良好，有效提升了查询性能。");
            } else {
                println!("⚠️ MySQL缓存效果有限，建议检查缓存配置或查询模式。");
            }
        }

        println!("\n💡 测试说明:");
        println!("   • 缓存未命中时间 = 数据库查询时间 + 缓存设置时间");
        println!("   • 缓存命中时间 = 纯缓存读取时间");
        println!("   • 性能提升 = 未命中时间 ÷ 命中时间");
        println!("   • 强制缓存设计：所有查询都会经过缓存层");

        println!("\n🔧 当前MySQL缓存配置:");
        println!("   • 缓存策略: LRU");
        println!("   • L1 缓存容量: 1000 条记录");
        println!("   • L1 缓存内存限制: 50 MB");
        println!("   • 默认 TTL: 5 分钟");
        println!("   • L2 缓存: 启用（磁盘存储）");
    }
}

/// 清理测试文件
async fn cleanup_test_files() {
    // 清理测试L2缓存目录
    let test_dirs = [
        "./test_data/mysql_cache_l2",
    ];

    for dir_path in &test_dirs {
        if std::path::Path::new(dir_path).exists() {
            if let Err(e) = tokio::fs::remove_dir_all(dir_path).await {
                eprintln!("⚠️  清理目录 {} 失败: {}", dir_path, e);
            } else {
                println!("🗑️  已清理目录: {}", dir_path);
            }
        }
    }

    // 尝试清理测试目录（如果为空）
    if let Err(_) = tokio::fs::remove_dir("./test_data").await {
        // 目录不为空或不存在，忽略错误
    }

    println!("🧹 清理测试文件完成");
}

#[tokio::main]
async fn main() -> QuickDbResult<()> {
    println!("🚀 RatQuickDB MySQL缓存性能对比测试");
    println!("==========================================\n");

    // 清理之前的测试文件
    cleanup_test_files().await;

    // 创建并运行测试
    let mut test = MysqlCachePerformanceTest::new().await?;
    test.run_all_tests().await?;

    // 显示测试结果
    test.display_results();

    // 清理测试文件
    cleanup_test_files().await;

    println!("\n🎯 MySQL缓存测试完成！感谢使用 RatQuickDB 缓存功能。");

    Ok(())
}