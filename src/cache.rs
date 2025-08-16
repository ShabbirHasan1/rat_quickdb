//! 缓存管理模块
//! 
//! 提供基于 rat_memcache 的自动缓存功能，支持 LRU 策略、自动更新/清理缓存
//! 以及可选的手动清理接口。

use crate::types::{
    CacheConfig, CacheStrategy, CompressionAlgorithm, DataValue, IdType, QueryOptions, SortDirection,
};
use rat_memcache::RatMemCacheBuilder;
use rat_memcache::config::{L1Config, L2Config, TtlConfig, CompressionConfig};
use rat_memcache::types::EvictionStrategy;
use anyhow::{anyhow, Result};
use rat_memcache::{RatMemCache, CacheOptions};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH, Instant},
    path::PathBuf,
};
use tokio::sync::RwLock;
use zerg_creep::{info, debug, warn};

/// 缓存键前缀
const CACHE_KEY_PREFIX: &str = "rat_quickdb";

/// 缓存性能统计
#[derive(Debug, Clone)]
pub struct CachePerformanceStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存写入次数
    pub writes: u64,
    /// 缓存删除次数
    pub deletes: u64,
    /// 总查询延迟（纳秒）
    pub total_query_latency_ns: u64,
    /// 总写入延迟（纳秒）
    pub total_write_latency_ns: u64,
    /// 查询次数
    pub query_count: u64,
    /// 写入次数
    pub write_count: u64,
}

impl CachePerformanceStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            writes: 0,
            deletes: 0,
            total_query_latency_ns: 0,
            total_write_latency_ns: 0,
            query_count: 0,
            write_count: 0,
        }
    }

    /// 计算命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// 计算平均查询延迟（毫秒）
    pub fn avg_query_latency_ms(&self) -> f64 {
        if self.query_count == 0 {
            0.0
        } else {
            (self.total_query_latency_ns as f64 / self.query_count as f64) / 1_000_000.0
        }
    }

    /// 计算平均写入延迟（毫秒）
    pub fn avg_write_latency_ms(&self) -> f64 {
        if self.write_count == 0 {
            0.0
        } else {
            (self.total_write_latency_ns as f64 / self.write_count as f64) / 1_000_000.0
        }
    }
}

