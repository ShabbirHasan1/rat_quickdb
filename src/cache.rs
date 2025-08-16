//! 缓存管理模块
//! 
//! 提供基于 rat_memcache 的自动缓存功能，支持 LRU 策略、自动更新/清理缓存
//! 以及可选的手动清理接口。

#[cfg(feature = "cache")]
use crate::types::{
    CacheConfig, CacheStrategy, CompressionAlgorithm, DataValue, IdType, QueryOptions,
};
#[cfg(feature = "cache")]
use rat_memcache::RatMemCacheBuilder;
#[cfg(feature = "cache")]
use rat_memcache::config::{L1Config, TtlConfig, CompressionConfig};
#[cfg(feature = "cache")]
use rat_memcache::types::EvictionStrategy;
#[cfg(feature = "cache")]
use anyhow::{anyhow, Result};
#[cfg(feature = "cache")]
use rat_memcache::{RatMemCache, CacheOptions};
#[cfg(feature = "cache")]
use bytes::Bytes;
#[cfg(feature = "cache")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "cache")]
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
    path::PathBuf,
};
#[cfg(feature = "cache")]
use tokio::sync::RwLock;
#[cfg(feature = "cache")]
use zerg_creep::{info, debug, warn};

/// 缓存键前缀
#[cfg(feature = "cache")]
const CACHE_KEY_PREFIX: &str = "rat_quickdb";

/// 缓存管理器
#[cfg(feature = "cache")]
#[derive(Debug, Clone)]
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

        let cache = rat_memcache::RatMemCacheBuilder::new()
            .development_preset()
            .map_err(|e| anyhow!("Failed to create development preset: {}", e))?
            .build()
            .await
            .map_err(|e| anyhow!("Failed to create cache: {}", e))?;

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

        let options = CacheOptions {
            ttl_seconds: Some(self.config.ttl_config.default_ttl_secs),
            ..Default::default()
        };

        self.cache.set_with_options(key.clone(), Bytes::from(serialized), &options).await
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

        let cache_options = CacheOptions {
            ttl_seconds: Some(self.config.ttl_config.default_ttl_secs),
            ..Default::default()
        };

        self.cache.set_with_options(key.clone(), Bytes::from(serialized), &cache_options).await
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

    // 新增的手动清理方法的空实现
    pub async fn clear_by_pattern(&self, _pattern: &str) -> anyhow::Result<usize> {
        Ok(0)
    }

    pub async fn clear_records_batch(&self, _table: &str, _ids: &[crate::types::IdType]) -> anyhow::Result<usize> {
        Ok(0)
    }

    pub async fn force_cleanup_expired(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    pub async fn list_cache_keys(&self) -> anyhow::Result<std::collections::HashMap<String, Vec<String>>> {
        Ok(std::collections::HashMap::new())
    }

    pub async fn list_table_cache_keys(&self, _table: &str) -> anyhow::Result<Vec<String>> {
        Ok(Vec::new())
    }

    pub async fn clear_table_query_cache(&self, _table: &str) -> anyhow::Result<usize> {
        Ok(0)
    }

    pub async fn clear_table_record_cache(&self, _table: &str) -> anyhow::Result<usize> {
        Ok(0)
    }

    pub fn is_enabled(&self) -> bool {
        false
    }
}