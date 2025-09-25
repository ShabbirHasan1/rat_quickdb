//! SQLite数据库适配器
//! 
//! 使用sqlx库实现真实的SQLite数据库操作

use super::{DatabaseAdapter, SqlQueryBuilder};
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::model::FieldType;
use crate::pool::{DatabaseConnection};
use crate::table::{TableManager, TableSchema, ColumnType};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use rat_logger::{info, error, warn, debug};

use sqlx::{Row, sqlite::SqliteRow, Column};

/// SQLite适配器
pub struct SqliteAdapter;

impl SqliteAdapter {

    /// 将sqlx的行转换为DataValue映射
    fn row_to_data_map(&self, row: &SqliteRow) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut map = HashMap::new();
        
        for column in row.columns() {
            let column_name = column.name();
            
            // 尝试获取不同类型的值
            let data_value = if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                // 使用通用的JSON字符串检测和反序列化方法
                match value {
                    Some(s) => crate::types::parse_json_string_to_data_value(s),
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<i64>, _>(column_name) {
                match value {
                    Some(i) => {
                        // 检查是否可能是boolean值（SQLite中boolean存储为0或1）
                        // 只对已知的boolean字段进行转换，避免误判其他integer字段
                        if matches!(column_name, "is_active" | "active" | "enabled" | "disabled" | "verified" | "is_admin" | "is_deleted")
                           && (i == 0 || i == 1) {
                            DataValue::Bool(i == 1)
                        } else {
                            DataValue::Int(i)
                        }
                    },
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<f64>, _>(column_name) {
                match value {
                    Some(f) => DataValue::Float(f),
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<bool>, _>(column_name) {
                match value {
                    Some(b) => DataValue::Bool(b),
                    None => DataValue::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(column_name) {
                match value {
                    Some(bytes) => DataValue::Bytes(bytes),
                    None => DataValue::Null,
                }
            } else {
                DataValue::Null
            };
            
            map.insert(column_name.to_string(), data_value);
        }
        
        Ok(map)
    }
}

#[async_trait]
impl DatabaseAdapter for SqliteAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<DataValue> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        
        // 自动建表逻辑：检查表是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                info!("表 {} 不存在，正在自动创建", table);
                let schema = TableSchema::infer_from_data(table.to_string(), data);
                // 将 ColumnDefinition 转换为 HashMap<String, FieldType>
                    let fields: HashMap<String, FieldType> = schema.columns.iter()
                        .map(|col| {
                            let field_type = match &col.column_type {
                                ColumnType::String { .. } => FieldType::String { max_length: None, min_length: None, regex: None },
                                ColumnType::Text | ColumnType::LongText => FieldType::String { max_length: None, min_length: None, regex: None },
                                ColumnType::Integer | ColumnType::SmallInteger => FieldType::Integer { min_value: None, max_value: None },
                                ColumnType::BigInteger => FieldType::Integer { min_value: None, max_value: None },
                                ColumnType::Float | ColumnType::Double => FieldType::Float { min_value: None, max_value: None },
                                ColumnType::Boolean => FieldType::Boolean,
                                ColumnType::DateTime | ColumnType::Date | ColumnType::Time | ColumnType::Timestamp => FieldType::DateTime,
                                ColumnType::Uuid => FieldType::Uuid,
                                ColumnType::Json => FieldType::Json,
                                _ => FieldType::String { max_length: None, min_length: None, regex: None }, // 默认为字符串
                            };
                            (col.name.clone(), field_type)
                        })
                        .collect();
                self.create_table(connection, table, &fields).await?;
                info!("自动创建SQLite表 '{}' 成功", table);
            }
            
            let (sql, params) = SqlQueryBuilder::new()
                .insert(data.clone())
                .from(table)
                .build()?;
            
            // 构建参数化查询，使用正确的参数顺序
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::Bytes(bytes) => { query = query.bind(bytes); },
                    DataValue::DateTime(dt) => { query = query.bind(dt.to_rfc3339()); },
                    DataValue::Uuid(uuid) => { query = query.bind(uuid.to_string()); },
                    DataValue::Json(json) => { query = query.bind(json.to_string()); },
                    DataValue::Array(_) => {
                        let json = param.to_json_value().to_string();
                        query = query.bind(json);
                    },
                    DataValue::Object(_) => {
                        let json = param.to_json_value().to_string();
                        query = query.bind(json);
                    },
                    DataValue::Null => { query = query.bind(Option::<String>::None); },
                }
            }
            
            let result = query.execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite插入失败: {}", e),
                })?;
            
