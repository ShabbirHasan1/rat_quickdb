//! 缓存操作快捷方法
//!
//! 提供数据库适配器使用的内部缓存操作方法
//! 缓存原样进出，不进行序列化/反序列化

use crate::types::{
    IdType, QueryOptions, QueryCondition,
    QueryConditionGroup, LogicalOperator, DataValue
};
use crate::cache_singleton::GlobalCacheManager;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use anyhow::Result;
use rat_logger::{debug, warn};

/// 缓存操作快捷方法（供数据库适配器内部使用）
pub struct CacheOps;

impl CacheOps {
    /// 检查全局缓存管理器是否已初始化
    pub fn is_initialized() -> bool {
        crate::cache_singleton::GlobalCacheManager::is_initialized()
    }
    /// 缓存空值标记（用于区分真正的空值和缓存未命中）
    const NULL_MARKER: &'static [u8] = b"CACHE_NULL_VALUE_2025";
    /// 生成记录缓存键
    ///
    /// 格式：{db_type}:{table}:{operation}:{id}
    pub(crate) fn generate_record_key(db_type: &str, table: &str, id: &IdType) -> String {
        let id_str = match id {
            IdType::Number(n) => n.to_string(),
            IdType::String(s) => s.clone(),
        };
        format!("{}:{}:record:{}", db_type, table, id_str)
    }

    /// 生成查询缓存键
    ///
    /// 格式：{db_type}:{table}:query:{hash}
    pub(crate) fn generate_query_key(db_type: &str, table: &str, query_hash: &str) -> String {
        format!("{}:{}:query:{}", db_type, table, query_hash)
    }

    /// 生成条件组合查询的哈希值
    pub(crate) fn hash_condition_groups(condition_groups: &[QueryConditionGroup], options: &QueryOptions) -> String {
        let mut hasher = DefaultHasher::new();

        // 对条件组合进行哈希
        for group in condition_groups {
            group.hash(&mut hasher);
        }

        // 对查询选项进行哈希
        options.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// 生成简单查询的哈希值
    pub(crate) fn hash_simple_query(conditions: &[QueryCondition], options: &QueryOptions) -> String {
        let mut hasher = DefaultHasher::new();

        // 对条件进行哈希
        for condition in conditions {
            condition.hash(&mut hasher);
        }

        // 对查询选项进行哈希
        options.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// 通用缓存获取方法（获取Vec<DataValue>）
    ///
    /// # 返回
    /// * `(true, Some(data))` - 缓存命中，返回Vec<DataValue>
    /// * `(true, None)` - 缓存命中，但数据为空值（有效空结果，空Vec）
    /// * `(false, None)` - 缓存未命中
    pub(crate) async fn get(key: &str) -> Result<(bool, Option<Vec<DataValue>>)> {
        let cache_manager = GlobalCacheManager::get().unwrap();

        debug!("获取缓存: {}", key);

        match cache_manager.get(key).await {
            Ok((true, Some(data))) => {
                // 缓存命中，检查是否为空值标记
                if data == Self::NULL_MARKER {
                    Ok((true, None)) // 真正的空值（空Vec）
                } else {
                    // 将字节数据反序列化为Vec<DataValue>
                    let vec_data = DataValue::vec_from_bytes(&data);
                    Ok((true, Some(vec_data))) // 正常Vec<DataValue>数据
                }
            },
            Ok((true, None)) => Ok((true, None)), // 直接的None值
            Ok((false, None)) => Ok((false, None)), // 缓存未命中
            Ok((false, Some(_))) => {
                // 这种情况理论上不应该出现，打印警告
                warn!("缓存返回了意外的状态：未命中但有数据，键: {}", key);
                Ok((false, None))
            },
            Err(e) => Err(e),
        }
    }

    /// 通用缓存设置方法（设置Vec<DataValue>）
    pub(crate) async fn set(key: &str, data: Option<Vec<DataValue>>, ttl: Option<u64>) -> Result<()> {
        let cache_manager = GlobalCacheManager::get().unwrap();

        debug!("设置缓存: {}", key);

        let cache_data = match data {
            Some(vec_data) => {
                // 将Vec<DataValue>转换为字节
                crate::types::DataValue::vec_to_bytes(&vec_data)
            },
            None => Self::NULL_MARKER.to_vec(), // 空值标记
        };

        cache_manager.set_with_ttl(key, cache_data, ttl).await
    }

    /// 删除记录缓存
    pub async fn delete_record(db_type: &str, table: &str, id: &IdType) -> Result<()> {
        let cache_manager = GlobalCacheManager::get().unwrap();
        let cache_key = Self::generate_record_key(db_type, table, id);

        debug!("删除记录缓存: {}", cache_key);
        cache_manager.delete(&cache_key).await
    }

    /// 清理表相关的所有缓存
    pub async fn clear_table(db_type: &str, table: &str) -> Result<()> {
        let cache_manager = GlobalCacheManager::get().unwrap();
        let prefix = format!("{}:{}:", db_type, table);

        debug!("清理表缓存: {}", prefix);
        cache_manager.clear_by_prefix(&prefix).await
    }

    /// 清理数据库相关的所有缓存
    pub async fn clear_database(db_type: &str) -> Result<()> {
        let cache_manager = GlobalCacheManager::get().unwrap();
        let prefix = format!("{}:", db_type);

        debug!("清理数据库缓存: {}", prefix);
        cache_manager.clear_by_prefix(&prefix).await
    }

    /// 清理所有查询缓存（保留记录缓存）
    pub async fn clear_all_queries() -> Result<()> {
        let cache_manager = GlobalCacheManager::get().unwrap();

        debug!("清理所有查询缓存");
        cache_manager.clear_by_prefix(":query:").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[tokio::test]
    async fn test_cache_initialization_and_basic_operations() {
        println!("🚀 开始测试缓存初始化和基础操作");

        // 创建测试配置
        let l2_config = Some(L2CacheConfig::new(Some("/tmp/test_cache".to_string())));

        let cache_config = CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            version: "test".to_string(),
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: false,
            },
            l2_config,
            ttl_config: TtlConfig::new(),
            compression_config: CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::Lz4,
                threshold_bytes: 1024,
            },
            performance_config: PerformanceConfig::new(),
        };

        // 测试缓存初始化
        if !crate::cache_singleton::GlobalCacheManager::is_initialized() {
            println!("📦 初始化全局缓存管理器");
            crate::cache_singleton::GlobalCacheManager::initialize(cache_config).await.unwrap();
            println!("✅ 缓存管理器初始化成功");
        }

        // 测试基础记录缓存操作
        let db_type = "sqlite";
        let table = "users";
        let id = IdType::Number(123);
        let test_data = "test_user".as_bytes().to_vec();

        println!("💾 测试设置记录缓存");
        CacheOps::set_record(db_type, table, &id, test_data.clone(), Some(3600)).await.unwrap();
        println!("✅ 记录缓存设置成功");

        println!("🔍 测试获取记录缓存");
        let (cache_hit, cached_data) = CacheOps::get_record(db_type, table, &id).await.unwrap();

        assert!(cache_hit, "❌ 缓存应该命中");
        assert!(cached_data.is_some(), "❌ 缓存数据应该存在");

        let cached_str = String::from_utf8(cached_data.unwrap()).unwrap();
        assert_eq!(cached_str, "test_user");
        println!("✅ 记录缓存获取成功: {}", cached_str);

        println!("🗑️ 测试删除记录缓存");
        let record_key = format!("{}:{}:record:{}", db_type, table, match &id {
            IdType::Number(n) => n.to_string(),
            IdType::String(s) => s.clone(),
        });
        println!("🔍 删除的记录键: {}", record_key);
        CacheOps::delete_record(db_type, table, &id).await.unwrap();
        println!("✅ 记录缓存删除成功");

        // 验证删除后无法获取
        let (cache_hit_after_delete, cached_data_after_delete) = CacheOps::get_record(db_type, table, &id).await.unwrap();
        println!("🔍 删除后 - 缓存命中: {}, 数据存在: {}", cache_hit_after_delete, cached_data_after_delete.is_some());
        assert!(!cache_hit_after_delete, "❌ 缓存应该未命中");
        assert!(cached_data_after_delete.is_none(), "❌ 缓存数据应该为None");
        println!("✅ 记录缓存删除验证成功");

        println!("🎉 所有基础缓存操作测试通过！");
    }