/// 缓存管理器
#[derive(Debug, Clone)]
pub struct CacheManager {
    /// 内部缓存实例
    cache: Arc<RatMemCache>,
    /// 缓存配置
    config: CacheConfig,
    /// 表名到缓存键的映射
    table_keys: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 性能统计
    stats: Arc<RwLock<CachePerformanceStats>>,
    /// 原子计数器用于高频统计
    hits_counter: Arc<AtomicU64>,
    misses_counter: Arc<AtomicU64>,
    writes_counter: Arc<AtomicU64>,
    deletes_counter: Arc<AtomicU64>,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub async fn new(config: CacheConfig) -> Result<Self> {
        // 直接使用用户传入的配置，不使用预设配置
        debug!("创建缓存管理器，配置: {:?}", config);
        
        let builder = RatMemCacheBuilder::new()
            .l1_config(rat_memcache::config::L1Config {
                max_memory: config.l1_config.max_memory_mb * 1024 * 1024, // 转换为字节
                max_entries: config.l1_config.max_capacity,
                eviction_strategy: match config.strategy {
                    CacheStrategy::Lru => EvictionStrategy::Lru,
                    CacheStrategy::Lfu => EvictionStrategy::Lfu,
                    CacheStrategy::Fifo => EvictionStrategy::Fifo,
                    CacheStrategy::Custom(_) => EvictionStrategy::Lru, // 默认使用LRU
                },
                enable_smart_transfer: true,
                pool_size: 1000,
            })
            .l2_config(rat_memcache::config::L2Config {
                enable_l2_cache: config.l2_config.is_some(),
                data_dir: config.l2_config.as_ref().map(|c| PathBuf::from(&c.storage_path)),
                max_disk_size: config.l2_config.as_ref().map(|c| c.max_disk_mb as u64 * 1024 * 1024).unwrap_or(500 * 1024 * 1024),
                write_buffer_size: 64 * 1024 * 1024,
                max_write_buffer_number: 3,
                block_cache_size: 16 * 1024 * 1024,
                enable_compression: config.compression_config.enabled,
                compression_level: config.l2_config.as_ref().map(|c| c.compression_level).unwrap_or(6),
                background_threads: 2,
            })
            .compression_config(rat_memcache::config::CompressionConfig {
                enable_lz4: config.compression_config.enabled,
                compression_threshold: config.compression_config.threshold_bytes,
                compression_level: config.l2_config.as_ref().map(|c| c.compression_level).unwrap_or(6),
                auto_compression: true,
                min_compression_ratio: 0.8,
            })
            .ttl_config(rat_memcache::config::TtlConfig {
                default_ttl: Some(config.ttl_config.default_ttl_secs),
                max_ttl: config.ttl_config.max_ttl_secs,
                cleanup_interval: config.ttl_config.check_interval_secs,
                max_cleanup_entries: 1000,
                lazy_expiration: true,
                active_expiration: true,
            })
            .performance_config(rat_memcache::config::PerformanceConfig {
                worker_threads: 4,
                enable_concurrency: true,
                read_write_separation: true,
                batch_size: 1000,
                enable_warmup: true,
                stats_interval: 60,
                enable_background_stats: true,
                l2_write_strategy: "WriteThrough".to_string(),
                l2_write_threshold: 1024,
                l2_write_ttl_threshold: 3600,
            })
            .logging_config(rat_memcache::config::LoggingConfig {
                level: "INFO".to_string(),
                enable_colors: true,
                show_timestamp: true,
                enable_performance_logs: true,
                enable_audit_logs: true,
                enable_cache_logs: true,
            });

        let cache = builder
            .build()
            .await
            .map_err(|e| anyhow!("Failed to create cache: {}", e))?;

        info!("缓存管理器初始化成功 - L1容量: {}, L1内存: {}MB, L2磁盘: {}MB, 策略: {:?}", 
              config.l1_config.max_capacity, 
              config.l1_config.max_memory_mb,
              config.l2_config.as_ref().map(|c| c.max_disk_mb).unwrap_or(0),
              config.strategy);

        Ok(Self {
            cache: Arc::new(cache),
            config,
            table_keys: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CachePerformanceStats::new())),
            hits_counter: Arc::new(AtomicU64::new(0)),
            misses_counter: Arc::new(AtomicU64::new(0)),
            writes_counter: Arc::new(AtomicU64::new(0)),
            deletes_counter: Arc::new(AtomicU64::new(0)),
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

    /// 生成查询缓存键 - 优化版本，避免复杂序列化
    fn generate_query_cache_key(&self, table: &str, options: &QueryOptions) -> String {
        let query_signature = self.build_query_signature(options);
        let key = format!("{}:{}:query:{}", CACHE_KEY_PREFIX, table, query_signature);
        debug!("生成查询缓存键: table={}, key={}", table, key);
        key
    }

    /// 构建查询签名 - 高效版本，避免JSON序列化
    fn build_query_signature(&self, options: &QueryOptions) -> String {
        let mut parts = Vec::new();
        
        // 分页信息
        if let Some(pagination) = &options.pagination {
            parts.push(format!("p{}_{}", pagination.skip, pagination.limit));
        }
        
        // 排序信息
        if !options.sort.is_empty() {
            let sort_str = options.sort.iter()
                .map(|s| format!("{}{}", s.field, match s.direction { SortDirection::Asc => "a", SortDirection::Desc => "d" }))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("s{}", sort_str));
        }
        
        // 投影信息
        if !options.fields.is_empty() {
            let proj_str = options.fields.join(",");
            parts.push(format!("f{}", proj_str));
        }
        
        // 连接部分生成最终签名
        if parts.is_empty() {
            "default".to_string()
        } else {
            parts.join("_")
        }
    }

