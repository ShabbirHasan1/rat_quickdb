//! 表版本管理
//! 
//! 提供表模式版本控制和迁移功能

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{QuickDbResult, QuickDbError};
use super::schema::TableSchema;

/// 模式版本信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaVersion {
    /// 表名
    pub table_name: String,
    /// 版本号
    pub version: u32,
    /// 模式定义
    pub schema: TableSchema,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 迁移脚本
    pub migration_script: Option<MigrationScript>,
    /// 版本描述
    pub description: Option<String>,
    /// 是否为当前版本
    pub is_current: bool,
}

/// 迁移脚本
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationScript {
    /// 脚本ID
    pub id: String,
    /// 源版本
    pub from_version: u32,
    /// 目标版本
    pub to_version: u32,
    /// 升级脚本
    pub up_script: String,
    /// 降级脚本
    pub down_script: Option<String>,
    /// 脚本类型
    pub script_type: MigrationScriptType,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 执行时间
    pub executed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 执行状态
    pub status: MigrationStatus,
}

/// 迁移脚本类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationScriptType {
    /// DDL脚本（结构变更）
    Ddl,
    /// DML脚本（数据变更）
    Dml,
    /// 混合脚本
    Mixed,
    /// 自定义脚本
    Custom,
}

/// 迁移状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 执行成功
    Success,
    /// 执行失败
    Failed,
    /// 已回滚
    Rollback,
}

/// 版本管理器
#[derive(Debug)]
pub struct VersionManager {
    /// 版本存储
    versions: HashMap<String, Vec<SchemaVersion>>,
    /// 迁移历史
    migration_history: Vec<MigrationRecord>,
}

/// 迁移记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// 记录ID
    pub id: String,
    /// 表名
    pub table_name: String,
    /// 迁移脚本ID
    pub migration_id: String,
    /// 源版本
    pub from_version: u32,
    /// 目标版本
    pub to_version: u32,
    /// 执行开始时间
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// 执行结束时间
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 执行状态
    pub status: MigrationStatus,
    /// 错误信息
    pub error_message: Option<String>,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
}

