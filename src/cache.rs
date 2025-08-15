//! 缓存管理模块
//! 
//! 提供基于 rat_memcache 的自动缓存功能，支持 LRU 策略、自动更新/清理缓存
//! 以及可选的手动清理接口。

#[cfg(feature = "cache")]
use crate::types::{
    CacheConfig, CacheStrategy, CompressionAlgorithm, DataValue, IdType, QueryOptions,
};
#[cfg(feature = "cache")]
use anyhow::{anyhow, Result};
#[cfg(feature = "cache")]
use rat_memcache::{
    cache::{RatMemCache, RatMemCacheBuilder},
    config::{CacheOptions, CompressionType, EvictionPolicy},
};
#[cfg(feature = "cache")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "cache")]
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[cfg(feature = "cache")]
use tokio::sync::RwLock;
#[cfg(feature = "cache")]
use zerg_creep::prelude::*;

/// 缓存键前缀
#[cfg(feature = "cache")]
const CACHE_KEY_PREFIX: &str = "rat_quickdb";

/// 缓存管理器
#[cfg(feature = "cache")]
#[derive(Clone)]
pub struct CacheManager {
    /// 内部缓存实例
    cache: Arc<RatMemCache>,
    /// 缓存配置
    config: CacheConfig,
    /// 表名到缓存键的映射
    table_keys: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

#[cfg(feature = "cache")]
impl CacheManager {
    /// 创建新的缓存管理器
    pub async fn new(config: CacheConfig) -> Result<Self> {
        let mut builder = RatMemCacheBuilder::new();

        // 配置 L1 缓存
        builder = builder
            .l1_capacity(config.l1_config.max_capacity)
            .l1_memory_limit_mb(config.l1_config.max_memory_mb)
            .enable_l1_stats(config.l1_config.enable_stats);

        // 配置 L2 缓存（如果启用）
        if let Some(l2_config) = &config.l2_config {
            builder = builder
                .enable_l2(&l2_config.storage_path)
                .l2_disk_limit_mb(l2_config.max_disk_mb)
                .l2_compression_level(l2_config.compression_level)
                .l2_enable_wal(l2_config.enable_wal);
        }

        // 配置驱逐策略
        let eviction_policy = match config.strategy {
            CacheStrategy::Lru => EvictionPolicy::Lru,
            CacheStrategy::Lfu => EvictionPolicy::Lfu,
            CacheStrategy::Fifo => EvictionPolicy::Fifo,
            CacheStrategy::Custom(_) => EvictionPolicy::Lru, // 默认使用 LRU
        };
        builder = builder.eviction_policy(eviction_policy);

        // 配置 TTL
        builder = builder
            .default_ttl(Duration::from_secs(config.ttl_config.default_ttl_secs))
            .max_ttl(Duration::from_secs(config.ttl_config.max_ttl_secs))
            .ttl_check_interval(Duration::from_secs(config.ttl_config.check_interval_secs));

        // 配置压缩
        if config.compression_config.enabled {
            let compression_type = match config.compression_config.algorithm {
                CompressionAlgorithm::Zstd => CompressionType::Zstd,
                CompressionAlgorithm::Lz4 => CompressionType::Lz4,
                CompressionAlgorithm::Gzip => CompressionType::Gzip,
            };
            builder = builder
                .enable_compression(compression_type)
                .compression_threshold(config.compression_config.threshold_bytes);
        }

        // 构建缓存实例
        let cache = builder.build().await.map_err(|e| {
            anyhow!("Failed to create cache instance: {}", e)
        })?;

        info!("缓存管理器初始化成功");

        Ok(Self {
            cache: Arc::new(cache),
            config,
            table_keys: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 生成缓存键
    fn generate_cache_key(&self, table: &str, id: &IdType, operation: &str) -> String {
        let id_str = match id {
            IdType::Number(n) => n.to_string(),
            IdType::String(s) => s.clone(),
        };
        format!("{}:{}:{}:{}", CACHE_KEY_PREFIX, table, operation, id_str)
    }

    /// 生成查询缓存键
    fn generate_query_cache_key(&self, table: &str, options: &QueryOptions) -> String {
        let query_hash = self.hash_query_options(options);
        format!("{}:{}:query:{}", CACHE_KEY_PREFIX, table, query_hash)
    }

    /// 计算查询选项的哈希值
    fn hash_query_options(&self, options: &QueryOptions) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        
        // 序列化查询选项并计算哈希
        if let Ok(serialized) = serde_json::to_string(options) {
            serialized.hash(&mut hasher);
        }
        
        format!("{:x}", hasher.finish())
    }

    /// 缓存单个记录
    pub async fn cache_record(
        &self,
        table: &str,
        id: &IdType,
        data: &DataValue,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let key = self.generate_cache_key(table, id, "record");
        let serialized = serde_json::to_vec(data)
            .map_err(|e| anyhow!("Failed to serialize data: {}", e))?;

        let options = CacheOptions::new()
            .with_ttl(Duration::from_secs(self.config.ttl_config.default_ttl_secs));

        self.cache.set_with_options(&key, serialized, options).await
            .map_err(|e| anyhow!("Failed to cache record: {}", e))?;

        // 记录缓存键
        self.track_cache_key(table, key).await;

        debug!("已缓存记录: table={}, id={:?}", table, id);
        Ok(())
    }

    /// 获取缓存的记录
    pub async fn get_cached_record(
        &self,
        table: &str,
        id: &IdType,
    ) -> Result<Option<DataValue>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let key = self.generate_cache_key(table, id, "record");
        
        match self.cache.get(&key).await {
            Ok(Some(data)) => {
                let deserialized: DataValue = serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize cached data: {}", e))?;
                debug!("缓存命中: table={}, id={:?}", table, id);
                Ok(Some(deserialized))
            }
            Ok(None) => {
                debug!("缓存未命中: table={}, id={:?}", table, id);
                Ok(None)
            }
            Err(e) => {
                warn!("缓存读取失败: {}", e);
                Ok(None)
            }
        }
    }

    /// 缓存查询结果
    pub async fn cache_query_result(
        &self,
        table: &str,
        options: &QueryOptions,
        results: &[DataValue],
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let key = self.generate_query_cache_key(table, options);
        let serialized = serde_json::to_vec(results)
            .map_err(|e| anyhow!("Failed to serialize query results: {}", e))?;

        let cache_options = CacheOptions::new()
            .with_ttl(Duration::from_secs(self.config.ttl_config.default_ttl_secs));

        self.cache.set_with_options(&key, serialized, cache_options).await
            .map_err(|e| anyhow!("Failed to cache query results: {}", e))?;

        // 记录缓存键
        self.track_cache_key(table, key).await;

        debug!("已缓存查询结果: table={}", table);
        Ok(())
    }

    /// 获取缓存的查询结果
    pub async fn get_cached_query_result(
        &self,
        table: &str,
        options: &QueryOptions,
    ) -> Result<Option<Vec<DataValue>>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let key = self.generate_query_cache_key(table, options);
        
        match self.cache.get(&key).await {
            Ok(Some(data)) => {
                let deserialized: Vec<DataValue> = serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize cached query results: {}", e))?;
                debug!("查询缓存命中: table={}", table);
                Ok(Some(deserialized))
            }
            Ok(None) => {
                debug!("查询缓存未命中: table={}", table);
                Ok(None)
            }
            Err(e) => {
                warn!("查询缓存读取失败: {}", e);
                Ok(None)
            }
        }
    }