    /// 缓存单个记录 - 优化版本
    pub async fn cache_record(
        &self,
        table: &str,
        id: &IdType,
        data: &DataValue,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let start_time = Instant::now();
        let key = self.generate_cache_key(table, id, "record");
        
        // 优化：使用更高效的序列化方式
        let serialized = match data {
            DataValue::Json(json_val) => {
                // 对于JSON值，直接序列化JSON而不是DataValue包装
                serde_json::to_vec(json_val)
                    .map_err(|e| anyhow!("Failed to serialize JSON data: {}", e))?
            }
            _ => {
                // 其他类型使用标准序列化
                serde_json::to_vec(data)
                    .map_err(|e| anyhow!("Failed to serialize data: {}", e))?
            }
        };

        let options = CacheOptions {
            ttl_seconds: Some(self.config.ttl_config.default_ttl_secs),
            ..Default::default()
        };

        self.cache.set_with_options(key.clone(), Bytes::from(serialized), &options).await
            .map_err(|e| anyhow!("Failed to cache record: {}", e))?;

        // 记录缓存键
        self.track_cache_key(table, key).await;

        // 更新统计信息
        let elapsed = start_time.elapsed();
        self.writes_counter.fetch_add(1, Ordering::Relaxed);
        {
            let mut stats = self.stats.write().await;
            stats.writes += 1;
            stats.write_count += 1;
            stats.total_write_latency_ns += elapsed.as_nanos() as u64;
        }

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

        let start_time = Instant::now();
        let key = self.generate_cache_key(table, id, "record");
        
        match self.cache.get(&key).await {
            Ok(Some(data)) => {
                let deserialized: DataValue = serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize cached data: {}", e))?;
                
                // 更新命中统计
                let elapsed = start_time.elapsed();
                self.hits_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.hits += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                debug!("缓存命中: table={}, id={:?}", table, id);
                Ok(Some(deserialized))
            }
            Ok(None) => {
                // 更新未命中统计
                let elapsed = start_time.elapsed();
                self.misses_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                debug!("缓存未命中: table={}, id={:?}", table, id);
                Ok(None)
            }
            Err(e) => {
                // 错误也算作未命中
                let elapsed = start_time.elapsed();
                self.misses_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                warn!("缓存读取失败: {}", e);
                Ok(None)
            }
        }
    }

    /// 缓存查询结果 - 优化版本
    pub async fn cache_query_result(
        &self,
        table: &str,
        options: &QueryOptions,
        results: &[DataValue],
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let start_time = Instant::now();
        let key = self.generate_query_cache_key(table, options);
        
        debug!("尝试缓存查询结果: table={}, key={}, options={:?}, 结果数量={}", table, key, options, results.len());

        // 优化：避免缓存空结果或过大结果
        if results.is_empty() {
            debug!("跳过缓存空查询结果: table={}", table);
            return Ok(());
        }
        
        // 限制缓存结果大小，避免内存浪费
        if results.len() > 1000 {
            debug!("跳过缓存过大查询结果: table={}, count={}", table, results.len());
            return Ok(());
        }
        
        // 优化：提取JSON值进行序列化，减少包装开销
        let json_results: Vec<serde_json::Value> = results.iter()
            .filter_map(|dv| if let DataValue::Json(json_val) = dv { Some(json_val.clone()) } else { None })
            .collect();
            
        let serialized = serde_json::to_vec(&json_results)
            .map_err(|e| anyhow!("Failed to serialize query results: {}", e))?;

        let cache_options = CacheOptions {
            ttl_seconds: Some(self.config.ttl_config.default_ttl_secs),
            ..Default::default()
        };

        self.cache.set_with_options(key.clone(), Bytes::from(serialized), &cache_options).await
            .map_err(|e| anyhow!("Failed to cache query results: {}", e))?;

        // 记录缓存键
        self.track_cache_key(table, key.clone()).await;

        // 更新统计信息
        let elapsed = start_time.elapsed();
        self.writes_counter.fetch_add(1, Ordering::Relaxed);
        {
            let mut stats = self.stats.write().await;
            stats.writes += 1;
            stats.write_count += 1;
            stats.total_write_latency_ns += elapsed.as_nanos() as u64;
        }

        debug!("已缓存查询结果: table={}, key={}, count={}", table, key, results.len());
        Ok(())
    }

    /// 获取缓存的查询结果 - 优化版本
    pub async fn get_cached_query_result(
        &self,
        table: &str,
        options: &QueryOptions,
    ) -> Result<Option<Vec<DataValue>>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let start_time = Instant::now();
        let key = self.generate_query_cache_key(table, options);
        
        debug!("尝试获取查询缓存: table={}, key={}, options={:?}", table, key, options);
        