            // 根据插入的数据返回相应的ID
            // 优先返回数据中的ID字段，如果没有则使用SQLite的rowid
            if let Some(id_value) = data.get("id") {
                Ok(id_value.clone())
            } else if let Some(id_value) = data.get("_id") {
                Ok(id_value.clone())
            } else {
                // 如果数据中没有ID字段，返回SQLite的自增ID
                let id = result.last_insert_rowid();
                if id > 0 {
                    Ok(DataValue::Int(id))
                } else {
                    // 如果没有自增ID，返回包含详细信息的对象
                    let mut result_map = HashMap::new();
                    result_map.insert("id".to_string(), DataValue::Int(id));
                    result_map.insert("affected_rows".to_string(), DataValue::Int(result.rows_affected() as i64));
                    Ok(DataValue::Object(result_map))
                }
            }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let sql = format!("SELECT * FROM {} WHERE id = ? LIMIT 1", table);
            
            let mut query = sqlx::query(&sql);
            match id {
                DataValue::String(s) => { query = query.bind(s); },
                DataValue::Int(i) => { query = query.bind(i); },
                _ => { query = query.bind(id.to_string()); },
            }
            
            let row = query.fetch_optional(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite根据ID查询失败: {}", e),
                })?;
            
            match row {
                Some(r) => {
                    let data_map = self.row_to_data_map(&r)?;
                    Ok(Some(DataValue::Object(data_map)))
                },
                None => Ok(None),
            }
        }
    }

    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        // 将简单条件转换为条件组合（AND逻辑）
        let condition_groups = if conditions.is_empty() {
            vec![]
        } else {
            let group_conditions = conditions.iter()
                .map(|c| QueryConditionGroup::Single(c.clone()))
                .collect();
            vec![QueryConditionGroup::Group {
                operator: LogicalOperator::And,
                conditions: group_conditions,
            }]
        };
        
        // 统一使用 find_with_groups 实现
        self.find_with_groups(connection, table, &condition_groups, options).await
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let (sql, params) = SqlQueryBuilder::new()
                .select(&["*"])
                .from(table)
                .where_condition_groups(condition_groups)
                .limit(options.pagination.as_ref().map(|p| p.limit).unwrap_or(1000))
                .offset(options.pagination.as_ref().map(|p| p.skip).unwrap_or(0))
                .build()?;
            
            debug!("执行SQLite条件组合查询: {}", sql);
            
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::Null => { query = query.bind(Option::<String>::None); },
                    _ => { query = query.bind(param.to_string()); },
                }
            }
            
            let rows = query.fetch_all(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite条件组合查询失败: {}", e),
                })?;
            
            let mut results = Vec::new();
            for row in rows {
                let data_map = self.row_to_data_map(&row)?;
                results.push(DataValue::Object(data_map));
            }
            
            Ok(results)
        }
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let (sql, params) = SqlQueryBuilder::new()
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    _ => { query = query.bind(param.to_string()); },
                }
            }
            
            let result = query.execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite更新失败: {}", e),
                })?;
            
            Ok(result.rows_affected())
        }
    }

    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        let condition = QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        };
        
        let affected_rows = self.update(connection, table, &[condition], data).await?;
        Ok(affected_rows > 0)
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let (sql, params) = SqlQueryBuilder::new()
                .delete()
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    _ => { query = query.bind(param.to_string()); },
                }
            }
            
            let result = query.execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite删除失败: {}", e),
                })?;
            
            Ok(result.rows_affected())
        }
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        let condition = QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        };
        
        let affected_rows = self.delete(connection, table, &[condition]).await?;
        Ok(affected_rows > 0)
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let (sql, params) = SqlQueryBuilder::new()
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let mut query = sqlx::query(&sql);
            for param in &params {
                match param {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    _ => { query = query.bind(param.to_string()); },
                }
            }
            
            let row = query.fetch_one(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite统计失败: {}", e),
                })?;
            
            let count: i64 = row.try_get("count")
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("获取统计结果失败: {}", e),
                })?;
            
            Ok(count as u64)
        }
    }

    async fn exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<bool> {
        let count = self.count(connection, table, conditions).await?;
        Ok(count > 0)
    }

    async fn create_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        fields: &HashMap<String, FieldType>,
    ) -> QuickDbResult<()> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (", table);
            let mut has_fields = false;
            
            // 检查是否已经有id字段，如果没有则添加默认的id主键
            if !fields.contains_key("id") {
                sql.push_str("id INTEGER PRIMARY KEY AUTOINCREMENT");
                has_fields = true;
            }
            
            for (field_name, field_type) in fields {
                if has_fields {
                    sql.push_str(", ");
                }
                
                let sql_type = match field_type {
                    FieldType::String { max_length, .. } => {
                        if let Some(max_len) = max_length {
                            format!("VARCHAR({})", max_len)
                        } else {
                            "TEXT".to_string()
                        }
                    },
                    FieldType::Integer { .. } => "INTEGER".to_string(),
                    FieldType::BigInteger => "INTEGER".to_string(), // SQLite只有INTEGER类型
                    FieldType::Float { .. } => "REAL".to_string(),
                    FieldType::Double => "REAL".to_string(), // SQLite只有REAL类型
                    FieldType::Text => "TEXT".to_string(),
                    FieldType::Boolean => "INTEGER".to_string(),
                    FieldType::DateTime => "TEXT".to_string(),
                    FieldType::Date => "TEXT".to_string(),
                    FieldType::Time => "TEXT".to_string(),
                    FieldType::Json => "TEXT".to_string(),
                    FieldType::Uuid => "TEXT".to_string(),
                    FieldType::Binary => "BLOB".to_string(),
                    FieldType::Decimal { precision: _, scale: _ } => "REAL".to_string(), // SQLite没有DECIMAL，使用REAL
                    FieldType::Array { .. } => "TEXT".to_string(), // 存储为JSON
                    FieldType::Object { .. } => "TEXT".to_string(), // 存储为JSON
                    FieldType::Reference { .. } => "TEXT".to_string(), // 存储引用ID
                };
                
                // 如果是id字段，添加主键约束
                if field_name == "id" {
                    sql.push_str(&format!("{} {} PRIMARY KEY", field_name, sql_type));
                } else {
                    sql.push_str(&format!("{} {}", field_name, sql_type));
                }
                has_fields = true;
            }
            
            sql.push(')');
            
            sqlx::query(&sql).execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("创建SQLite表失败: {}", e),
                })?;
            
            Ok(())
        }
    }

    async fn create_index(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        index_name: &str,
        fields: &[String],
        unique: bool,
    ) -> QuickDbResult<()> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let unique_keyword = if unique { "UNIQUE " } else { "" };
            let fields_str = fields.join(", ");
            let sql = format!(
                "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
                unique_keyword, index_name, table, fields_str
            );
            
            sqlx::query(&sql).execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("创建SQLite索引失败: {}", e),
                })?;
            
            Ok(())
        }
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
            let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
            let row = sqlx::query(sql)
                .bind(table)
                .fetch_optional(pool)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("检查SQLite表是否存在失败: {}", e),
                })?;
            
            Ok(row.is_some())
        }
    }

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        
        let sql = format!("DROP TABLE IF EXISTS {}", table);
        
        debug!("执行SQLite删除表SQL: {}", sql);
        
        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("删除SQLite表失败: {}", e),
            })?;
        
        info!("成功删除SQLite表: {}", table);
        Ok(())
    }
}