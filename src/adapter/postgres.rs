//! PostgreSQL数据库适配器
//! 
//! 使用tokio-postgres库实现真实的PostgreSQL数据库操作

use super::{DatabaseAdapter, SqlQueryBuilder};
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::pool::DatabaseConnection;
use crate::table::{TableManager, TableSchema, ColumnType};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use zerg_creep::{info, error, warn, debug};
#[cfg(feature = "postgresql")]
use tokio_postgres::{Row, types::ToSql};
// 移除不存在的zerg_creep::prelude导入

/// PostgreSQL适配器
#[cfg(feature = "postgresql")]
pub struct PostgresAdapter;

impl PostgresAdapter {
    /// 将DataValue转换为PostgreSQL参数
    fn data_value_to_param(&self, value: &DataValue) -> Box<dyn ToSql + Sync + Send> {
        match value {
            DataValue::String(s) => Box::new(s.clone()),
            DataValue::Int(i) => Box::new(*i),
            DataValue::Float(f) => Box::new(*f),
            DataValue::Bool(b) => Box::new(*b),
            DataValue::DateTime(dt) => Box::new(dt.clone()),
            DataValue::Uuid(uuid) => Box::new(*uuid),
            DataValue::Json(json) => Box::new(json.clone()),
            DataValue::Array(arr) => {
                let json = serde_json::to_value(arr).unwrap_or(Value::Null);
                Box::new(json)
            },
            DataValue::Object(obj) => {
                let json = serde_json::to_value(obj).unwrap_or(Value::Null);
                Box::new(json)
            },
            // Reference类型不存在，移除此行
            DataValue::Null => Box::new(Option::<String>::None),
        }
    }

