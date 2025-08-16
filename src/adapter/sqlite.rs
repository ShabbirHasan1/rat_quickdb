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
use zerg_creep::{info, error, warn, debug};

use sqlx::{Row, sqlite::SqliteRow, Column};

/// SQLite适配器
pub struct SqliteAdapter;

impl SqliteAdapter {

    /// 将sqlx的行转换为JSON值
    fn row_to_json(&self, row: &SqliteRow) -> QuickDbResult<Value> {
        let mut map = serde_json::Map::new();
        
        for column in row.columns() {
            let column_name = column.name();
            
            // 尝试获取不同类型的值
            let json_value = if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                match value {
                    Some(s) => Value::String(s),
                    None => Value::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<i64>, _>(column_name) {
                match value {
                    Some(i) => Value::Number(i.into()),
                    None => Value::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<f64>, _>(column_name) {
                match value {
                    Some(f) => Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into())),
                    None => Value::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<bool>, _>(column_name) {
                match value {
                    Some(b) => Value::Bool(b),
                    None => Value::Null,
                }
            } else if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(column_name) {
                match value {
                    Some(bytes) => Value::String(base64::encode(bytes)),
                    None => Value::Null,
                }
            } else {
                Value::Null
            };
            
            map.insert(column_name.to_string(), json_value);
        }
        
        Ok(Value::Object(map))
    }
}

#[async_trait]
impl DatabaseAdapter for SqliteAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<Value> {
        let pool = match connection {
            DatabaseConnection::SQLite(pool) => pool,
            _ => return Err(QuickDbError::ConnectionError {
                message: "Invalid connection type for SQLite".to_string(),
            }),
        };
        {
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
            
            let (sql, _params) = SqlQueryBuilder::new()
                .insert(data.clone())
                .from(table)
                .build()?;
            
            // 构建参数化查询
            let mut query = sqlx::query(&sql);
            for value in data.values() {
                match value {
                    DataValue::String(s) => { query = query.bind(s); },
                    DataValue::Int(i) => { query = query.bind(i); },
                    DataValue::Float(f) => { query = query.bind(f); },
                    DataValue::Bool(b) => { query = query.bind(b); },
                    DataValue::Bytes(bytes) => { query = query.bind(bytes); },
                    DataValue::DateTime(dt) => { query = query.bind(dt.to_rfc3339()); },
                    DataValue::Uuid(uuid) => { query = query.bind(uuid.to_string()); },
                    DataValue::Json(json) => { query = query.bind(json.to_string()); },
                    DataValue::Array(arr) => {
                        let json = serde_json::to_string(arr).unwrap_or_default();
                        query = query.bind(json);
                    },
                    DataValue::Object(obj) => {
                        let json = serde_json::to_string(obj).unwrap_or_default();
                        query = query.bind(json);
                    },
                    DataValue::Null => { query = query.bind(Option::<String>::None); },
                }
            }
            
            let result = query.execute(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite插入失败: {}", e),
                })?;
            
            Ok(serde_json::json!({
                "id": result.last_insert_rowid(),
                "affected_rows": result.rows_affected()
            }))
        }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<Value>> {
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
                Some(r) => Ok(Some(self.row_to_json(&r)?)),
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
    ) -> QuickDbResult<Vec<Value>> {
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
                .where_conditions(conditions)
                .limit(options.pagination.as_ref().map(|p| p.limit).unwrap_or(1000))
                .offset(options.pagination.as_ref().map(|p| p.skip).unwrap_or(0))
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
            
            let rows = query.fetch_all(pool).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行SQLite查询失败: {}", e),
                })?;
            
            let mut results = Vec::new();
            for row in rows {
                results.push(self.row_to_json(&row)?);
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
                    FieldType::String { .. } => "TEXT",
                    FieldType::Integer { .. } => "INTEGER",
                    FieldType::Float { .. } => "REAL",
                    FieldType::Boolean => "INTEGER",
                    FieldType::DateTime => "TEXT",
                    FieldType::Json => "TEXT",
                    FieldType::Uuid => "TEXT",
                    FieldType::Array { .. } => "TEXT", // 存储为JSON
                    FieldType::Object { .. } => "TEXT", // 存储为JSON
                    FieldType::Reference { .. } => "TEXT", // 存储引用ID
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
}