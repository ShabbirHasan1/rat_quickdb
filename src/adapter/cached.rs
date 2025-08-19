//! 缓存数据库适配器
//! 
//! 提供带缓存功能的数据库适配器包装器，在适配器层实现缓存逻辑

use super::DatabaseAdapter;
use crate::cache::CacheManager;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::FieldType;
use crate::pool::DatabaseConnection;
use async_trait::async_trait;
use serde_json::Value;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use zerg_creep::{warn, debug};

/// 带缓存功能的数据库适配器包装器
pub struct CachedDatabaseAdapter {
    /// 内部真实的数据库适配器
    inner: Box<dyn DatabaseAdapter>,
    /// 缓存管理器
    cache_manager: Arc<CacheManager>,
}

impl CachedDatabaseAdapter {
    /// 创建新的缓存适配器
    pub fn new(inner: Box<dyn DatabaseAdapter>, cache_manager: Arc<CacheManager>) -> Self {
        Self {
            inner,
            cache_manager,
        }
    }
}

#[async_trait]
impl DatabaseAdapter for CachedDatabaseAdapter {
    /// 创建记录 - 创建成功后智能清理相关缓存
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<DataValue> {
        let result = self.inner.create(connection, table, data).await;
        
        // 创建成功后只清理查询缓存，保留记录缓存
        if result.is_ok() {
            if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                warn!("清理表查询缓存失败: {}", e);
            }
            debug!("已清理表查询缓存: table={}", table);
        }
        