impl VersionManager {
    /// 创建新的版本管理器
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
            migration_history: Vec::new(),
        }
    }
    
    /// 注册表的新版本
    pub fn register_version(
        &mut self,
        table_name: String,
        schema: TableSchema,
        description: Option<String>,
    ) -> QuickDbResult<u32> {
        let versions = self.versions.entry(table_name.clone()).or_insert_with(Vec::new);
        
        // 计算新版本号
        let new_version = versions.iter().map(|v| v.version).max().unwrap_or(0) + 1;
        
        // 将之前的版本标记为非当前版本
        for version in versions.iter_mut() {
            version.is_current = false;
        }
        
        // 创建新版本
        let schema_version = SchemaVersion {
            table_name: table_name.clone(),
            version: new_version,
            schema,
            created_at: chrono::Utc::now(),
            migration_script: None,
            description,
            is_current: true,
        };
        
        versions.push(schema_version);
        
        Ok(new_version)
    }
    
    /// 获取表的当前版本
    pub fn get_current_version(&self, table_name: &str) -> Option<&SchemaVersion> {
        self.versions
            .get(table_name)?
            .iter()
            .find(|v| v.is_current)
    }
    
    /// 获取表的指定版本
    pub fn get_version(&self, table_name: &str, version: u32) -> Option<&SchemaVersion> {
        self.versions
            .get(table_name)?
            .iter()
            .find(|v| v.version == version)
    }
    
    /// 获取表的所有版本
    pub fn get_all_versions(&self, table_name: &str) -> Option<&Vec<SchemaVersion>> {
        self.versions.get(table_name)
    }
    
    /// 获取表的版本历史
    pub fn get_version_history(&self, table_name: &str) -> Vec<&SchemaVersion> {
        if let Some(versions) = self.versions.get(table_name) {
            let mut sorted_versions: Vec<&SchemaVersion> = versions.iter().collect();
            sorted_versions.sort_by(|a, b| a.version.cmp(&b.version));
            sorted_versions
        } else {
            Vec::new()
        }
    }
    
    /// 创建迁移脚本
    pub fn create_migration(
        &mut self,
        table_name: &str,
        from_version: u32,
        to_version: u32,
        up_script: String,
        down_script: Option<String>,
        script_type: MigrationScriptType,
    ) -> QuickDbResult<String> {
        // 验证版本存在
        if self.get_version(table_name, from_version).is_none() {
            return Err(QuickDbError::ValidationError {
                field: "from_version".to_string(),
                message: format!("源版本 {} 不存在", from_version),
            });
        }
        
        if self.get_version(table_name, to_version).is_none() {
            return Err(QuickDbError::ValidationError {
                field: "to_version".to_string(),
                message: format!("目标版本 {} 不存在", to_version),
            });
        }
        
        let migration_id = format!(
            "{}_{}_to_{}_{}_{}",
            table_name,
            from_version,
            to_version,
            chrono::Utc::now().timestamp(),
            uuid::Uuid::new_v4().to_string()[..8].to_string()
        );
        
        let migration = MigrationScript {
            id: migration_id.clone(),
            from_version,
            to_version,
            up_script,
            down_script,
            script_type,
            created_at: chrono::Utc::now(),
            executed_at: None,
            status: MigrationStatus::Pending,
        };
        
        // 将迁移脚本添加到对应版本
        if let Some(versions) = self.versions.get_mut(table_name) {
            if let Some(version) = versions.iter_mut().find(|v| v.version == to_version) {
                version.migration_script = Some(migration);
            }
        }
        
        Ok(migration_id)
    }
    
    /// 执行迁移
    pub async fn execute_migration(
        &mut self,
        table_name: &str,
        migration_id: &str,
    ) -> QuickDbResult<()> {
        // 先获取迁移信息，避免借用冲突
        let (from_version, to_version) = {
            let migration = self.find_migration_mut(table_name, migration_id)
                .ok_or_else(|| QuickDbError::ValidationError {
                    field: "migration_id".to_string(),
                    message: format!("迁移脚本 {} 不存在", migration_id),
                })?;
            
            // 检查迁移状态
            if migration.status != MigrationStatus::Pending {
                return Err(QuickDbError::ValidationError {
                    field: "migration_status".to_string(),
                    message: format!("迁移脚本 {} 状态不正确: {:?}", migration_id, migration.status),
                });
            }
            
            let start_time = chrono::Utc::now();
            migration.status = MigrationStatus::Running;
            
            (migration.from_version, migration.to_version)
        };
        
        let start_time = chrono::Utc::now();
        
        // 创建迁移记录
        let record = MigrationRecord {
            id: uuid::Uuid::new_v4().to_string(),
            table_name: table_name.to_string(),
            migration_id: migration_id.to_string(),
            from_version,
            to_version,
            started_at: start_time,
            completed_at: None,
            status: MigrationStatus::Running,
            error_message: None,
            duration_ms: None,
        };
        
        self.migration_history.push(record);
        
        // TODO: 实际执行迁移脚本
        // 这里需要与数据库适配器集成
        
        let end_time = chrono::Utc::now();
        let duration = end_time.signed_duration_since(start_time);
        
        // 更新迁移状态
        if let Some(migration) = self.find_migration_mut(table_name, migration_id) {
            migration.status = MigrationStatus::Success;
            migration.executed_at = Some(end_time);
        }
        
        // 更新迁移记录
        if let Some(record) = self.migration_history.last_mut() {
            record.completed_at = Some(end_time);
            record.status = MigrationStatus::Success;
            record.duration_ms = Some(duration.num_milliseconds() as u64);
        }
        
        Ok(())
    }
    
    /// 回滚迁移
    pub async fn rollback_migration(
        &mut self,
        table_name: &str,
        migration_id: &str,
    ) -> QuickDbResult<()> {
        // 先获取迁移信息，避免借用冲突
        let (from_version, to_version, down_script) = {
            let migration = self.find_migration_mut(table_name, migration_id)
                .ok_or_else(|| QuickDbError::ValidationError {
                    field: "migration_id".to_string(),
                    message: format!("迁移脚本 {} 不存在", migration_id),
                })?;
            
            // 检查是否有回滚脚本
            let down_script = migration.down_script.clone()
                .ok_or_else(|| QuickDbError::ValidationError {
                    field: "down_script".to_string(),
                    message: format!("迁移脚本 {} 没有回滚脚本", migration_id),
                })?;
            
            // 检查迁移状态
            if migration.status != MigrationStatus::Success {
                return Err(QuickDbError::ValidationError {
                    field: "migration_status".to_string(),
                    message: format!("迁移脚本 {} 状态不正确，无法回滚: {:?}", migration_id, migration.status),
                });
            }
            
            (migration.to_version, migration.from_version, down_script)
        };
        
        let start_time = chrono::Utc::now();
        
        // 创建回滚记录
        let record = MigrationRecord {
            id: uuid::Uuid::new_v4().to_string(),
            table_name: table_name.to_string(),
            migration_id: format!("{}_rollback", migration_id),
            from_version,
            to_version,
            started_at: start_time,
            completed_at: None,
            status: MigrationStatus::Running,
            error_message: None,
            duration_ms: None,
        };
        
        self.migration_history.push(record);
        
        // TODO: 实际执行回滚脚本
        // 这里需要与数据库适配器集成
        
        let end_time = chrono::Utc::now();
        let duration = end_time.signed_duration_since(start_time);
        
        // 更新迁移状态
        if let Some(migration) = self.find_migration_mut(table_name, migration_id) {
            migration.status = MigrationStatus::Rollback;
        }
        
        // 更新迁移记录
        if let Some(record) = self.migration_history.last_mut() {
            record.completed_at = Some(end_time);
            record.status = MigrationStatus::Success;
            record.duration_ms = Some(duration.num_milliseconds() as u64);
        }
        
        Ok(())
    }
    
    /// 获取迁移历史
    pub fn get_migration_history(&self, table_name: Option<&str>) -> Vec<&MigrationRecord> {
        if let Some(table) = table_name {
            self.migration_history
                .iter()
                .filter(|record| record.table_name == table)
                .collect()
        } else {
            self.migration_history.iter().collect()
        }
    }
    
    /// 检查是否需要迁移
    pub fn needs_migration(&self, table_name: &str, current_db_version: u32) -> bool {
        if let Some(current_version) = self.get_current_version(table_name) {
            current_version.version > current_db_version
        } else {
            false
        }
    }
    
    /// 获取迁移路径
    pub fn get_migration_path(
        &self,
        table_name: &str,
        from_version: u32,
        to_version: u32,
    ) -> QuickDbResult<Vec<&MigrationScript>> {
        if from_version == to_version {
            return Ok(Vec::new());
        }
        
        let versions = self.versions.get(table_name)
            .ok_or_else(|| QuickDbError::ValidationError {
                field: "table_name".to_string(),
                message: format!("表 {} 不存在", table_name),
            })?;
        
        let mut path = Vec::new();
        
        if from_version < to_version {
            // 升级路径
            for version in (from_version + 1)..=to_version {
                if let Some(schema_version) = versions.iter().find(|v| v.version == version) {
                    if let Some(migration) = &schema_version.migration_script {
                        path.push(migration);
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "migration_script".to_string(),
                            message: format!("版本 {} 缺少迁移脚本", version),
                        });
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "version".to_string(),
                        message: format!("版本 {} 不存在", version),
                    });
                }
            }
        } else {
            // 降级路径
            for version in (to_version + 1..=from_version).rev() {
                if let Some(schema_version) = versions.iter().find(|v| v.version == version) {
                    if let Some(migration) = &schema_version.migration_script {
                        if migration.down_script.is_some() {
                            path.push(migration);
                        } else {
                            return Err(QuickDbError::ValidationError {
                                field: "down_script".to_string(),
                                message: format!("版本 {} 缺少回滚脚本", version),
                            });
                        }
                    } else {
                        return Err(QuickDbError::ValidationError {
                            field: "migration_script".to_string(),
                            message: format!("版本 {} 缺少迁移脚本", version),
                        });
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "version".to_string(),
                        message: format!("版本 {} 不存在", version),
                    });
                }
            }
        }
        
        Ok(path)
    }
    
    /// 查找迁移脚本（可变引用）
    fn find_migration_mut(
        &mut self,
        table_name: &str,
        migration_id: &str,
    ) -> Option<&mut MigrationScript> {
        self.versions
            .get_mut(table_name)?
            .iter_mut()
            .find_map(|v| {
                if let Some(migration) = &mut v.migration_script {
                    if migration.id == migration_id {
                        Some(migration)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    }
    
    /// 清理历史记录
    pub fn cleanup_history(&mut self, keep_days: u32) {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(keep_days as i64);
        
        self.migration_history.retain(|record| {
            record.started_at > cutoff_date
        });
    }
    
    /// 导出版本信息
    pub fn export_versions(&self) -> QuickDbResult<String> {
        serde_json::to_string_pretty(&self.versions)
            .map_err(|e| QuickDbError::SerializationError {
                message: e.to_string(),
            })
    }
    
    /// 导入版本信息
    pub fn import_versions(&mut self, data: &str) -> QuickDbResult<()> {
        let versions: HashMap<String, Vec<SchemaVersion>> = serde_json::from_str(data)
            .map_err(|e| QuickDbError::SerializationError {
                message: e.to_string(),
            })?;
        
        self.versions = versions;
        Ok(())
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}