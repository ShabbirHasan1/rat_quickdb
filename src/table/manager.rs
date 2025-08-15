//! 表管理器
//! 
//! 提供表的创建、检查、迁移等管理功能

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::{QuickDbResult, QuickDbError};
use crate::adapter::DatabaseAdapter;
use crate::PoolManager;
use super::schema::{TableSchema, ColumnDefinition, ColumnType};
use super::version::{VersionManager, SchemaVersion, MigrationScriptType};
use zerg_creep::info;


/// 表管理器
#[derive(Debug)]
pub struct TableManager {
    /// 连接池管理器
    pool_manager: Arc<crate::PoolManager>,
    /// 版本管理器
    version_manager: Arc<RwLock<VersionManager>>,
    /// 表模式缓存
    schema_cache: Arc<RwLock<HashMap<String, TableSchema>>>,
    /// 表存在性缓存
    existence_cache: Arc<RwLock<HashMap<String, bool>>>,
    /// 自动创建表开关
    auto_create_tables: bool,
    /// 自动迁移开关
    auto_migrate: bool,
}

/// 表管理配置
#[derive(Debug, Clone)]
pub struct TableManagerConfig {
    /// 是否自动创建表
    pub auto_create_tables: bool,
    /// 是否自动迁移
    pub auto_migrate: bool,
    /// 缓存过期时间（秒）
    pub cache_ttl_seconds: u64,
    /// 是否启用模式验证
    pub enable_schema_validation: bool,
}

impl Default for TableManagerConfig {
    fn default() -> Self {
        Self {
            auto_create_tables: true,
            auto_migrate: false,
            cache_ttl_seconds: 300, // 5分钟
            enable_schema_validation: true,
        }
    }
}

/// 表创建选项
#[derive(Debug, Clone)]
pub struct TableCreateOptions {
    /// 如果表已存在是否跳过
    pub if_not_exists: bool,
    /// 是否创建索引
    pub create_indexes: bool,
    /// 是否添加约束
    pub add_constraints: bool,
    /// 表选项
    pub table_options: Option<HashMap<String, String>>,
}

impl Default for TableCreateOptions {
    fn default() -> Self {
        Self {
            if_not_exists: true,
            create_indexes: true,
            add_constraints: true,
            table_options: None,
        }
    }
}

/// 表检查结果
#[derive(Debug, Clone)]
pub struct TableCheckResult {
    /// 表是否存在
    pub exists: bool,
    /// 当前数据库中的版本
    pub current_version: Option<u32>,
    /// 代码中定义的版本
    pub expected_version: Option<u32>,
    /// 是否需要迁移
    pub needs_migration: bool,
    /// 模式差异
    pub schema_diff: Option<SchemaDiff>,
}

/// 模式差异
#[derive(Debug, Clone)]
pub struct SchemaDiff {
    /// 新增的列
    pub added_columns: Vec<ColumnDefinition>,
    /// 删除的列
    pub removed_columns: Vec<String>,
    /// 修改的列
    pub modified_columns: Vec<(String, ColumnDefinition)>,
    /// 新增的索引
    pub added_indexes: Vec<String>,
    /// 删除的索引
    pub removed_indexes: Vec<String>,
}

impl TableManager {
    /// 创建新的表管理器
    pub fn new(
        pool_manager: Arc<crate::PoolManager>,
        config: TableManagerConfig,
    ) -> Self {
        Self {
            pool_manager,
            version_manager: Arc::new(RwLock::new(VersionManager::new())),
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
            existence_cache: Arc::new(RwLock::new(HashMap::new())),
            auto_create_tables: config.auto_create_tables,
            auto_migrate: config.auto_migrate,
        }
    }
    
    /// 检查表是否存在
    pub async fn check_table_exists(&self, table_name: &str) -> QuickDbResult<bool> {
        // 先检查缓存
        {
            let cache = self.existence_cache.read().await;
            if let Some(&exists) = cache.get(table_name) {
                return Ok(exists);
            }
        }
        
        // 从数据库检查
        let connection = self.pool_manager.get_connection(Some("default")).await?;
        let db_type = self.pool_manager.get_database_type("default")?;
        let adapter = crate::adapter::create_adapter(&db_type)?;
        
        // TODO: 实现 table_exists 方法
        // 这需要在 DatabaseAdapter trait 中添加 table_exists 方法
        let exists = false;
        
        self.pool_manager.release_connection(&connection).await?;
        
        // 更新缓存
        {
            let mut cache = self.existence_cache.write().await;
            cache.insert(table_name.to_string(), exists);
        }
        
        Ok(exists)
    }
    