        match self.cache.get(&key).await {
            Ok(Some(data)) => {
                // 优化：直接反序列化为JSON值，然后包装为DataValue
                let json_results: Vec<serde_json::Value> = serde_json::from_slice(&data)
                    .map_err(|e| anyhow!("Failed to deserialize cached query results: {}", e))?;
                    
                let data_values: Vec<DataValue> = json_results.into_iter()
                    .map(|json_val| DataValue::Json(json_val))
                    .collect();
                
                // 更新命中统计
                let elapsed = start_time.elapsed();
                self.hits_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.hits += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                    
                debug!("查询缓存命中: table={}, key={}, count={}", table, key, data_values.len());
                Ok(Some(data_values))
            }
            Ok(None) => {
                // 更新未命中统计
                let elapsed = start_time.elapsed();
                self.misses_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                debug!("查询缓存未命中: table={}, key={}", table, key);
                Ok(None)
            }
            Err(e) => {
                // 错误也算作未命中
                let elapsed = start_time.elapsed();
                self.misses_counter.fetch_add(1, Ordering::Relaxed);
                {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                    stats.query_count += 1;
                    stats.total_query_latency_ns += elapsed.as_nanos() as u64;
                }
                
                warn!("查询缓存读取失败: table={}, key={}, error={}", table, key, e);
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
        } else {
            // 更新删除统计
            self.deletes_counter.fetch_add(1, Ordering::Relaxed);
            {
                let mut stats = self.stats.write().await;
                stats.deletes += 1;
            }
        }

        debug!("已删除缓存记录: table={}, id={:?}", table, id);
        Ok(())
    }

    /// 清理表的所有缓存
    pub async fn invalidate_table(&self, table: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let pattern = format!("{}:{}:*", CACHE_KEY_PREFIX, table);
        self.clear_by_pattern(&pattern).await.map(|_| ())
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

    /// 按模式清理缓存（支持通配符匹配）
    /// 
    /// # 参数
    /// * `pattern` - 缓存键模式，支持通配符 * 和 ?
    /// 
    /// # 示例
    /// ```no_run
    /// # use rat_quickdb::cache::CacheManager;
    /// # async fn example(cache_manager: &CacheManager) -> rat_quickdb::QuickDbResult<()> {
    /// // 清理所有用户表相关的缓存
    /// cache_manager.clear_by_pattern("rat_quickdb:users:*").await?;
    /// // 清理所有查询缓存
    /// cache_manager.clear_by_pattern("*:query:*").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear_by_pattern(&self, pattern: &str) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let mut cleared_count = 0;
        let table_keys = self.table_keys.read().await;
        
        // 遍历所有跟踪的缓存键
        for (table_name, keys) in table_keys.iter() {
            let mut keys_to_remove = Vec::new();
            
            for key in keys {
                if self.matches_pattern(key, pattern) {
                    if let Err(e) = self.cache.delete(key).await {
                        warn!("删除匹配模式的缓存键失败: key={}, pattern={}, error={}", key, pattern, e);
                    } else {
                        keys_to_remove.push(key.clone());
                        cleared_count += 1;
                        debug!("已删除匹配模式的缓存键: key={}, pattern={}", key, pattern);
                    }
                }
            }
        }

        info!("按模式清理缓存完成: pattern={}, cleared_count={}", pattern, cleared_count);
        Ok(cleared_count)
    }

    /// 批量清理记录缓存
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `ids` - 要清理的记录ID列表
    pub async fn clear_records_batch(&self, table: &str, ids: &[IdType]) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let mut cleared_count = 0;
        
        for id in ids {
            let key = self.generate_cache_key(table, id, "record");
            
            if let Err(e) = self.cache.delete(&key).await {
                warn!("批量删除缓存记录失败: table={}, id={:?}, error={}", table, id, e);
            } else {
                cleared_count += 1;
                debug!("已删除缓存记录: table={}, id={:?}", table, id);
            }
        }