    /// 将PostgreSQL行转换为JSON值
    fn row_to_json(&self, row: &Row) -> QuickDbResult<Value> {
        let mut map = serde_json::Map::new();
        
        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();
            
            // 根据PostgreSQL类型转换值
            let json_value = match column.type_().name() {
                "int4" | "int8" => {
                    if let Ok(val) = row.try_get::<_, Option<i64>>(i) {
                        val.map(|v| Value::Number(v.into())).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                },
                "float4" | "float8" => {
                    if let Ok(val) = row.try_get::<_, Option<f64>>(i) {
                        val.and_then(|v| serde_json::Number::from_f64(v))
                            .map(Value::Number)
                            .unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                },
                "bool" => {
                    if let Ok(val) = row.try_get::<_, Option<bool>>(i) {
                        val.map(Value::Bool).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                },
                "text" | "varchar" | "char" => {
                    if let Ok(val) = row.try_get::<_, Option<String>>(i) {
                        val.map(Value::String).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                },
                "uuid" => {
                    if let Ok(val) = row.try_get::<_, Option<uuid::Uuid>>(i) {
                        val.map(|u| Value::String(u.to_string())).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                },
                "json" | "jsonb" => {
                    if let Ok(val) = row.try_get::<_, Option<Value>>(i) {
                        val.unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                },
                "timestamp" | "timestamptz" => {
                    if let Ok(val) = row.try_get::<_, Option<chrono::DateTime<chrono::Utc>>>(i) {
                        val.map(|dt| Value::String(dt.to_rfc3339())).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                },
                _ => {
                    // 对于未知类型，尝试作为字符串获取
                    if let Ok(val) = row.try_get::<_, Option<String>>(i) {
                        val.map(Value::String).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
            };
            
            map.insert(column_name.to_string(), json_value);
        }
        
        Ok(Value::Object(map))
    }

    /// 执行查询并返回结果
    async fn execute_query(
        &self,
        client: &tokio_postgres::Client,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<Vec<Value>> {
        let pg_params: Vec<Box<dyn ToSql + Sync + Send>> = params
            .iter()
            .map(|p| self.data_value_to_param(p))
            .collect();
        
        let param_refs: Vec<&(dyn ToSql + Sync)> = pg_params
            .iter()
            .map(|p| p.as_ref())
            .collect();
        
        let rows = client.query(sql, &param_refs)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行PostgreSQL查询失败: {}", e),
            })?;
        
        let mut results = Vec::new();
        for row in rows {
            let value = self.row_to_json(&row)?;
            results.push(value);
        }
        
        Ok(results)
    }

    /// 执行更新操作
    async fn execute_update(
        &self,
        client: &tokio_postgres::Client,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<u64> {
        let pg_params: Vec<Box<dyn ToSql + Sync + Send>> = params
            .iter()
            .map(|p| self.data_value_to_param(p))
            .collect();
        
        let param_refs: Vec<&(dyn ToSql + Sync)> = pg_params
            .iter()
            .map(|p| p.as_ref())
            .collect();
        
        let affected_rows = client.execute(sql, &param_refs)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行PostgreSQL更新失败: {}", e),
            })?;
        
        Ok(affected_rows)
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<Value> {
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
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
                info!("自动创建PostgreSQL表 '{}' 成功", table);
            }
            
            let (sql, params) = SqlQueryBuilder::new()
                .insert(data.clone())
                .from(table)
                .returning(&["id"])
                .build()?;
            
            debug!("执行PostgreSQL插入: {}", sql);
            
            let results = self.execute_query(client, &sql, &params).await?;
            
            if let Some(result) = results.first() {
                Ok(result.clone())
            } else {
                Ok(serde_json::json!({
                    "affected_rows": 1
                }))
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<Value>> {
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .select(&["*"])
                .from(table)
                .where_condition(condition)
                .limit(1)
                .build()?;
            
            debug!("执行PostgreSQL根据ID查询: {}", sql);
            
            let results = self.execute_query(client, &sql, &params).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn find(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<Value>> {
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let mut builder = SqlQueryBuilder::new()
                .select(&["*"])
                .from(table)
                .where_conditions(conditions);
            
            // 添加排序
            if let Some(sort) = &options.sort {
                for sort_field in sort {
                    builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
                }
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.offset);
            }
            
            let (sql, params) = builder.build()?;
            
            debug!("执行PostgreSQL查询: {}", sql);
            
            self.execute_query(client, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            debug!("执行PostgreSQL更新: {}", sql);
            
            self.execute_update(client, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn update_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<bool> {
        let conditions = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        }];
        
        let affected = self.update(connection, table, &conditions, data).await?;
        Ok(affected > 0)
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .delete()
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            debug!("执行PostgreSQL删除: {}", sql);
            
            self.execute_update(client, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        let conditions = vec![QueryCondition {
            field: "id".to_string(),
            operator: QueryOperator::Eq,
            value: id.clone(),
        }];
        
        let affected = self.delete(connection, table, &conditions).await?;
        Ok(affected > 0)
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            debug!("执行PostgreSQL计数: {}", sql);
            
            let results = self.execute_query(client, &sql, &params).await?;
            if let Some(result) = results.first() {
                if let Some(count) = result.get("count") {
                    if let Some(count_num) = count.as_u64() {
                        return Ok(count_num);
                    }
                }
            }
            
            Ok(0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
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
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let mut field_definitions = Vec::new();
            
            // 检查是否已经有id字段，如果没有则添加默认的id主键
            if !fields.contains_key("id") {
                field_definitions.push("id SERIAL PRIMARY KEY".to_string());
            }
            
            for (name, field_type) in fields {
                let sql_type = match field_type {
                    FieldType::String => "TEXT",
                    FieldType::Integer => "BIGINT",
                    FieldType::Float => "DOUBLE PRECISION",
                    FieldType::Boolean => "BOOLEAN",
                    FieldType::DateTime => "TIMESTAMPTZ",
                    FieldType::Uuid => "UUID",
                    FieldType::Json => "JSONB",
                    FieldType::Array(_) => "JSONB",
                    FieldType::Object => "JSONB",
                    FieldType::Reference(_) => "TEXT",
                };
                
                // 如果是id字段，添加主键约束
                if name == "id" {
                    field_definitions.push(format!("{} {} PRIMARY KEY", name, sql_type));
                } else {
                    field_definitions.push(format!("{} {}", name, sql_type));
                }
            }
            
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                table,
                field_definitions.join(", ")
            );
            
            debug!("执行PostgreSQL建表: {}", sql);
            
            self.execute_update(client, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
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
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let unique_clause = if unique { "UNIQUE " } else { "" };
            let sql = format!(
                "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
                unique_clause,
                index_name,
                table,
                fields.join(", ")
            );
            
            debug!("执行PostgreSQL索引创建: {}", sql);
            
            self.execute_update(client, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::PostgreSQL(ref client) = connection {
            let sql = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1";
            let pg_params: Vec<Box<dyn ToSql + Sync + Send>> = vec![Box::new(table.to_string())];
            let param_refs: Vec<&(dyn ToSql + Sync)> = pg_params
                .iter()
                .map(|p| p.as_ref())
                .collect();
            
            let rows = client.query(sql, &param_refs)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("检查PostgreSQL表是否存在失败: {}", e),
                })?;
            
            Ok(!rows.is_empty())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }
}