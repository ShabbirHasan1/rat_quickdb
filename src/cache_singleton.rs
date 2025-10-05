//! 全局缓存管理器单例
//!
//! 提供全局唯一的缓存管理器实例，确保性能和一致性

use crate::types::{CacheConfig, CacheStrategy, CompressionAlgorithm};
use rat_memcache::{RatMemCache, RatMemCacheBuilder, CacheOptions};
use rat_memcache::config::{L1Config, L2Config, TtlConfig, PerformanceConfig as RatMemCachePerformanceConfig};
use rat_memcache::types::EvictionStrategy;
use rat_memcache::config::CacheWarmupStrategy;
use anyhow::{anyhow, Result};
use bytes::Bytes;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use rat_logger::{info, debug, warn};

/// 缓存管理器
#[derive(Debug, Clone)]
pub struct CacheManager {
    cache: Arc<RatMemCache>,
    config: CacheConfig,
    /// 键索引，用于前缀匹配清理
    key_index: Arc<RwLock<HashMap<String, usize>>>, // 键 -> 访问次数映射
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub async fn new(config: CacheConfig) -> Result<Self> {
        debug!("创建缓存管理器，配置: {:?}", config);

        let builder = RatMemCacheBuilder::new()
            .l1_config(L1Config {
                max_memory: config.l1_config.max_memory_mb * 1024 * 1024,
                max_entries: config.l1_config.max_capacity,
                eviction_strategy: match config.strategy {
                    CacheStrategy::Lru => EvictionStrategy::Lru,
                    CacheStrategy::Lfu => EvictionStrategy::Lfu,
                    CacheStrategy::Fifo => EvictionStrategy::Fifo,
                    CacheStrategy::Custom(_) => EvictionStrategy::Lru,
                },
            })
            .l2_config(if let Some(l2_config) = &config.l2_config {
                L2Config {
                    enable_l2_cache: l2_config.enable_l2_cache,
                    data_dir: l2_config.data_dir.clone().map(PathBuf::from),
                    max_disk_size: l2_config.max_disk_size,
                    write_buffer_size: l2_config.write_buffer_size as usize,
                    max_write_buffer_number: l2_config.max_write_buffer_number as i32,
                    block_cache_size: l2_config.block_cache_size as usize,
                    background_threads: l2_config.background_threads as i32,
                    enable_lz4: l2_config.enable_lz4,
                    compression_threshold: l2_config.compression_threshold,
                    compression_max_threshold: l2_config.compression_max_threshold,
                    compression_level: l2_config.compression_level,
                    clear_on_startup: l2_config.clear_on_startup,
                    cache_size_mb: l2_config.cache_size_mb,
                    max_file_size_mb: l2_config.max_file_size_mb,
                    smart_flush_enabled: l2_config.smart_flush_enabled,
                    smart_flush_base_interval_ms: l2_config.smart_flush_base_interval_ms as usize,
                    smart_flush_min_interval_ms: l2_config.smart_flush_min_interval_ms as usize,
                    smart_flush_max_interval_ms: l2_config.smart_flush_max_interval_ms as usize,
                    smart_flush_write_rate_threshold: l2_config.smart_flush_write_rate_threshold as usize,
                    smart_flush_accumulated_bytes_threshold: l2_config.smart_flush_accumulated_bytes_threshold as usize,
                    cache_warmup_strategy: match l2_config.cache_warmup_strategy.as_str() {
                        "None" => CacheWarmupStrategy::None,
                        "Recent" => CacheWarmupStrategy::Recent,
                        "Frequent" => CacheWarmupStrategy::Frequent,
                        "Full" => CacheWarmupStrategy::Full,
                        _ => CacheWarmupStrategy::Recent, // 默认值
                    },
                    zstd_compression_level: l2_config.zstd_compression_level,
                    l2_write_strategy: l2_config.l2_write_strategy.clone(),
                    l2_write_threshold: l2_config.l2_write_threshold as usize,
                    l2_write_ttl_threshold: l2_config.l2_write_ttl_threshold as u64,
                }
            } else {
                // 提供默认的L2配置（禁用L2缓存）
                L2Config {
                    enable_l2_cache: false,
                    data_dir: None,
                    max_disk_size: 256 * 1024 * 1024,
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
                    cache_warmup_strategy: CacheWarmupStrategy::Recent,
                    zstd_compression_level: None,
                    l2_write_strategy: "async".to_string(),
                    l2_write_threshold: 1024,
                    l2_write_ttl_threshold: 3600,
                }
            })
            .performance_config(RatMemCachePerformanceConfig {
                worker_threads: config.performance_config.worker_threads,
                enable_concurrency: config.performance_config.enable_concurrency,
                read_write_separation: config.performance_config.read_write_separation,
                batch_size: config.performance_config.batch_size,
                enable_warmup: config.performance_config.enable_warmup,
                large_value_threshold: config.performance_config.large_value_threshold,
            })
              .ttl_config(TtlConfig {
                expire_seconds: config.ttl_config.expire_seconds,
                cleanup_interval: config.ttl_config.cleanup_interval,
                max_cleanup_entries: config.ttl_config.max_cleanup_entries,
                lazy_expiration: config.ttl_config.lazy_expiration,
                active_expiration: config.ttl_config.active_expiration,
            });

        let cache = builder
            .build()
            .await
            .map_err(|e| anyhow!("Failed to create cache: {}", e))?;

        info!("缓存管理器初始化成功 - L1容量: {}, L1内存: {}MB, L2磁盘: {}MB, 策略: {:?}",
              config.l1_config.max_capacity,
              config.l1_config.max_memory_mb,
              config.l2_config.as_ref().map(|c| c.max_disk_size / (1024 * 1024)).unwrap_or(0),
              config.strategy);

        Ok(Self {
            cache: Arc::new(cache),
            config,
            key_index: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 获取缓存数据（内部方法）- 原样进出
    pub(crate) async fn get(&self, key: &str) -> Result<(bool, Option<Vec<u8>>)> {
        let result = match self.cache.get(key).await {
            Ok(Some(data)) => {
                // 缓存命中，更新键索引
                let mut index = self.key_index.write().await;
                let count = index.entry(key.to_string()).or_insert(0);
                *count += 1;
                Ok((true, Some(data.to_vec())))
            },
            Ok(None) => {
                // 缓存未命中
                Ok((false, None))
            },
            Err(e) => {
                warn!("获取缓存失败: {}", e);
                Ok((false, None))
            }
        };

        result
    }

    /// 设置缓存数据（带TTL，内部方法）- 原样进出
    pub(crate) async fn set_with_ttl(&self, key: &str, data: Vec<u8>, ttl: Option<u64>) -> Result<()> {
        let cache_options = CacheOptions {
            ttl_seconds: ttl,
            ..Default::default()
        };

        self.cache.set_with_options(key.to_string(), Bytes::from(data), &cache_options).await?;

        // 添加到键索引
        let mut index = self.key_index.write().await;
        index.entry(key.to_string()).or_insert(0);

        Ok(())
    }

    /// 删除缓存数据（内部方法）
    pub(crate) async fn delete(&self, key: &str) -> Result<()> {
        let deleted = self.cache.delete(key).await?;

        // 只有真正删除了才从键索引中移除
        if deleted {
            let mut index = self.key_index.write().await;
            index.remove(key);
            debug!("成功删除缓存键: {}", key);
        } else {
            debug!("缓存键不存在，无需删除: {}", key);
        }

        Ok(())
    }

    /// 按前缀清理缓存（内部方法）
    pub(crate) async fn clear_by_prefix(&self, prefix: &str) -> Result<()> {
        debug!("按前缀清理缓存: {}", prefix);

        // 读取键索引中的所有键
        let keys_to_delete: Vec<String> = {
            let index = self.key_index.read().await;
            index.keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect()
        };

        let mut deleted_count = 0;

        // 删除匹配前缀的缓存项
        for key in &keys_to_delete {
            if let Ok(_) = self.cache.delete(key).await {
                deleted_count += 1;
                debug!("删除缓存键: {}", key);
            }
        }

        // 从键索引中删除这些键
        if deleted_count > 0 {
            let mut index = self.key_index.write().await;
            for key in &keys_to_delete {
                index.remove(key);
            }
        }

        info!("按前缀清理完成，删除了 {} 个缓存项", deleted_count);
        Ok(())
    }

    /// 清理所有缓存（内部方法）
    pub(crate) async fn clear_all(&self) -> Result<()> {
        self.cache.clear().await?;
        info!("已清理所有缓存");
        Ok(())
    }

    /// 检查缓存是否启用
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// 检查是否启用了L2持久化缓存
    pub fn is_l2_enabled(&self) -> bool {
        self.config.l2_config.is_some()
    }

    /// 获取缓存配置
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// 获取缓存统计信息
    pub async fn get_stats(&self) -> Result<CacheStats> {
        // TODO: 实现统计信息获取
        // 目前rat_memcache可能没有提供统计接口，暂时返回默认值
        Ok(CacheStats::default())
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// 全局缓存管理器单例实例
static GLOBAL_CACHE_MANAGER: OnceCell<Arc<CacheManager>> = OnceCell::const_new();

/// 全局缓存管理器错误
#[derive(Debug, thiserror::Error)]
pub enum GlobalCacheError {
    #[error("缓存管理器未初始化")]
    NotInitialized,
    #[error("缓存管理器已初始化，不能重复初始化")]
    AlreadyInitialized,
    #[error("缓存配置错误: {0}")]
    ConfigError(String),
}

/// 全局缓存管理器
pub struct GlobalCacheManager;

impl GlobalCacheManager {
    /// 初始化全局缓存管理器
    ///
    /// # 参数
    /// * `config` - 缓存配置
    ///
    /// # 返回
    /// * `Ok(())` - 初始化成功
    /// * `Err(GlobalCacheError)` - 初始化失败
    ///
    /// # 注意
    /// 只能调用一次，重复调用会返回错误
    pub async fn initialize(config: CacheConfig) -> Result<(), GlobalCacheError> {
        debug!("开始初始化全局缓存管理器");

        let cache_manager = match CacheManager::new(config).await {
            Ok(manager) => {
                debug!("缓存管理器创建成功");
                Arc::new(manager)
            }
            Err(e) => {
                warn!("缓存管理器创建失败: {}", e);
                return Err(GlobalCacheError::ConfigError(e.to_string()));
            }
        };

        GLOBAL_CACHE_MANAGER
            .set(cache_manager)
            .map_err(|_| GlobalCacheError::AlreadyInitialized)?;

        info!("全局缓存管理器初始化成功");
        Ok(())
    }

    /// 获取全局缓存管理器
    ///
    /// # 返回
    /// * `Some(Arc<CacheManager>)` - 缓存管理器实例
    /// * `None` - 缓存管理器未初始化
    pub fn get() -> Option<Arc<CacheManager>> {
        GLOBAL_CACHE_MANAGER.get().cloned()
    }

    /// 获取全局缓存管理器（如果未初始化则返回错误）
    ///
    /// # 返回
    /// * `Ok(Arc<CacheManager>)` - 缓存管理器实例
    /// * `Err(GlobalCacheError::NotInitialized)` - 缓存管理器未初始化
    pub fn get_or_error() -> Result<Arc<CacheManager>, GlobalCacheError> {
        GLOBAL_CACHE_MANAGER
            .get()
            .cloned()
            .ok_or(GlobalCacheError::NotInitialized)
    }

    /// 检查是否已初始化
    ///
    /// # 返回
    /// * `true` - 已初始化
    /// * `false` - 未初始化
    pub fn is_initialized() -> bool {
        GLOBAL_CACHE_MANAGER.get().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CacheConfig;

    #[tokio::test]
    async fn test_global_cache_manager() {
        // 确保未初始化
        assert!(!GlobalCacheManager::is_initialized());
        assert!(GlobalCacheManager::get().is_none());

        // 初始化
        let config = CacheConfig::default();
        assert!(GlobalCacheManager::initialize(config).await.is_ok());

        // 验证已初始化
        assert!(GlobalCacheManager::is_initialized());
        assert!(GlobalCacheManager::get().is_some());

        // 重复初始化应该失败
        let config = CacheConfig::default();
        assert!(matches!(
            GlobalCacheManager::initialize(config).await,
            Err(GlobalCacheError::AlreadyInitialized)
        ));
    }

    #[tokio::test]
    async fn test_get_or_error() {
        // 未初始化时应该返回错误
        assert!(matches!(
            GlobalCacheManager::get_or_error(),
            Err(GlobalCacheError::NotInitialized)
        ));

        // 初始化后应该成功
        let config = CacheConfig::default();
        assert!(GlobalCacheManager::initialize(config).await.is_ok());
        assert!(GlobalCacheManager::get_or_error().is_ok());
    }
}