        info!("批量清理记录缓存完成: table={}, total={}, cleared={}", table, ids.len(), cleared_count);
        Ok(cleared_count)
    }

    /// 强制清理过期缓存
    /// 
    /// 手动触发过期缓存的清理，通常用于内存紧张或需要立即释放空间的场景
    pub async fn force_cleanup_expired(&self) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        // 由于 rat_memcache 内部处理过期清理，这里我们通过重新验证所有跟踪的键来实现
        let mut expired_count = 0;
        let mut table_keys = self.table_keys.write().await;
        
        for (table_name, keys) in table_keys.iter_mut() {
            let mut valid_keys = Vec::new();
            
            for key in keys.iter() {
                // 尝试获取缓存，如果不存在则认为已过期
                match self.cache.get(key).await {
                    Ok(Some(_)) => {
                        valid_keys.push(key.clone());
                    }
                    Ok(None) => {
                        expired_count += 1;
                        debug!("发现过期缓存键: key={}", key);
                    }
                    Err(e) => {
                        warn!("检查缓存键时出错: key={}, error={}", key, e);
                        // 出错的键也从跟踪中移除
                        expired_count += 1;
                    }
                }
            }
            
            *keys = valid_keys;
        }

        info!("强制清理过期缓存完成: expired_count={}", expired_count);
        Ok(expired_count)
    }

    /// 缓存预热 - 预加载热点数据
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `hot_ids` - 热点数据ID列表
    pub async fn warmup_cache(&self, table: &str, hot_ids: &[IdType]) -> Result<usize> {
        if !self.config.enabled || hot_ids.is_empty() {
            return Ok(0);
        }

        let mut warmed_count = 0;
        for id in hot_ids {
            let cache_key = self.generate_cache_key(table, id, "record");
            
            // 检查是否已缓存
            if self.cache.get(&cache_key).await.is_ok() {
                continue; // 已缓存，跳过
            }
            
            // 这里应该从数据库加载数据并缓存
            // 由于需要数据库连接，这个方法应该在适配器层实现
            // 这里只是标记需要预热的键
            debug!("标记预热缓存键: {}", cache_key);
            warmed_count += 1;
        }
        
        info!("缓存预热完成: table={}, warmed_count={}", table, warmed_count);
        Ok(warmed_count)
    }

    /// 批量缓存记录 - 优化批量操作
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `records` - 记录列表
    pub async fn cache_records_batch_optimized(&self, table: &str, records: &[(IdType, DataValue)]) -> Result<usize> {
        if !self.config.enabled || records.is_empty() {
            return Ok(0);
        }

        let mut cached_count = 0;
        for (id, data) in records {
            if let Err(e) = self.cache_record(table, id, data).await {
                warn!("批量缓存记录失败: table={}, id={:?}, error={}", table, id, e);
                continue;
            }
            cached_count += 1;
        }
        
        info!("批量缓存记录完成: table={}, cached_count={}", table, cached_count);
        Ok(cached_count)
    }

    /// 获取所有缓存键列表（按表分组）
    /// 
    /// 用于调试和监控，可以查看当前缓存中有哪些键
    pub async fn list_cache_keys(&self) -> Result<HashMap<String, Vec<String>>> {
        if !self.config.enabled {
            return Ok(HashMap::new());
        }

        let table_keys = self.table_keys.read().await;
        let result = table_keys.clone();
        
        info!("获取缓存键列表: 表数量={}, 总键数量={}", 
              result.len(), 
              result.values().map(|v| v.len()).sum::<usize>());
        
        Ok(result)
    }

    /// 获取指定表的缓存键列表
    pub async fn list_table_cache_keys(&self, table: &str) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let table_keys = self.table_keys.read().await;
        let keys = table_keys.get(table).cloned().unwrap_or_default();
        
        debug!("获取表缓存键列表: table={}, 键数量={}", table, keys.len());
        Ok(keys)
    }

    /// 清理指定表的查询缓存
    /// 
    /// 只清理查询缓存，保留记录缓存
    pub async fn clear_table_query_cache(&self, table: &str) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let pattern = format!("{}:{}:query:*", CACHE_KEY_PREFIX, table);
        let cleared_count = self.clear_by_pattern(&pattern).await?;
        
        info!("清理表查询缓存完成: table={}, cleared_count={}", table, cleared_count);
        Ok(cleared_count)
    }

    /// 清理指定表的记录缓存
    /// 
    /// 只清理记录缓存，保留查询缓存
    pub async fn clear_table_record_cache(&self, table: &str) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let pattern = format!("{}:{}:record:*", CACHE_KEY_PREFIX, table);
        let cleared_count = self.clear_by_pattern(&pattern).await?;
        
        info!("清理表记录缓存完成: table={}, cleared_count={}", table, cleared_count);
        Ok(cleared_count)
    }

    /// 检查缓存键是否匹配模式
    /// 
    /// 支持简单的通配符匹配：* 匹配任意字符序列，? 匹配单个字符
    fn matches_pattern(&self, key: &str, pattern: &str) -> bool {
        // 简单的通配符匹配实现
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let key_chars: Vec<char> = key.chars().collect();
        
        self.match_recursive(&key_chars, 0, &pattern_chars, 0)
    }

    /// 递归匹配算法
    fn match_recursive(&self, key: &[char], key_idx: usize, pattern: &[char], pattern_idx: usize) -> bool {
        // 如果模式已经匹配完
        if pattern_idx >= pattern.len() {
            return key_idx >= key.len();
        }
        
        // 如果键已经匹配完但模式还有非*字符
        if key_idx >= key.len() {
            return pattern[pattern_idx..].iter().all(|&c| c == '*');
        }
        
        match pattern[pattern_idx] {
            '*' => {
                // * 可以匹配0个或多个字符
                // 尝试匹配0个字符（跳过*）
                if self.match_recursive(key, key_idx, pattern, pattern_idx + 1) {
                    return true;
                }
                // 尝试匹配1个或多个字符
                self.match_recursive(key, key_idx + 1, pattern, pattern_idx)
            }
            '?' => {
                // ? 匹配任意单个字符
                self.match_recursive(key, key_idx + 1, pattern, pattern_idx + 1)
            }
            c => {
                // 普通字符必须完全匹配
                if key[key_idx] == c {
                    self.match_recursive(key, key_idx + 1, pattern, pattern_idx + 1)
                } else {
                    false
                }
            }
        }
    }

    /// 获取缓存统计信息
    pub async fn get_stats(&self) -> Result<CacheStats> {
        if !self.config.enabled {
            return Ok(CacheStats::default());
        }

        // 从原子计数器和详细统计中获取数据
        let hits = self.hits_counter.load(Ordering::Relaxed);
        let misses = self.misses_counter.load(Ordering::Relaxed);
        let writes = self.writes_counter.load(Ordering::Relaxed);
        let deletes = self.deletes_counter.load(Ordering::Relaxed);
        
        let stats = self.stats.read().await;
        let hit_rate = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64
        } else {
            0.0
        };
        
        // 估算内存使用量（基于跟踪的键数量）
        let table_keys = self.table_keys.read().await;
        let entries = table_keys.values().map(|v| v.len()).sum::<usize>();
        
        Ok(CacheStats {
            hits,
            misses,
            hit_rate,
            entries,
            memory_usage_bytes: entries * 1024, // 粗略估算每个条目1KB
            disk_usage_bytes: 0, // rat_memcache 主要是内存缓存
        })
    }

    /// 获取详细的性能统计信息
    pub async fn get_performance_stats(&self) -> Result<CachePerformanceStats> {
        if !self.config.enabled {
            return Ok(CachePerformanceStats::new());
        }

        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        self.hits_counter.store(0, Ordering::Relaxed);
        self.misses_counter.store(0, Ordering::Relaxed);
        self.writes_counter.store(0, Ordering::Relaxed);
        self.deletes_counter.store(0, Ordering::Relaxed);
        
        {
            let mut stats = self.stats.write().await;
            *stats = CachePerformanceStats::new();
        }
        
        info!("缓存统计信息已重置");
        Ok(())
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



    /// 批量缓存记录 - 优化批量操作
    /// 
    /// # 参数
    /// * `table` - 表名
    /// * `records` - 记录列表
    pub async fn cache_records_batch(&self, table: &str, records: Vec<(IdType, DataValue)>) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let mut cached_count = 0;
        
        // 批量处理，减少锁竞争
        for (id, data) in records {
            if let Err(e) = self.cache_record(table, &id, &data).await {
                warn!("批量缓存记录失败: table={}, id={:?}, error={}", table, id, e);
            } else {
                cached_count += 1;
            }
        }

        info!("批量缓存记录完成: table={}, cached_count={}", table, cached_count);
        Ok(cached_count)
    }
}

/// 缓存统计信息
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