        result
    }

    /// 根据ID查找记录 - 先检查缓存，缓存未命中时查询数据库并缓存结果
    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        // 将DataValue转换为IdType
        let id_type = match id {
            DataValue::Int(n) => IdType::Number(*n),
            DataValue::String(s) => IdType::String(s.clone()),
            _ => {
                warn!("无法将DataValue转换为IdType: {:?}", id);
                return self.inner.find_by_id(connection, table, id).await;
            }
        };

        // 先检查缓存
        match self.cache_manager.get_cached_record(table, &id_type).await {
            Ok(Some(cached_result)) => {
                debug!("缓存命中: 表={}, ID={:?}", table, id);
                return Ok(Some(cached_result));
            }
            Ok(None) => {
                debug!("缓存未命中: 表={}, ID={:?}", table, id);
            }
            Err(e) => {
                warn!("缓存查询失败: {}, 继续查询数据库", e);
            }
        }
        
        // 缓存未命中或查询失败，查询数据库
        let result = self.inner.find_by_id(connection, table, id).await;
        
        // 查询成功时缓存结果
        if let Ok(Some(ref record)) = result {
            if let Err(e) = self.cache_manager.cache_record(table, &id_type, record).await {
                warn!("缓存记录失败: {}", e);
            }
        }
        
        result
    }

    /// 查找记录 - 先检查缓存，缓存未命中时查询数据库并缓存结果
    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 先检查缓存
        match self.cache_manager.get_cached_query_result(table, options).await {
            Ok(Some(cached_result)) => {
                debug!("查询缓存命中: 表={}", table);
                return Ok(cached_result);
            }
            Ok(None) => {
                debug!("查询缓存未命中: 表={}", table);
            }
            Err(e) => {
                warn!("查询缓存失败: {}, 继续查询数据库", e);
            }
        }
        
        // 缓存未命中或查询失败，查询数据库
        let result = self.inner.find(connection, table, conditions, options).await;
        
        // 查询成功时缓存结果
        if let Ok(ref records) = result {
            if let Err(e) = self.cache_manager.cache_query_result(table, options, records).await {
                warn!("缓存查询结果失败: {}", e);
            }
        }
        
        result
    }

    /// 使用条件组合查找记录 - 先检查缓存，缓存未命中时查询数据库并缓存结果
    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 生成条件组合查询缓存键
        let cache_key = self.cache_manager.generate_condition_groups_cache_key(table, condition_groups, options);
        
        // 先检查缓存
        match self.cache_manager.get_cached_query_result(table, options).await {
            Ok(Some(cached_result)) => {
                debug!("条件组合查询缓存命中: 表={}, 键={}", table, cache_key);
                return Ok(cached_result);
            }
            Ok(None) => {
                debug!("条件组合查询缓存未命中: 表={}, 键={}", table, cache_key);
            }
            Err(e) => {
                warn!("获取条件组合查询缓存失败: {}", e);
            }
        }
        
        // 缓存未命中，查询数据库
        let result = self.inner.find_with_groups(connection, table, condition_groups, options).await?;
        
        // 缓存查询结果
        if let Err(e) = self.cache_manager.cache_query_result(table, options, &result).await {
            warn!("缓存条件组合查询结果失败: {}", e);
        } else {
            debug!("已缓存条件组合查询结果: 表={}, 键={}, 结果数量={}", table, cache_key, result.len());
        }
        
        Ok(result)
    }

    /// 更新记录 - 更新成功后智能清理相关缓存
    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        // 直接调用内部适配器更新记录
        let result = self.inner.update(connection, table, conditions, data).await;
        
        // 更新成功后只清理查询缓存，避免过度清理
        if let Ok(updated_count) = result {
            if updated_count > 0 {
                if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                    warn!("清理表查询缓存失败: {}", e);
                }
                debug!("已清理表查询缓存: table={}, updated_count={}", table, updated_count);
            }
        }
        
        result
    }

    /// 根据ID更新记录 - 更新成功后精确清理相关缓存
    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        // 直接调用内部适配器更新记录
        let result = self.inner.update_by_id(connection, table, id, data).await;
        
        // 更新成功后精确清理相关缓存
        if let Ok(true) = result {
            // 清理特定记录的缓存
            let id_value = match id {
                DataValue::Int(n) => IdType::Number(*n),
                DataValue::String(s) => IdType::String(s.clone()),
                _ => {
                    warn!("无法将DataValue转换为IdType: {:?}", id);
                    return result;
                }
            };
            
            // 清理记录缓存
            if let Err(e) = self.cache_manager.invalidate_record(table, &id_value).await {
                warn!("清理记录缓存失败: {}", e);
            }
            
            // 只清理查询缓存，不清理其他记录缓存
            if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                warn!("清理表查询缓存失败: {}", e);
            }
            
            debug!("已清理记录和查询缓存: table={}, id={:?}", table, id);
        }
        
        result
    }

    /// 删除记录 - 删除成功后智能清理相关缓存
    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        // 直接调用内部适配器删除记录
        let result = self.inner.delete(connection, table, conditions).await;
        
        // 删除成功后智能清理相关缓存
        if let Ok(deleted_count) = result {
            if deleted_count > 0 {
                // 对于批量删除，清理整个表的缓存是合理的
                if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                    warn!("清理表查询缓存失败: {}", e);
                }
                if let Err(e) = self.cache_manager.clear_table_record_cache(table).await {
                    warn!("清理表记录缓存失败: {}", e);
                }
                debug!("已清理表缓存: table={}, deleted_count={}", table, deleted_count);
            }
        }
        
        result
    }

    /// 根据ID删除记录 - 删除成功后精确清理相关缓存
    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        // 直接调用内部适配器删除记录
        let result = self.inner.delete_by_id(connection, table, id).await;
        
        // 删除成功后精确清理相关缓存
        if let Ok(true) = result {
            // 清理特定记录的缓存
            let id_value = match id {
                DataValue::Int(n) => IdType::Number(*n),
                DataValue::String(s) => IdType::String(s.clone()),
                _ => {
                    warn!("无法将DataValue转换为IdType: {:?}", id);
                    return result;
                }
            };
            
            // 清理记录缓存
            if let Err(e) = self.cache_manager.invalidate_record(table, &id_value).await {
                warn!("清理记录缓存失败: {}", e);
            }
            
            // 只清理查询缓存
            if let Err(e) = self.cache_manager.clear_table_query_cache(table).await {
                warn!("清理表查询缓存失败: {}", e);
            }
            
            debug!("已清理记录和查询缓存: table={}, id={:?}", table, id);
        }
        
        result
    }

    /// 统计记录数量 - 直接调用内部适配器，不缓存统计结果
    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        // 统计操作不缓存，直接调用内部适配器
        self.inner.count(connection, table, conditions).await
    }

    /// 检查记录是否存在 - 直接调用内部适配器，不缓存存在性检查结果
    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        // 存在性检查不缓存，直接调用内部适配器
        self.inner.exists(connection, table, conditions).await
    }

    /// 创建表/集合 - 直接调用内部适配器
    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldType>,
    ) -> QuickDbResult<()> {
        // 表结构操作不缓存，直接调用内部适配器
        self.inner.create_table(connection, table, fields).await
    }

    /// 创建索引 - 直接调用内部适配器
    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        // 索引操作不缓存，直接调用内部适配器
        self.inner.create_index(connection, table, index_name, fields, unique).await
    }

    /// 检查表是否存在 - 直接调用内部适配器
    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        // 表存在性检查不缓存，直接调用内部适配器
        self.inner.table_exists(connection, table).await
    }
}