    #[tokio::test]
    async fn test_query_cache_operations() {
        println!("🚀 开始测试查询缓存操作");

        // 使用相同的配置初始化
        let l2_config = Some(L2CacheConfig::new(Some("/tmp/test_cache".to_string())));

        let cache_config = CacheConfig {
            enabled: true,
            strategy: CacheStrategy::Lru,
            version: "test".to_string(),
            l1_config: L1CacheConfig {
                max_capacity: 1000,
                max_memory_mb: 100,
                enable_stats: false,
            },
            l2_config,
            ttl_config: TtlConfig::new(),
            compression_config: CompressionConfig {
                enabled: false,
                algorithm: CompressionAlgorithm::Lz4,
                threshold_bytes: 1024,
            },
            performance_config: PerformanceConfig::new(),
        };

        if !crate::cache_singleton::GlobalCacheManager::is_initialized() {
            crate::cache_singleton::GlobalCacheManager::initialize(cache_config).await.unwrap();
        }

        let db_type = "sqlite";
        let table = "users";
        let condition = QueryCondition {
            field: "name".to_string(),
            operator: QueryOperator::Eq,
            value: DataValue::String("Alice".to_string()),
        };
        let options = QueryOptions::default();

        let query_data = "[{\"id\":1,\"name\":\"Alice\"}]".as_bytes().to_vec();

        println!("💾 测试设置查询缓存");
        CacheOps::set_simple_query(db_type, table, &[condition.clone()], &options, query_data.clone(), Some(3600)).await.unwrap();
        println!("✅ 查询缓存设置成功");

        println!("🔍 测试获取查询缓存");
        let (cache_hit, cached_data) = CacheOps::get_simple_query(db_type, table, &[condition.clone()], &options).await.unwrap();

        assert!(cache_hit, "❌ 查询缓存应该命中");
        assert!(cached_data.is_some(), "❌ 查询缓存数据应该存在");

        let cached_str = String::from_utf8(cached_data.unwrap()).unwrap();
        assert_eq!(cached_str, "[{\"id\":1,\"name\":\"Alice\"}]");
        println!("✅ 查询缓存获取成功: {}", cached_str);

        println!("🗑️ 测试清理表缓存");
        CacheOps::clear_table(db_type, table).await.unwrap();
        println!("✅ 表缓存清理成功");

        // 验证清理后无法获取
        let (cache_hit, cached_data) = CacheOps::get_simple_query(db_type, table, &[condition.clone()], &options).await.unwrap();
        assert!(!cache_hit, "❌ 查询缓存应该未命中");
        assert!(cached_data.is_none(), "❌ 查询缓存数据应该为None");
        println!("✅ 查询缓存清理验证成功");

        println!("🎉 查询缓存操作测试通过！");
    }
}