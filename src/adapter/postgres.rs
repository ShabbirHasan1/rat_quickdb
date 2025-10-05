//! PostgreSQL数据库适配器
//! 
//! 使用tokio-postgres库实现真实的PostgreSQL数据库操作

use super::{DatabaseAdapter, SqlQueryBuilder};
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::FieldType;
use crate::pool::DatabaseConnection;
use crate::table::{TableManager, TableSchema, ColumnType};
use crate::cache::CacheOps;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use rat_logger::{info, error, warn, debug};
use sqlx::{Row, Column, TypeInfo};
// 移除不存在的rat_logger::prelude导入

/// PostgreSQL适配器
pub struct PostgresAdapter;

impl PostgresAdapter {


    /// 将PostgreSQL行转换为DataValue映射
    fn row_to_data_map(&self, row: &sqlx::postgres::PgRow) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut map = HashMap::new();
        
        for column in row.columns() {
            let column_name = column.name();
            
            // 根据PostgreSQL类型转换值
            let data_value = match column.type_info().name() {
                "INT4" | "INT8" => {
                    if let Ok(val) = row.try_get::<Option<i32>, _>(column_name) {
                        match val {
                            Some(i) => DataValue::Int(i as i64),
                            None => DataValue::Null,
                        }
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(column_name) {
                        match val {
                            Some(i) => DataValue::Int(i),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "FLOAT4" | "FLOAT8" => {
                    if let Ok(val) = row.try_get::<Option<f32>, _>(column_name) {
                        match val {
                            Some(f) => DataValue::Float(f as f64),
                            None => DataValue::Null,
                        }
                    } else if let Ok(val) = row.try_get::<Option<f64>, _>(column_name) {
                        match val {
                            Some(f) => DataValue::Float(f),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "BOOL" => {
                    if let Ok(val) = row.try_get::<Option<bool>, _>(column_name) {
                        match val {
                            Some(b) => DataValue::Bool(b),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "TEXT" | "VARCHAR" | "CHAR" => {
                    if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                        match val {
                            Some(s) => DataValue::String(s),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "UUID" => {
                    if let Ok(val) = row.try_get::<Option<uuid::Uuid>, _>(column_name) {
                        match val {
                            Some(u) => DataValue::Uuid(u),
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "JSON" | "JSONB" => {
                    // PostgreSQL原生支持JSONB，直接获取serde_json::Value
                    // 无需像MySQL/SQLite那样解析JSON字符串
                    if let Ok(val) = row.try_get::<Option<serde_json::Value>, _>(column_name) {
                        match val {
                            Some(json_val) => {
                                // 使用现有的转换函数，确保类型正确
                                crate::types::json_value_to_data_value(json_val)
                            },
                            None => DataValue::Null,
                        }
                    } else {
                        DataValue::Null
                    }
                },
                "TIMESTAMP" | "TIMESTAMPTZ" => {
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
                    // 对于未知类型，尝试作为字符串获取
                    if let Ok(val) = row.try_get::<Option<String>, _>(column_name) {
                        match val {
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

    /// 将PostgreSQL行转换为JSON值（保留用于向后兼容）
    fn row_to_json(&self, row: &sqlx::postgres::PgRow) -> QuickDbResult<Value> {
        let data_map = self.row_to_data_map(row)?;
        let mut json_map = serde_json::Map::new();
        
        for (key, value) in data_map {
            json_map.insert(key, value.to_json_value());
        }
        
        Ok(Value::Object(json_map))
    }

    /// 执行查询并返回结果
    async fn execute_query(
        &self,
        pool: &sqlx::Pool<sqlx::Postgres>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<Vec<DataValue>> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => query.bind(s),
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => {
                    // 使用 to_json_value() 避免序列化时包含类型标签
                    let json_array = DataValue::Array(arr.clone()).to_json_value();
                    query.bind(json_array)
                },
                DataValue::Object(obj) => {
                    // 使用 to_json_value() 避免序列化时包含类型标签
                    let json_object = DataValue::Object(obj.clone()).to_json_value();
                    query.bind(json_object)
                },
            };
        }
        
        let rows = query.fetch_all(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行PostgreSQL查询失败: {}", e),
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
        pool: &sqlx::Pool<sqlx::Postgres>,
        sql: &str,
        params: &[DataValue],
    ) -> QuickDbResult<u64> {
        let mut query = sqlx::query(sql);
        
        // 绑定参数
        for param in params {
            query = match param {
                DataValue::String(s) => query.bind(s),
                DataValue::Int(i) => query.bind(*i),
                DataValue::Float(f) => query.bind(*f),
                DataValue::Bool(b) => query.bind(*b),
                DataValue::DateTime(dt) => query.bind(*dt),
                DataValue::Uuid(uuid) => query.bind(*uuid),
                DataValue::Json(json) => query.bind(json),
                DataValue::Bytes(bytes) => query.bind(bytes.as_slice()),
                DataValue::Null => query.bind(Option::<String>::None),
                DataValue::Array(arr) => query.bind(serde_json::to_value(arr).unwrap_or_default()),
                DataValue::Object(obj) => query.bind(serde_json::to_value(obj).unwrap_or_default()),
            };
        }
        
        let result = query.execute(pool)
            .await
            .map_err(|e| QuickDbError::QueryError {
                message: format!("执行PostgreSQL更新失败: {}", e),
            })?;
        
        Ok(result.rows_affected())
    }

    /// 直接根据ID查询数据库（无缓存，内部方法）
    async fn query_database_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let condition = QueryCondition {
                field: "id".to_string(),
                operator: QueryOperator::Eq,
                value: id.clone(),
            };

            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .select(&["*"])
                .from(table)
                .where_condition(condition)
                .limit(1)
                .build()?;

            debug!("执行PostgreSQL根据ID查询: {}", sql);

            let results = self.execute_query(pool, &sql, &params).await?;
            Ok(results.into_iter().next())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }

    /// 直接根据条件组查询数据库（无缓存，内部方法）
    async fn query_database_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<DataValue>> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let mut builder = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .select(&["*"])
                .from(table)
                .where_condition_groups(condition_groups);

            // 添加排序
            if !options.sort.is_empty() {
                for sort_field in &options.sort {
                    builder = builder.order_by(&sort_field.field, sort_field.direction.clone());
                }
            }

            // 添加分页
            if let Some(pagination) = &options.pagination {
                builder = builder.limit(pagination.limit).offset(pagination.skip);
            }

            let (sql, params) = builder.build()?;

            debug!("执行PostgreSQL条件组查询: {}", sql);

            self.execute_query(pool, &sql, &params).await
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            // 自动建表逻辑：检查表是否存在，如果不存在则创建
            let mut has_auto_increment_id = false;
            if !self.table_exists(connection, table).await? {
                info!("表 {} 不存在，正在自动创建", table);
                let schema = TableSchema::infer_from_data(table.to_string(), data);
                
                // 检查是否有自增id字段
                if let Some(id_col) = schema.columns.iter().find(|col| col.name == "id") {
                    has_auto_increment_id = id_col.auto_increment;
                }
                
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
            } else {
                // 表已存在，检查是否有SERIAL类型的id字段
                let check_serial_sql = "SELECT column_default FROM information_schema.columns WHERE table_name = $1 AND column_name = 'id'";
                let rows = sqlx::query(check_serial_sql)
                    .bind(table)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| QuickDbError::QueryError {
                        message: format!("检查表结构失败: {}", e),
                    })?;
                
                if let Some(row) = rows.first() {
                    if let Ok(Some(default_value)) = row.try_get::<Option<String>, _>("column_default") {
                        has_auto_increment_id = default_value.starts_with("nextval");
                    }
                }
            }
            
            // 准备插入数据
            // 如果数据中没有id字段，说明期望使用自增ID，不需要在INSERT中包含id字段
            // 如果数据中有id字段但表使用SERIAL自增，也要移除id字段让PostgreSQL自动生成
            let mut insert_data = data.clone();
            let data_has_id = insert_data.contains_key("id");
            
            if !data_has_id || (data_has_id && has_auto_increment_id) {
                insert_data.remove("id");
                debug!("使用PostgreSQL SERIAL自增，不在INSERT中包含id字段");
            }
            
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .insert(insert_data)
                .from(table)
                .returning(&["id"])
                .build()?;
            
            debug!("执行PostgreSQL插入: {}", sql);
            
            let results = self.execute_query(pool, &sql, &params).await?;
            
            if let Some(result) = results.first() {
                Ok(result.clone())
            } else {
                // 创建一个表示成功插入的DataValue
                let mut success_map = HashMap::new();
                success_map.insert("affected_rows".to_string(), DataValue::Int(1));
                Ok(DataValue::Object(success_map))
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
    ) -> QuickDbResult<Option<DataValue>> {
        // 生成缓存键
        let id_str = match id {
            DataValue::String(s) => s.clone(),
            DataValue::Int(i) => i.to_string(),
            DataValue::Uuid(u) => u.to_string(),
            _ => format!("{:?}", id),
        };
        let cache_key = format!("postgres:{}:record:{}", table, id_str);

        // 先检查缓存
        match CacheOps::get(&cache_key).await {
            Ok((true, Some(vec_data))) => {
                // 缓存命中
                if vec_data.len() == 1 {
                    info!("🔥 PostgreSQL缓存命中: {} (结果数量: {})", cache_key, vec_data.len());
                    return Ok(Some(vec_data.into_iter().next().unwrap()));
                } else {
                    info!("🔥 PostgreSQL缓存命中: {} (结果数量: {})", cache_key, vec_data.len());
                    return Ok(None);
                }
            }
            Ok((true, None)) => {
                // 缓存命中，但结果为空
                info!("🔥 PostgreSQL缓存命中: {} (空结果)", cache_key);
                return Ok(None);
            }
            Ok((false, _)) => {
                // 缓存未命中
                info!("❌ PostgreSQL缓存未命中: {}", cache_key);
            }
            Err(e) => {
                warn!("获取缓存失败: {}, 继续查询数据库", e);
            }
        }

        // 缓存未命中，查询数据库
        let result = self.query_database_by_id(connection, table, id).await?;

        // 缓存查询结果
        if let Some(ref data_value) = result {
            if let Err(e) = CacheOps::set(&cache_key, Some(vec![data_value.clone()]), Some(3600)).await {
                warn!("设置缓存失败: {}", e);
            } else {
                info!("📦 PostgreSQL设置缓存: {} (数据量: 1)", cache_key);
            }
        } else {
            // 缓存空结果
            if let Err(e) = CacheOps::set(&cache_key, None, Some(600)).await {
                warn!("设置空缓存失败: {}", e);
            } else {
                info!("📦 PostgreSQL设置空缓存: {}", cache_key);
            }
        }

        Ok(result)
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
        // 生成查询缓存键
        let query_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            condition_groups.hash(&mut hasher);
            options.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        };
        let cache_key = format!("postgres:{}:query:{}", table, query_hash);

        // 先检查缓存
        match CacheOps::get(&cache_key).await {
            Ok((true, Some(vec_data))) => {
                // 缓存命中
                info!("🔥 PostgreSQL缓存命中: {} (结果数量: {})", cache_key, vec_data.len());
                return Ok(vec_data);
            }
            Ok((true, None)) => {
                // 缓存命中，但结果为空
                info!("🔥 PostgreSQL缓存命中: {} (空结果)", cache_key);
                return Ok(Vec::new());
            }
            Ok((false, _)) => {
                // 缓存未命中
                info!("❌ PostgreSQL缓存未命中: {}", cache_key);
            }
            Err(e) => {
                warn!("获取查询缓存失败: {}, 继续查询数据库", e);
            }
        }

        // 缓存未命中，查询数据库
        let result = self.query_database_with_groups(connection, table, condition_groups, options).await?;

        // 缓存查询结果
        if let Err(e) = CacheOps::set(&cache_key, Some(result.clone()), Some(1800)).await {
            warn!("设置查询缓存失败: {}", e);
        } else {
            info!("📦 PostgreSQL设置缓存: {} (数据量: {})", cache_key, result.len());
        }

        Ok(result)
    }

    async fn update(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        conditions: &[QueryCondition],
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<u64> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .update(data.clone())
                .from(table)
                .where_conditions(conditions)
                .build()?;

            debug!("执行PostgreSQL更新: {}", sql);

            let result = self.execute_update(pool, &sql, &params).await?;

            // 更新成功后清理相关缓存
            if result > 0 {
                // 清理表相关的所有缓存
                if let Err(e) = CacheOps::clear_table("postgres", table).await {
                    warn!("清理PostgreSQL表缓存失败: {}", e);
                } else {
                    info!("✅ PostgreSQL更新后清理表缓存: postgres:{}", table);
                }
            }

            Ok(result)
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
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .delete()
                .from(table)
                .where_conditions(conditions)
                .build()?;

            debug!("执行PostgreSQL删除: {}", sql);

            let result = self.execute_update(pool, &sql, &params).await?;

            // 删除成功后清理相关缓存
            if result > 0 {
                // 清理表相关的所有缓存
                if let Err(e) = CacheOps::clear_table("postgres", table).await {
                    warn!("清理PostgreSQL表缓存失败: {}", e);
                } else {
                    info!("✅ PostgreSQL删除后清理表缓存: postgres:{}", table);
                }
            }

            Ok(result)
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
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let (sql, params) = SqlQueryBuilder::new()
                .database_type(super::query_builder::DatabaseType::PostgreSQL)
                .select(&["COUNT(*) as count"])
                .from(table)
                .where_conditions(conditions)
                .build()?;
            
            debug!("执行PostgreSQL计数: {}", sql);
            
            let results = self.execute_query(pool, &sql, &params).await?;
            if let Some(result) = results.first() {
                if let DataValue::Object(obj) = result {
                    if let Some(DataValue::Int(count)) = obj.get("count") {
                        return Ok(*count as u64);
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
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let mut field_definitions = Vec::new();
            
            // 检查是否已经有id字段，如果没有则添加默认的id主键
            if !fields.contains_key("id") {
                field_definitions.push("id SERIAL PRIMARY KEY".to_string());
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
                    FieldType::Integer { .. } => "INTEGER".to_string(),
                    FieldType::BigInteger => "BIGINT".to_string(),
                    FieldType::Float { .. } => "REAL".to_string(),
                    FieldType::Double => "DOUBLE PRECISION".to_string(),
                    FieldType::Text => "TEXT".to_string(),
                    FieldType::Boolean => "BOOLEAN".to_string(),
                    FieldType::DateTime => "TIMESTAMPTZ".to_string(),
                    FieldType::Date => "DATE".to_string(),
                    FieldType::Time => "TIME".to_string(),
                    FieldType::Uuid => "UUID".to_string(),
                    FieldType::Json => "JSONB".to_string(),
                    FieldType::Binary => "BYTEA".to_string(),
                    FieldType::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
                    FieldType::Array { item_type: _, max_items: _, min_items: _ } => "JSONB".to_string(),
                    FieldType::Object { .. } => "JSONB".to_string(),
                    FieldType::Reference { target_collection: _ } => "TEXT".to_string(),
                };
                
                // 如果是id字段，检查是否需要自增
                if name == "id" {
                    // 对于id字段，如果是整数类型且配置了AutoIncrement策略，使用SERIAL
                    if matches!(field_type, FieldType::Integer { .. }) {
                        field_definitions.push("id SERIAL PRIMARY KEY".to_string());
                    } else {
                        field_definitions.push(format!("{} {} PRIMARY KEY", name, sql_type));
                    }
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
            
            self.execute_update(pool, &sql, &[]).await?;
            
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
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let unique_clause = if unique { "UNIQUE " } else { "" };
            let sql = format!(
                "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
                unique_clause,
                index_name,
                table,
                fields.join(", ")
            );
            
            debug!("执行PostgreSQL索引创建: {}", sql);
            
            self.execute_update(pool, &sql, &[]).await?;
            
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
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let sql = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1";
            
            let rows = sqlx::query(sql)
                .bind(table)
                .fetch_all(pool)
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

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::PostgreSQL(pool) = connection {
            let sql = format!("DROP TABLE IF EXISTS {} CASCADE", table);
            
            debug!("执行PostgreSQL删除表SQL: {}", sql);
            
            sqlx::query(&sql)
                .execute(pool)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("删除PostgreSQL表失败: {}", e),
                })?;
            
            info!("成功删除PostgreSQL表: {}", table);
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望PostgreSQL连接".to_string(),
            })
        }
    }
}