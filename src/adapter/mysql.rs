//! MySQL数据库适配器
//!
//! 基于mysql_async库实现的MySQL数据库适配器，提供完整的CRUD操作支持

use crate::adapter::DatabaseAdapter;
use crate::pool::DatabaseConnection;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::{DataValue, QueryCondition, QueryConditionGroup, QueryOperator, QueryOptions, SortDirection};
use crate::adapter::query_builder::SqlQueryBuilder;
use crate::table::{TableManager, TableSchema, ColumnType};
use crate::model::FieldType;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use zerg_creep::{info, error, warn, debug};
use sqlx::{MySql, Pool, Row, Column, TypeInfo};
use sqlx::mysql::MySqlRow;
use serde_json::Value as JsonValue;
// 移除不存在的zerg_creep::prelude导入

/// MySQL适配器
#[derive(Debug, Clone)]
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

    /// 将MySQL行转换为DataValue映射
    fn row_to_data_map(&self, row: &MySqlRow) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut map = HashMap::new();
        
        for column in row.columns() {
            let column_name = column.name();
            
            // 根据MySQL类型转换值
            let data_value = match column.type_info().name() {
                "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => {
                    if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
                        match val {
                            Some(i) => DataValue::Int(i),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "FLOAT" | "DOUBLE" => {
                    if let Ok(val) = row.try_get::<Option<f64>, _>(column_name) {
                        match val {
                            Some(f) => DataValue::Float(f),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "BOOLEAN" | "BOOL" => {
                    if let Ok(val) = row.try_get::<Option<bool>, _>(column_name) {
                        match val {
                            Some(b) => DataValue::Bool(b),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "VARCHAR" | "TEXT" | "CHAR" => {
                    if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                        match value {
                            Some(s) => crate::types::parse_optional_json_string_to_data_value(Some(s)),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "JSON" => {
                    if let Ok(val) = row.try_get::<Option<JsonValue>, _>(column_name) {
                        match val {
                            Some(json_val) => crate::types::json_value_to_data_value(json_val),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "DATETIME" | "TIMESTAMP" => {
                    if let Ok(val) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column_name) {
                        match val {
                            Some(dt) => DataValue::DateTime(dt),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                _ => {
                    // 对于未知类型，尝试作为字符串处理
                    if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                        match value {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                }
            };
            
            map.insert(column_name.to_string(), data_value);
        }
        
        Ok(map)
    }



    /// 执行查询并返回结果
    async fn execute_query(
        &self,
        pool: &Pool<MySql>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<Vec<DataValue>> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // 检查是否为JSON字符串，如果是则转换为对应的DataValue类型
                    let converted_value = crate::types::parse_json_string_to_data_value(s.clone());
                    match converted_value {
                        DataValue::Json(json_val) => {
                            query.bind(serde_json::to_string(&json_val).unwrap_or_default())
                        },
                        _ => query.bind(s)
                    }
                },
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json.to_string()),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => {
                    // 将DataValue数组转换为原始JSON数组
                    let json_values: Vec<serde_json::Value> = arr.iter()
                        .map(|v| v.to_json_value())
                        .collect();
                    query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                },
                DataValue::Object(obj) => {
                    // 将DataValue对象转换为原始JSON对象
                    let json_map: serde_json::Map<String, serde_json::Value> = obj.iter()
                        .map(|(k, v)| (k.clone(), v.to_json_value()))
                        .collect();
                    query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                },
            };
        }

        let rows = query.fetch_all(pool).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行MySQL查询失败: {}", e),
            })?;
        
        let mut results = Vec::new();
        for row in rows {
            let data_map = self.row_to_data_map(&row)?;
            results.push(DataValue::Object(data_map));
        }
        
        Ok(results)
    }

    /// 执行更新操作
    async fn execute_update(
        &self,
        pool: &Pool<MySql>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<u64> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => {
                    // 检查是否为JSON字符串，如果是则转换为对应的DataValue类型
                    let converted_value = crate::types::parse_json_string_to_data_value(s.clone());
                    match converted_value {
                        DataValue::Json(json_val) => {
                            query.bind(serde_json::to_string(&json_val).unwrap_or_default())
                        },
                        _ => query.bind(s)
                    }
                },
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json.to_string()),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => {
                    // 将DataValue数组转换为原始JSON数组
                    let json_values: Vec<serde_json::Value> = arr.iter()
                        .map(|v| v.to_json_value())
                        .collect();
                    query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                },
                DataValue::Object(obj) => {
                    // 将DataValue对象转换为原始JSON对象
                    let json_map: serde_json::Map<String, serde_json::Value> = obj.iter()
                        .map(|(k, v)| (k.clone(), v.to_json_value()))
                        .collect();
                    query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                },
            };
        }

        let result = query.execute(pool).await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行MySQL更新失败: {}", e),
            })?;
        
        Ok(result.rows_affected())
    }
}

#[async_trait]
impl DatabaseAdapter for MysqlAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::MySQL(pool) = connection {
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
                Ok(DataValue::Object({
                    let mut map = std::collections::HashMap::new();
                    map.insert("affected_rows".to_string(), DataValue::Int(affected_rows as i64));
                    map
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
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
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
    ) -> QuickDbResult<Vec<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
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

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .select(&["*"])
                .from(table)
                .where_condition_groups(condition_groups);
            
            // 添加排序
            for sort_field in &options.sort {
                builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.skip);
            }
            
            let (sql, params) = builder.build()?;
            
            debug!("执行MySQL条件组合查询: {}", sql);
            
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
        if let DatabaseConnection::MySQL(pool) = connection {
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
        if let DatabaseConnection::MySQL(pool) = connection {
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
        if let DatabaseConnection::MySQL(pool) = connection {
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
        if let DatabaseConnection::MySQL(pool) = connection {
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
        if let DatabaseConnection::MySQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            let results = self.execute_query(pool, &sql, &params).await?;
            
            if let Some(result) = results.first() {
                if let DataValue::Object(map) = result {
                    if let Some(DataValue::Int(count)) = map.get("count") {
                        return Ok(*count as u64);
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
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut field_definitions = Vec::new();
            
            // 检查是否已经有id字段，如果没有则添加默认的id主键
            if !fields.contains_key("id") {
                field_definitions.push("id BIGINT AUTO_INCREMENT PRIMARY KEY".to_string());
            }
            
            for (name, field_type) in fields {
                let sql_type = match field_type {
                    FieldType::String { max_length, .. } => {
                        if let Some(max_len) = max_length {
                            format!("VARCHAR({})", max_len)
                        } else {
                            "TEXT".to_string()
                        }
                    },
                    FieldType::Integer { .. } => "INT".to_string(),
                    FieldType::BigInteger => "BIGINT".to_string(),
                    FieldType::Float { .. } => "FLOAT".to_string(),
                    FieldType::Double => "DOUBLE".to_string(),
                    FieldType::Text => "TEXT".to_string(),
                    FieldType::Boolean => "BOOLEAN".to_string(),
                    FieldType::DateTime => "DATETIME".to_string(),
                    FieldType::Date => "DATE".to_string(),
                    FieldType::Time => "TIME".to_string(),
                    FieldType::Uuid => "VARCHAR(36)".to_string(),
                    FieldType::Json => "JSON".to_string(),
                    FieldType::Binary => "BLOB".to_string(),
                    FieldType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
                    FieldType::Array { .. } => "JSON".to_string(),
                    FieldType::Object { .. } => "JSON".to_string(),
                    FieldType::Reference { .. } => "VARCHAR(255)".to_string(),
                };
                
                // 如果是id字段，添加主键约束
                if name == "id" {
                    field_definitions.push(format!("{} {} AUTO_INCREMENT PRIMARY KEY", name, sql_type));
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
        if let DatabaseConnection::MySQL(pool) = connection {
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
        if let DatabaseConnection::MySQL(pool) = connection {
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

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MySQL(pool) = connection {
            let sql = format!("DROP TABLE IF EXISTS {}", table);
            
            debug!("执行MySQL删除表SQL: {}", sql);
            
            self.execute_update(pool, &sql, &[]).await?;
            
            info!("成功删除MySQL表: {}", table);
            Ok(())
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