    /// 创建表
    pub async fn create_table(
        &self,
        schema: &TableSchema,
        options: Option<TableCreateOptions>,
    ) -> QuickDbResult<()> {
        let options = options.unwrap_or_default();
        
        // 检查表是否已存在
        if options.if_not_exists {
            let exists = self.check_table_exists(&schema.name).await?;
            if exists {
                info!("表 {} 已存在，跳过创建", schema.name);
                return Ok(());
            }
        }
        
        let connection = self.pool_manager.get_connection(Some("default")).await?;
        let db_type = self.pool_manager.get_database_type("default")?;
        let adapter = crate::adapter::create_adapter(&db_type)?;
        
        // TODO: 实现 create_table 方法
        // 这需要在 DatabaseAdapter trait 中添加 create_table 方法
        let result = Ok(());
        
        self.pool_manager.release_connection(&connection).await?;
        
        if result.is_ok() {
            // 更新缓存
            {
                let mut cache = self.existence_cache.write().await;
                cache.insert(schema.name.clone(), true);
            }
            
            {
                let mut cache = self.schema_cache.write().await;
                cache.insert(schema.name.clone(), schema.clone());
            }
            
            // 注册版本
            {
                let mut version_manager = self.version_manager.write().await;
                version_manager.register_version(
                    schema.name.clone(),
                    schema.clone(),
                    Some("初始版本".to_string()),
                )?;
            }
            
            info!("成功创建表: {}", schema.name);
        }
        
        result
    }
    
    /// 删除表
    pub async fn drop_table(&self, table_name: &str) -> QuickDbResult<()> {
        let connection = self.pool_manager.get_connection(Some("default")).await?;
        let db_type = self.pool_manager.get_database_type("default")?;
        let adapter = crate::adapter::create_adapter(&db_type)?;
        
        // TODO: 实现 drop_table 方法
        // 这需要在 DatabaseAdapter trait 中添加 drop_table 方法
        let result = Ok(());
        
        self.pool_manager.release_connection(&connection).await?;
        
        if result.is_ok() {
            // 清除缓存
            {
                let mut cache = self.existence_cache.write().await;
                cache.remove(table_name);
            }
            
            {
                let mut cache = self.schema_cache.write().await;
                cache.remove(table_name);
            }
            
            info!("成功删除表: {}", table_name);
        }
        
        result
    }
    
    /// 获取表模式
    pub async fn get_table_schema(&self, table_name: &str) -> QuickDbResult<Option<TableSchema>> {
        // 先检查缓存
        {
            let cache = self.schema_cache.read().await;
            if let Some(schema) = cache.get(table_name) {
                return Ok(Some(schema.clone()));
            }
        }
        
        // 从数据库获取
        let connection = self.pool_manager.get_connection(Some("default")).await?;
        let db_type = self.pool_manager.get_database_type("default")?;
        let adapter = crate::adapter::create_adapter(&db_type)?;
        
        // TODO: 实现 get_table_schema 方法
        // 这需要在 DatabaseAdapter trait 中添加 get_table_schema 方法
        let schema: Option<TableSchema> = None;
        
        self.pool_manager.release_connection(&connection).await?;
        
        // 更新缓存
        if let Some(ref schema) = schema {
            let mut cache = self.schema_cache.write().await;
            cache.insert(table_name.to_string(), schema.clone());
        }
        
        Ok(schema)
    }
    
    /// 列出所有表
    pub async fn list_tables(&self) -> QuickDbResult<Vec<String>> {
        let mut connection = self.pool_manager.get_connection(Some("default")).await?;
        let db_type = self.pool_manager.get_database_type("default")?;
        let adapter = crate::adapter::create_adapter(&db_type)?;
        
        // TODO: 实现 list_tables 方法
        // 这需要在 DatabaseAdapter trait 中添加 list_tables 方法
        let tables = Vec::new();
        
        self.pool_manager.release_connection(&connection).await?;
        
        Ok(tables)
    }
    
    /// 检查表状态
    pub async fn check_table_status(&self, table_name: &str) -> QuickDbResult<TableCheckResult> {
        let exists = self.check_table_exists(table_name).await?;
        
        if !exists {
            return Ok(TableCheckResult {
                exists: false,
                current_version: None,
                expected_version: None,
                needs_migration: false,
                schema_diff: None,
            });
        }
        
        // 获取当前版本信息
        let version_manager = self.version_manager.read().await;
        let expected_version = version_manager.get_current_version(table_name)
            .map(|v| v.version);
        
        // TODO: 从数据库获取当前版本
        // 这需要一个版本追踪表来存储版本信息
        let current_version = None;
        
        let needs_migration = if let (Some(current), Some(expected)) = (current_version, expected_version) {
            current < expected
        } else {
            false
        };
        
        // TODO: 计算模式差异
        let schema_diff = None;
        
        Ok(TableCheckResult {
            exists,
            current_version,
            expected_version,
            needs_migration,
            schema_diff,
        })
    }
    
