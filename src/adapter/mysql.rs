//! MySQL数据库适配器
//!
//! 基于mysql_async库实现的MySQL数据库适配器，提供完整的CRUD操作支持

use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::{DataValue, QueryCondition, QueryOperator, QueryOptions, SortDirection};
use crate::adapter::query_builder::SqlQueryBuilder;
use crate::table::{TableManager, TableSchema, ColumnType};
use crate::model::FieldType;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use zerg_creep::{info, error, warn, debug};
#[cfg(feature = "mysql")]
use mysql_async::{Pool, Conn, Row, Value as MySqlValue, Params};
#[cfg(feature = "mysql")]
use mysql_async::prelude::*;
// 移除不存在的zerg_creep::prelude导入

/// MySQL适配器
#[derive(Debug, Clone)]
#[cfg(feature = "mysql")]
pub struct MysqlAdapter {
    /// 适配器名称
    pub name: String,
}

impl MysqlAdapter {
    /// 创建新的MySQL适配器
    pub fn new() -> Self {
        Self {
            name: "MySQL".to_string(),
        }
    }

    /// 将MySQL行转换为JSON值
    fn row_to_json(&self, row: &Row) -> QuickDbResult<Value> {
        let mut map = serde_json::Map::new();
        
        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name_str();
            let value: MySqlValue = row.get(i).unwrap_or(MySqlValue::NULL);
            
            let json_value = match value {
                MySqlValue::NULL => Value::Null,
                MySqlValue::Bytes(bytes) => {
                    // 尝试将字节转换为字符串
                    match String::from_utf8(bytes) {
                        Ok(s) => Value::String(s),
                        Err(_) => Value::String(base64::encode(&bytes)),
                    }
                },
                MySqlValue::Int(i) => Value::Number(serde_json::Number::from(i)),
                MySqlValue::UInt(u) => Value::Number(serde_json::Number::from(u)),
                MySqlValue::Float(f) => {
                    if let Some(num) = serde_json::Number::from_f64(f as f64) {
                        Value::Number(num)
                    } else {
                        Value::Null
                    }
                },
                MySqlValue::Double(d) => {
                    if let Some(num) = serde_json::Number::from_f64(d) {
                        Value::Number(num)
                    } else {
                        Value::Null
                    }
                },
                MySqlValue::Date(year, month, day, hour, minute, second, micro) => {
                    let datetime_str = format!(
                        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                        year, month, day, hour, minute, second, micro
                    );
                    Value::String(datetime_str)
                },
                MySqlValue::Time(neg, days, hours, minutes, seconds, micros) => {
                    let time_str = format!(
                        "{}{} {:02}:{:02}:{:02}.{:06}",
                        if neg { "-" } else { "" },
                        days,
                        hours,
                        minutes,
                        seconds,
                        micros
                    );
                    Value::String(time_str)
                },
            };
            
            map.insert(column_name.to_string(), json_value);
        }
        
        Ok(Value::Object(map))
    }

    /// 将DataValue转换为MySQL参数
    fn data_value_to_mysql_value(&self, value: &DataValue) -> MySqlValue {
        match value {
            DataValue::Null => MySqlValue::NULL,
            DataValue::Bool(b) => MySqlValue::Int(if *b { 1 } else { 0 }),
            DataValue::Int(i) => MySqlValue::Int(*i),
            DataValue::Float(f) => MySqlValue::Double(*f),
            DataValue::String(s) => MySqlValue::Bytes(s.as_bytes().to_vec()),
            DataValue::Bytes(b) => MySqlValue::Bytes(b.clone()),
            DataValue::DateTime(dt) => MySqlValue::Bytes(dt.to_rfc3339().as_bytes().to_vec()),
            DataValue::Uuid(uuid) => MySqlValue::Bytes(uuid.to_string().as_bytes().to_vec()),
            DataValue::Json(json) => {
                let json_str = serde_json::to_string(json).unwrap_or_default();
                MySqlValue::Bytes(json_str.as_bytes().to_vec())
            },
            DataValue::Array(arr) => {
                let json_str = serde_json::to_string(arr).unwrap_or_default();
                MySqlValue::Bytes(json_str.as_bytes().to_vec())
            },
            DataValue::Object(obj) => {
                let json_str = serde_json::to_string(obj).unwrap_or_default();
                MySqlValue::Bytes(json_str.as_bytes().to_vec())
            },
        }
    }

    /// 执行查询并返回结果
    async fn execute_query(
        &self,
        pool: &Pool,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<Vec<Value>> {
        let mut conn = pool.get_conn().await
            .map_err(|e| QuickDbError::ConnectionError {
                message: format!("获取MySQL连接失败: {}", e),
            })?;

        // 转换参数
        let mysql_params: Vec<MySqlValue> = params.iter()
            .map(|p| self.data_value_to_mysql_value(p))
            .collect();

        let rows: Vec<Row> = conn.exec(sql, mysql_params).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行MySQL查询失败: {}", e),
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
        pool: &Pool,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<u64> {
        let mut conn = pool.get_conn().await
            .map_err(|e| QuickDbError::ConnectionError {
                message: format!("获取MySQL连接失败: {}", e),
            })?;

        // 转换参数
        let mysql_params: Vec<MySqlValue> = params.iter()
            .map(|p| self.data_value_to_mysql_value(p))
            .collect();

        let result = conn.exec_drop(sql, mysql_params).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行MySQL更新失败: {}", e),
            })?;
        
        Ok(conn.affected_rows())
    }
}

