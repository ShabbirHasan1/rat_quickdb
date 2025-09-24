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
use rat_logger::{info, error, warn, debug};
use sqlx::{MySql, Pool, Row, Column, TypeInfo};
use sqlx::mysql::MySqlRow;
use serde_json::Value as JsonValue;
// 移除不存在的rat_logger::prelude导入

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
    
    /// 安全地读取整数字段，防止 byteorder 错误
    fn safe_read_integer(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
        // 尝试多种整数类型读取，按照从最常见到最不常见的顺序
        
        // 1. 尝试读取为 Option<i64>
        if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
            return Ok(match val {
                Some(i) => DataValue::Int(i),
                None => DataValue::Null,
            });
        }
        
        // 2. 尝试读取为 Option<i32>
        if let Ok(val) = row.try_get::<Option<i32>, _>(column_name) {
            return Ok(match val {
                Some(i) => DataValue::Int(i as i64),
                None => DataValue::Null,
            });
        }
        
        // 3. 尝试读取为 Option<u64>
        if let Ok(val) = row.try_get::<Option<u64>, _>(column_name) {
            return Ok(match val {
                Some(i) => {
                    if i <= i64::MAX as u64 {
                        DataValue::Int(i as i64)
                    } else {
                        // 如果超出 i64 范围，转为字符串
                        DataValue::String(i.to_string())
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 4. 尝试读取为 Option<u32>
        if let Ok(val) = row.try_get::<Option<u32>, _>(column_name) {
            return Ok(match val {
                Some(i) => DataValue::Int(i as i64),
                None => DataValue::Null,
            });
        }
        
        // 5. 最后尝试读取为字符串
        if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
            return Ok(match val {
                Some(s) => {
                    // 尝试解析为数字
                    if let Ok(i) = s.parse::<i64>() {
                        DataValue::Int(i)
                    } else {
                        DataValue::String(s)
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 如果所有尝试都失败，返回错误
        Err(QuickDbError::SerializationError {
            message: format!("无法读取整数字段 '{}' 的值，所有类型转换都失败", column_name),
        })
    }

    /// 安全读取浮点数，避免 byteorder 错误
    fn safe_read_float(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
        // 首先尝试读取 f32 (MySQL FLOAT 是 4 字节)
        if let Ok(val) = row.try_get::<Option<f32>, _>(column_name) {
            return Ok(match val {
                Some(f) => DataValue::Float(f as f64),
                None => DataValue::Null,
            });
        }
        
        // 然后尝试读取 f64 (MySQL DOUBLE 是 8 字节)
        if let Ok(val) = row.try_get::<Option<f64>, _>(column_name) {
            return Ok(match val {
                Some(f) => DataValue::Float(f),
                None => DataValue::Null,
            });
        }
        
        // 尝试以字符串读取并解析
        if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
            return Ok(match val {
                Some(s) => {
                    if let Ok(f) = s.parse::<f64>() {
                        DataValue::Float(f)
                    } else {
                        DataValue::String(s)
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 所有尝试都失败，返回错误
        Err(QuickDbError::SerializationError { message: format!("无法读取浮点数字段 '{}'", column_name) })
    }

    /// 安全读取布尔值，避免 byteorder 错误
    fn safe_read_bool(row: &MySqlRow, column_name: &str) -> QuickDbResult<DataValue> {
        // 尝试以 bool 读取
        if let Ok(val) = row.try_get::<Option<bool>, _>(column_name) {
            return Ok(match val {
                Some(b) => DataValue::Bool(b),
                None => DataValue::Null,
            });
        }
        
        // 尝试以整数读取（MySQL 中 BOOLEAN 通常存储为 TINYINT）
        if let Ok(val) = row.try_get::<Option<i8>, _>(column_name) {
            return Ok(match val {
                Some(i) => DataValue::Bool(i != 0),
                None => DataValue::Null,
            });
        }
        
        // 尝试以字符串读取并解析
        if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
            return Ok(match val {
                Some(s) => {
                    match s.to_lowercase().as_str() {
                        "true" | "1" | "yes" | "on" => DataValue::Bool(true),
                        "false" | "0" | "no" | "off" => DataValue::Bool(false),
                        _ => DataValue::String(s),
                    }
                },
                None => DataValue::Null,
            });
        }
        
        // 所有尝试都失败，返回错误
        Err(QuickDbError::SerializationError { message: format!("无法读取布尔字段 '{}'", column_name) })
    }

    /// 将MySQL行转换为DataValue映射
    fn row_to_data_map(&self, row: &MySqlRow) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut data_map = HashMap::new();
        
        for column in row.columns() {
            let column_name = column.name();
            let column_type = column.type_info().name();
            
            // 调试：输出列类型信息
            info!("开始处理MySQL列 '{}' 的类型: '{}'", column_name, column_type);
              
            // 根据MySQL类型转换值
            let data_value = match column_type {
                "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => {
                    debug!("准备读取整数字段: {}", column_name);
                    // 使用安全的整数读取方法，防止 byteorder 错误
                    match Self::safe_read_integer(row, column_name) {
                        Ok(value) => {
                            debug!("成功读取整数字段 {}: {:?}", column_name, value);
                            value
                        },
                        Err(e) => {
                            error!("读取整数字段 {} 时发生错误: {}", column_name, e);
                            DataValue::Null
                        }
                    }
                },
                // 处理UNSIGNED整数类型
                "INT UNSIGNED" | "BIGINT UNSIGNED" | "SMALLINT UNSIGNED" | "TINYINT UNSIGNED" => {
                    // 对于LAST_INSERT_ID()，MySQL返回的是unsigned long long，但sqlx可能会将其作为i64处理
                    // 我们应该优先尝试i64，因为MySQL的LAST_INSERT_ID()通常在合理范围内
                    
                    // 1. 首先尝试i64，因为MySQL的自增ID通常不会超过i64::MAX
                    if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
                        match val {
                            Some(i) => {
                                // 如果i64为负数，这可能是类型转换错误，尝试u64
                                if i < 0 {
                                    if let Ok(u_val) = row.try_get::<Option<u64>, _>(column_name) {
                                        if let Some(u) = u_val {
                                            DataValue::Int(u as i64)
                                        } else {
                                            DataValue::Null
                                        }
                                    } else {
                                        DataValue::Null
                                    }
                                } else {
                                    DataValue::Int(i)
                                }
                            },
                            None => DataValue::Null,
                        }
                    }
                    // 2. 尝试u64
                    else if let Ok(val) = row.try_get::<Option<u64>, _>(column_name) {
                        match val {
                            Some(u) => {
                                if u <= i64::MAX as u64 {
                                    DataValue::Int(u as i64)
                                } else {
                                    DataValue::String(u.to_string())
                                }
                            },
                            None => DataValue::Null,
                        }
                    }
                    // 3. 尝试作为字符串读取，避免字节序问题
                    else if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                        match val {
                            Some(s) => {
                                if let Ok(u) = s.parse::<u64>() {
                                    if u <= i64::MAX as u64 {
                                        DataValue::Int(u as i64)
                                    } else {
                                        DataValue::String(u.to_string())
                                    }
                                } else if let Ok(i) = s.parse::<i64>() {
                                    DataValue::Int(i)
                                } else {
                                    DataValue::String(s)
                                }
                            },
                            None => DataValue::Null,
                        }
                    } else {
                        warn!("无法读取无符号整数字段 '{}' 的值，类型: {}", column_name, column_type);
                        DataValue::Null
                    }
                },
                "FLOAT" | "DOUBLE" => {
                    debug!("准备读取浮点数字段: {}", column_name);
                    match Self::safe_read_float(row, column_name) {
                        Ok(value) => {
                            debug!("成功读取浮点数字段 {}: {:?}", column_name, value);
                            value
                        },
                        Err(e) => {
                            error!("读取浮点数字段 {} 时发生错误: {}", column_name, e);
                            DataValue::Null
                        }
                    }
                },
                "BOOLEAN" | "BOOL" => {
                    debug!("准备读取布尔字段: {}", column_name);
                    match Self::safe_read_bool(row, column_name) {
                        Ok(value) => {
                            debug!("成功读取布尔字段 {}: {:?}", column_name, value);
                            value
                        },
                        Err(e) => {
                            error!("读取布尔字段 {} 时发生错误: {}", column_name, e);
                            DataValue::Null
                        }
                    }
                },
                "VARCHAR" | "TEXT" | "CHAR" => {
                    debug!("准备读取字符串字段: {}", column_name);
                    if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                        let result = match value {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        };
                        debug!("成功读取字符串字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取字符串字段: {}", column_name);
                        DataValue::Null
                    }
                },
                "JSON" => {
                    debug!("准备读取JSON字段: {}", column_name);
                    if let Ok(value) = row.try_get::<Option<JsonValue>, _>(column_name) {
                        let result = match value {
                            Some(json) => DataValue::Json(json),
                            None => DataValue::Null,
                        };
                        debug!("成功读取JSON字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取JSON字段: {}", column_name);
                        DataValue::Null
                    }
                },
                "DATETIME" | "TIMESTAMP" => {
                    debug!("准备读取日期时间字段: {}", column_name);
                    if let Ok(value) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(column_name) {
                        let result = match value {
                            Some(dt) => DataValue::DateTime(dt),
                            None => DataValue::Null,
                        };
                        debug!("成功读取日期时间字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取日期时间字段: {}", column_name);
                        DataValue::Null
                    }
                },
                _ => {
                    debug!("处理未知类型字段: {} (类型: {})", column_name, column_type);
                    // 对于未知类型，尝试作为字符串处理
                    if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
                        let result = match value {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        };
                        debug!("成功读取未知类型字段 {}: {:?}", column_name, result);
                        result
                    } else {
                        error!("无法读取未知类型字段: {}", column_name);
                        DataValue::Null
                    }
                }
            };
            
            data_map.insert(column_name.to_string(), data_value);
        }
        
        Ok(data_map)
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
            // 使用 catch_unwind 捕获可能的 panic，防止连接池崩溃
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.row_to_data_map(&row)
            })) {
                Ok(Ok(data_map)) => {
                    results.push(DataValue::Object(data_map));
                },
                Ok(Err(e)) => {
                    error!("行数据转换失败: {}", e);
                    // 创建一个包含错误信息的对象，而不是跳过这一行
                    let mut error_map = HashMap::new();
                    error_map.insert("error".to_string(), DataValue::String(format!("数据转换失败: {}", e)));
                    results.push(DataValue::Object(error_map));
                },
                Err(panic_info) => {
                    error!("行数据转换时发生 panic: {:?}", panic_info);
                    // 创建一个包含 panic 信息的对象
                    let mut error_map = HashMap::new();
                    error_map.insert("error".to_string(), DataValue::String("数据转换时发生内部错误".to_string()));
                    results.push(DataValue::Object(error_map));
                }
            }
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
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
                .insert(data.clone())
                .from(table)
                .build()?;
            
            // 使用事务确保插入和获取ID在同一个连接中
            let mut tx = pool.begin().await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("开始事务失败: {}", e),
                })?;
            
            let affected_rows = {
                let mut query = sqlx::query(&sql);
                // 绑定参数
                for param in &params {
                    query = match param {
                        DataValue::String(s) => query.bind(s),
                        DataValue::Int(i) => query.bind(*i),
                        DataValue::Float(f) => query.bind(*f),
                        DataValue::Bool(b) => query.bind(*b),
                        DataValue::DateTime(dt) => query.bind(*dt),
                        DataValue::Uuid(uuid) => query.bind(*uuid),
                        DataValue::Json(json) => query.bind(json.to_string()),
                        DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                        DataValue::Null => query.bind(Option::<String>::None),
                        DataValue::Array(arr) => {
                            let json_values: Vec<serde_json::Value> = arr.iter()
                                .map(|v| v.to_json_value())
                                .collect();
                            query.bind(serde_json::to_string(&json_values).unwrap_or_default())
                        },
                        DataValue::Object(obj) => {
                            let json_map: serde_json::Map<String, serde_json::Value> = obj.iter()
                                .map(|(k, v)| (k.clone(), v.to_json_value()))
                                .collect();
                            query.bind(serde_json::to_string(&json_map).unwrap_or_default())
                        },
                    };
                }
                
                query.execute(&mut *tx).await
                    .map_err(|e| QuickDbError::QueryError {
                        message: format!("执行插入失败: {}", e),
                    })?
                    .rows_affected()
            };
            
            debug!("插入操作影响的行数: {}", affected_rows);
            
            // 在同一个事务中查询LAST_INSERT_ID()
            let last_id_row = sqlx::query("SELECT LAST_INSERT_ID() as id")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("获取LAST_INSERT_ID失败: {}", e),
                })?;
            
            // 调试：输出事务中的ID查询结果
            info!("事务中的LAST_INSERT_ID结果: {:?}", last_id_row);

            // 提交事务
            tx.commit().await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("提交事务失败: {}", e),
                })?;
            
            // MySQL的LAST_INSERT_ID()返回的是u64
            let last_id: u64 = last_id_row.try_get("id")
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("解析LAST_INSERT_ID失败: {}", e),
                })?;
            
            info!("获取到的LAST_INSERT_ID: {}", last_id);
            info!("转换为i64的ID: {}", last_id as i64);
            
            // 构造返回的DataValue
            let mut result_map = std::collections::HashMap::new();
            // 将u64转换为i64，因为DataValue::Int使用i64
            // MySQL自增ID通常在i64范围内，如果超出则使用字符串
            let id_value = if last_id <= i64::MAX as u64 {
                let id_as_i64 = last_id as i64;
                DataValue::Int(id_as_i64)
            } else {
                DataValue::String(last_id.to_string())
            };
            result_map.insert("id".to_string(), id_value.clone());
            result_map.insert("affected_rows".to_string(), DataValue::Int(affected_rows as i64));

            info!("最终返回的DataValue: {:?}", DataValue::Object(result_map.clone()));
            Ok(DataValue::Object(result_map))
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
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
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
        // 将简单条件转换为条件组合（AND逻辑）
        let condition_groups = if conditions.is_empty() {
            vec![]
        } else {
            let group_conditions = conditions.iter()
                .map(|c| QueryConditionGroup::Single(c.clone()))
                .collect();
            vec![QueryConditionGroup::Group {
                operator: crate::types::LogicalOperator::And,
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
        if let DatabaseConnection::MySQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
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
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
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
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
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
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
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
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
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
                .database_type(crate::adapter::query_builder::DatabaseType::MySQL)
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
                // 使用BIGINT UNSIGNED确保与LAST_INSERT_ID()兼容
                field_definitions.push("id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY".to_string());
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