    /// 迁移表
    pub async fn migrate_table(
        &self,
        table_name: &str,
        target_version: Option<u32>,
    ) -> QuickDbResult<()> {
        // 确定目标版本
        let target_version = {
            let version_manager = self.version_manager.read().await;
            
            if let Some(version) = target_version {
                version
            } else {
                version_manager.get_current_version(table_name)
                    .map(|v| v.version)
                    .ok_or_else(|| QuickDbError::ValidationError {
                        field: "table_name".to_string(),
                        message: format!("表 {} 没有注册版本", table_name),
                    })?
            }
        };
        
        // TODO: 获取当前数据库版本
        let current_version = 0u32;
        
        if current_version == target_version {
            info!("表 {} 已是最新版本 {}", table_name, target_version);
            return Ok(());
        }
        
        // 获取迁移路径并执行
        {
            let version_manager_guard = self.version_manager.read().await;
            let migration_path = version_manager_guard.get_migration_path(
                table_name,
                current_version,
                target_version,
            )?;
            
            // 收集迁移ID以便后续执行
            let migration_ids: Vec<String> = migration_path.iter()
                .map(|m| m.id.clone())
                .collect();
            
            // 释放读锁
            drop(version_manager_guard);
            
            // 执行迁移
            for migration_id in migration_ids {
                info!("执行迁移: {}", migration_id);
                let mut version_manager = self.version_manager.write().await;
                version_manager.execute_migration(table_name, &migration_id).await?;
            }
        }
        
        info!("表 {} 迁移完成，版本: {}", table_name, target_version);
        
        Ok(())
    }
    
    /// 确保表存在（自动创建）
    pub async fn ensure_table_exists(&self, schema: &TableSchema) -> QuickDbResult<()> {
        if !self.auto_create_tables {
            return Ok(());
        }
        
        let exists = self.check_table_exists(&schema.name).await?;
        
        if !exists {
            info!("表 {} 不存在，自动创建", schema.name);
            self.create_table(schema, None).await?;
        } else if self.auto_migrate {
            // 检查是否需要迁移
            let status = self.check_table_status(&schema.name).await?;
            if status.needs_migration {
                info!("表 {} 需要迁移", schema.name);
                self.migrate_table(&schema.name, None).await?;
            }
        }
        
        Ok(())
    }
    
    /// 注册表模式
    pub async fn register_schema(
        &self,
        schema: TableSchema,
        description: Option<String>,
    ) -> QuickDbResult<u32> {
        let mut version_manager = self.version_manager.write().await;
        let version = version_manager.register_version(
            schema.name.clone(),
            schema.clone(),
            description,
        )?;
        
        // 更新缓存
        {
            let mut cache = self.schema_cache.write().await;
            cache.insert(schema.name.clone(), schema);
        }
        
        Ok(version)
    }
    
    /// 创建迁移脚本
    pub async fn create_migration_script(
        &self,
        table_name: &str,
        from_version: u32,
        to_version: u32,
        up_script: String,
        down_script: Option<String>,
        script_type: MigrationScriptType,
    ) -> QuickDbResult<String> {
        let mut version_manager = self.version_manager.write().await;
        version_manager.create_migration(
            table_name,
            from_version,
            to_version,
            up_script,
            down_script,
            script_type,
        )
    }
    
    /// 清除缓存
    pub async fn clear_cache(&self) {
        {
            let mut cache = self.existence_cache.write().await;
            cache.clear();
        }
        
        {
            let mut cache = self.schema_cache.write().await;
            cache.clear();
        }
        
        info!("表管理器缓存已清除");
    }
    
    /// 获取版本管理器
    pub fn get_version_manager(&self) -> Arc<RwLock<VersionManager>> {
        self.version_manager.clone()
    }
    
    /// 获取统计信息
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        {
            let existence_cache = self.existence_cache.read().await;
            stats.insert(
                "existence_cache_size".to_string(),
                serde_json::Value::Number(existence_cache.len().into()),
            );
        }
        
        {
            let schema_cache = self.schema_cache.read().await;
            stats.insert(
                "schema_cache_size".to_string(),
                serde_json::Value::Number(schema_cache.len().into()),
            );
        }
        
        stats.insert(
            "auto_create_tables".to_string(),
            serde_json::Value::Bool(self.auto_create_tables),
        );
        
        stats.insert(
            "auto_migrate".to_string(),
            serde_json::Value::Bool(self.auto_migrate),
        );
        
        stats
    }
}