#[async_trait]
impl DatabaseAdapter for MysqlAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<Value> {
        if let DatabaseConnection::MySQL(ref pool) = connection {
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
                info!("自动创建MySQL表 '{}' 成功", table);
            }
            
            let (sql, params) = SqlQueryBuilder::new()
                .insert(data.clone())
                .from(table)
                .build()?;
            
            let affected_rows = self.execute_update(pool, &sql, &params).await?;
            
            // 获取插入的行ID
            let last_id_sql = "SELECT LAST_INSERT_ID() as id";
            let id_results = self.execute_query(pool, last_id_sql, &[]).await?;
            
            if let Some(result) = id_results.first() {
                Ok(result.clone())
            } else {
                Ok(serde_json::json!({
                    "affected_rows": affected_rows
                }))
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn find_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<Value>> {
        if let DatabaseConnection::MySQL(ref pool) = connection {
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
            
            let results = self.execute_query(pool, &sql, &params).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
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
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .select(&["*"])
                .from(table)
                .where_conditions(conditions);
            
            // 添加排序
            for sort_field in &options.sort {
                builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.skip);
            }
            
            let (sql, params) = builder.build()?;
            
            self.execute_query(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
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
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
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
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .update(data.clone())
                .from(table)
                .where_condition(condition)
                .build()?;
            
            let affected_rows = self.execute_update(pool, &sql, &params).await?;
            Ok(affected_rows > 0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn delete(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .delete()
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            self.execute_update(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn delete_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };
            
            let (sql, params) = SqlQueryBuilder::new()
                .delete()
                .from(table)
                .where_condition(condition)
                .build()?;
            
            let affected_rows = self.execute_update(pool, &sql, &params).await?;
            Ok(affected_rows > 0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn count(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let results = self.execute_query(pool, &sql, &params).await?;
            
            if let Some(result) = results.first() {
                if let Some(count_value) = result.get("count") {
                    if let Some(count) = count_value.as_u64() {
                        return Ok(count);
                    }
                }
            }
            
            Ok(0)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
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
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let mut field_definitions = Vec::new();
            
            // 检查是否已经有id字段，如果没有则添加默认的id主键
            if !fields.contains_key("id") {
                field_definitions.push("id BIGINT AUTO_INCREMENT PRIMARY KEY".to_string());
            }
            
            for (name, field_type) in fields {
                let sql_type = match field_type {
                    FieldType::String => "TEXT",
                    FieldType::Integer => "BIGINT",
                    FieldType::Float => "DOUBLE",
                    FieldType::Boolean => "BOOLEAN",
                    FieldType::DateTime => "DATETIME",
                    FieldType::Uuid => "VARCHAR(36)",
                    FieldType::Json => "JSON",
                    FieldType::Array(_) => "JSON",
                    FieldType::Object => "JSON",
                };
                
                // 如果是id字段，添加主键约束
                if name == "id" {
                    field_definitions.push(format!("{} {} PRIMARY KEY AUTO_INCREMENT", name, sql_type));
                } else {
                    field_definitions.push(format!("{} {}", name, sql_type));
                }
            }
            
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                table,
                field_definitions.join(", ")
            );
            
            self.execute_update(pool, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
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
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let unique_clause = if unique { "UNIQUE " } else { "" };
            let sql = format!(
                "CREATE {}INDEX {} ON {} ({})",
                unique_clause,
                index_name,
                table,
                fields.join(", ")
            );
            
            self.execute_update(pool, &sql, &[]).await?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MySQL(ref pool) = connection {
            let sql = "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?";
            let params = vec![DataValue::String(table.to_string())];
            let results = self.execute_query(pool, sql, &params).await?;
            
            Ok(!results.is_empty())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MySQL连接".to_string(),
            })
        }
    }
}

impl Default for MysqlAdapter {
    fn default() -> Self {
        Self::new()
    }
}