    /// 删除单个记录的缓存
    pub async fn invalidate_record(&self, table: &str, id: &IdType) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let key = self.generate_cache_key(table, id, "record");
        
        if let Err(e) = self.cache.delete(&key).await {
            warn!("删除缓存记录失败: {}", e);
        }

        debug!("已删除缓存记录: table={}, id={:?}", table, id);
        Ok(())
    }

    /// 清理表的所有缓存
    pub async fn invalidate_table(&self, table: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut table_keys = self.table_keys.write().await;
        if let Some(keys) = table_keys.get(table) {
            for key in keys {
                if let Err(e) = self.cache.delete(key).await {
                    warn!("删除缓存键失败: key={}, error={}", key, e);
                }
            }
            table_keys.remove(table);
        }

        info!("已清理表缓存: table={}", table);
        Ok(())
    }

    /// 清理所有缓存
    pub async fn clear_all(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Err(e) = self.cache.clear().await {
            return Err(anyhow!("Failed to clear all cache: {}", e));
        }

        // 清理键跟踪
        let mut table_keys = self.table_keys.write().await;
        table_keys.clear();

        info!("已清理所有缓存");
        Ok(())
    }

    /// 获取缓存统计信息
    pub async fn get_stats(&self) -> Result<CacheStats> {
        if !self.config.enabled {
            return Ok(CacheStats::default());
        }

        // 这里需要根据 rat_memcache 的实际 API 来获取统计信息
        // 暂时返回默认值
        Ok(CacheStats::default())
    }

    /// 记录缓存键（用于表级别的缓存清理）
    async fn track_cache_key(&self, table: &str, key: String) {
        let mut table_keys = self.table_keys.write().await;
        table_keys.entry(table.to_string())
            .or_insert_with(Vec::new)
            .push(key);
    }

    /// 检查缓存是否启用
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// 缓存统计信息
#[cfg(feature = "cache")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存命中率
    pub hit_rate: f64,
    /// 当前缓存条目数
    pub entries: usize,
    /// 内存使用量（字节）
    pub memory_usage_bytes: usize,
    /// 磁盘使用量（字节）
    pub disk_usage_bytes: usize,
}

#[cfg(feature = "cache")]
impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            entries: 0,
            memory_usage_bytes: 0,
            disk_usage_bytes: 0,
        }
    }
}

#[cfg(not(feature = "cache"))]
pub struct CacheManager;

#[cfg(not(feature = "cache"))]
impl CacheManager {
    pub async fn new(_config: ()) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub async fn cache_record(&self, _table: &str, _id: &crate::types::IdType, _data: &crate::types::DataValue) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn get_cached_record(&self, _table: &str, _id: &crate::types::IdType) -> anyhow::Result<Option<crate::types::DataValue>> {
        Ok(None)
    }

    pub async fn cache_query_result(&self, _table: &str, _options: &crate::types::QueryOptions, _results: &[crate::types::DataValue]) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn get_cached_query_result(&self, _table: &str, _options: &crate::types::QueryOptions) -> anyhow::Result<Option<Vec<crate::types::DataValue>>> {
        Ok(None)
    }

    pub async fn invalidate_record(&self, _table: &str, _id: &crate::types::IdType) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn invalidate_table(&self, _table: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn clear_all(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        false
    }
}