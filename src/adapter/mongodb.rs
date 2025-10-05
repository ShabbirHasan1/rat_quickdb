//! MongoDB数据库适配器
//! 
//! 使用mongodb库实现真实的MongoDB文档操作

use super::DatabaseAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::pool::DatabaseConnection;
use crate::table::{TableManager, TableSchema, ColumnType};
use crate::model::FieldType;
use crate::cache::CacheOps;
use async_trait::async_trait;

use std::collections::HashMap;
use mongodb::{bson::{doc, Document, Bson}, Collection};
use rat_logger::{info, error, warn, debug};

/// MongoDB适配器
pub struct MongoAdapter;

impl MongoAdapter {
    /// 将DataValue转换为BSON值
    fn data_value_to_bson(&self, value: &DataValue) -> Bson {
        match value {
            DataValue::String(s) => Bson::String(s.clone()),
            DataValue::Int(i) => Bson::Int64(*i),
            DataValue::Float(f) => Bson::Double(*f),
            DataValue::Bool(b) => Bson::Boolean(*b),
            DataValue::DateTime(dt) => Bson::DateTime(mongodb::bson::DateTime::from_system_time(dt.clone().into())),
            DataValue::Uuid(uuid) => Bson::String(uuid.to_string()),
            DataValue::Json(json) => {
                // 尝试将JSON转换为BSON文档
                if let Ok(doc) = mongodb::bson::to_document(json) {
                    Bson::Document(doc)
                } else {
                    Bson::String(json.to_string())
                }
            },
            DataValue::Array(arr) => {
                let bson_array: Vec<Bson> = arr.iter()
                    .map(|v| self.data_value_to_bson(v))
                    .collect();
                Bson::Array(bson_array)
            },
            DataValue::Object(obj) => {
                let mut doc = Document::new();
                for (key, value) in obj {
                    doc.insert(key, self.data_value_to_bson(value));
                }
                Bson::Document(doc)
            },
            // Reference类型不存在，移除此行
            DataValue::Null => Bson::Null,
            DataValue::Bytes(bytes) => Bson::Binary(mongodb::bson::Binary {
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
                bytes: bytes.clone(),
            }),
        }
    }

    /// 将BSON文档转换为DataValue映射（不包装在Object中）
    fn document_to_data_map(&self, doc: &Document) -> QuickDbResult<HashMap<String, DataValue>> {
        let mut data_map = HashMap::new();

        for (key, value) in doc {
            let data_value = self.bson_to_data_value(value)?;
            // 保持原始字段名，包括MongoDB的_id字段
            data_map.insert(key.clone(), data_value);
        }

        Ok(data_map)
    }

    /// 将BSON文档转换为DataValue（保持兼容性）
    fn document_to_data_value(&self, doc: &Document) -> QuickDbResult<DataValue> {
        let data_map = self.document_to_data_map(doc)?;
        Ok(DataValue::Object(data_map))
    }
    
    /// 将BSON值转换为JSON Value
    fn bson_to_json_value(&self, bson: &Bson) -> QuickDbResult<serde_json::Value> {
        match bson {
            Bson::ObjectId(oid) => Ok(serde_json::Value::String(oid.to_hex())),
            Bson::String(s) => Ok(serde_json::Value::String(s.clone())),
            Bson::Int32(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
            Bson::Int64(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
            Bson::Double(f) => {
                if let Some(num) = serde_json::Number::from_f64(*f) {
                    Ok(serde_json::Value::Number(num))
                } else {
                    Ok(serde_json::Value::String(f.to_string()))
                }
            },
            Bson::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            Bson::Null => Ok(serde_json::Value::Null),
            Bson::Array(arr) => {
                let mut json_arr = Vec::new();
                for item in arr {
                    json_arr.push(self.bson_to_json_value(item)?);
                }
                Ok(serde_json::Value::Array(json_arr))
            },
            Bson::Document(doc) => {
                let mut json_map = serde_json::Map::new();
                for (key, value) in doc {
                    json_map.insert(key.clone(), self.bson_to_json_value(value)?);
                }
                Ok(serde_json::Value::Object(json_map))
            },
            Bson::DateTime(dt) => {
                // 将BSON DateTime转换为ISO 8601字符串
                let system_time: std::time::SystemTime = dt.clone().into();
                let datetime = chrono::DateTime::<chrono::Utc>::from(system_time);
                Ok(serde_json::Value::String(datetime.to_rfc3339()))
            },
            Bson::Binary(bin) => Ok(serde_json::Value::String(base64::encode(&bin.bytes))),
            Bson::Decimal128(dec) => Ok(serde_json::Value::String(dec.to_string())),
            _ => Ok(serde_json::Value::String(format!("{:?}", bson))),
        }
    }
    
    /// 将BSON值转换为DataValue，正确处理ObjectId
    fn bson_to_data_value(&self, bson: &Bson) -> QuickDbResult<DataValue> {
        match bson {
            Bson::ObjectId(oid) => Ok(DataValue::String(oid.to_hex())),
            Bson::String(s) => Ok(DataValue::String(s.clone())),
            Bson::Int32(i) => Ok(DataValue::Int(*i as i64)),
            Bson::Int64(i) => Ok(DataValue::Int(*i)),
            Bson::Double(f) => Ok(DataValue::Float(*f)),
            Bson::Boolean(b) => Ok(DataValue::Bool(*b)),
            Bson::Null => Ok(DataValue::Null),
            Bson::Array(arr) => {
                let mut data_arr = Vec::new();
                for item in arr {
                    data_arr.push(self.bson_to_data_value(item)?);
                }
                Ok(DataValue::Array(data_arr))
            },
            Bson::Document(doc) => {
                let mut data_map = HashMap::new();
                for (key, value) in doc {
                    let data_value = self.bson_to_data_value(value)?;
                    data_map.insert(key.clone(), data_value);
                }
                Ok(DataValue::Object(data_map))
            },
            Bson::DateTime(dt) => {
                // 将BSON DateTime转换为chrono::DateTime
                let system_time: std::time::SystemTime = dt.clone().into();
                let datetime = chrono::DateTime::<chrono::Utc>::from(system_time);
                Ok(DataValue::DateTime(datetime))
            },
            Bson::Binary(bin) => Ok(DataValue::Bytes(bin.bytes.clone())),
            Bson::Decimal128(dec) => Ok(DataValue::String(dec.to_string())),
            _ => {
                // 对于其他BSON类型，转换为字符串
                Ok(DataValue::String(bson.to_string()))
            }
        }
    }

    /// 构建MongoDB查询文档
    fn build_query_document(&self, conditions: &[QueryCondition]) -> QuickDbResult<Document> {
        println!("[MongoDB] 开始构建查询文档，条件数量: {}", conditions.len());
        let mut query_doc = Document::new();

        for (index, condition) in conditions.iter().enumerate() {
            let field_name = self.map_field_name(&condition.field);

            // 特殊处理_id字段的ObjectId格式
            let bson_value = if field_name == "_id" {
                if let DataValue::String(id_str) = &condition.value {
                    // 处理ObjectId格式：ObjectId("xxx") 或直接是ObjectId字符串
                    let actual_id = if id_str.starts_with("ObjectId(\"") && id_str.ends_with("\")") {
                        // 提取ObjectId字符串部分
                        &id_str[10..id_str.len()-2]
                    } else {
                        id_str
                    };

                    // 尝试解析为ObjectId
                    if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(actual_id) {
                        Bson::ObjectId(object_id)
                    } else {
                        Bson::String(actual_id.to_string())
                    }
                } else {
                    self.data_value_to_bson(&condition.value)
                }
            } else {
                self.data_value_to_bson(&condition.value)
            };

            println!("[MongoDB] 条件[{}]: 字段='{}' -> '{}', 操作符={:?}, 原始值={:?}, BSON值={:?}",
                   index, condition.field, field_name, condition.operator, condition.value, bson_value);

            match condition.operator {
                QueryOperator::Eq => {
                    println!("[MongoDB] 处理Eq操作符: {} = {:?}", field_name, bson_value);
                    query_doc.insert(field_name, bson_value);
                },
                QueryOperator::Ne => {
                    query_doc.insert(field_name, doc! { "$ne": bson_value });
                },
                QueryOperator::Gt => {
                    query_doc.insert(field_name, doc! { "$gt": bson_value });
                },
                QueryOperator::Gte => {
                    query_doc.insert(field_name, doc! { "$gte": bson_value });
                },
                QueryOperator::Lt => {
                    query_doc.insert(field_name, doc! { "$lt": bson_value });
                },
                QueryOperator::Lte => {
                    query_doc.insert(field_name, doc! { "$lte": bson_value });
                },
                QueryOperator::Contains => {
                    if let Bson::String(s) = bson_value {
                        let regex_doc = doc! { "$regex": s.clone(), "$options": "i" };
                        println!("[MongoDB] 处理Contains操作符(字符串): {} = {:?}", field_name, regex_doc);
                        query_doc.insert(field_name, regex_doc);
                    } else {
                        let in_doc = doc! { "$in": [bson_value.clone()] };
                        println!("[MongoDB] 处理Contains操作符(非字符串): {} = {:?}", field_name, in_doc);
                        query_doc.insert(field_name, in_doc);
                    }
                },
                QueryOperator::StartsWith => {
                    if let Bson::String(s) = bson_value {
                        query_doc.insert(field_name, doc! { "$regex": format!("^{}", regex::escape(&s)), "$options": "i" });
                    }
                },
                QueryOperator::EndsWith => {
                    if let Bson::String(s) = bson_value {
                        query_doc.insert(field_name, doc! { "$regex": format!("{}$", regex::escape(&s)), "$options": "i" });
                    }
                },
                QueryOperator::In => {
                    if let Bson::Array(arr) = bson_value {
                        let in_doc = doc! { "$in": arr.clone() };
                        println!("[MongoDB] 处理In操作符: {} = {:?}", field_name, in_doc);
                        query_doc.insert(field_name, in_doc);
                    } else {
                        println!("[MongoDB] In操作符期望数组类型，但得到: {:?}", bson_value);
                    }
                },
                QueryOperator::NotIn => {
                    if let Bson::Array(arr) = bson_value {
                        query_doc.insert(field_name, doc! { "$nin": arr });
                    }
                },
                QueryOperator::Regex => {
                    if let Bson::String(s) = bson_value {
                        query_doc.insert(field_name, doc! { "$regex": s, "$options": "i" });
                    }
                },
                QueryOperator::Exists => {
                    if let Bson::Boolean(exists) = bson_value {
                        query_doc.insert(field_name, doc! { "$exists": exists });
                    }
                },
                QueryOperator::IsNull => {
                    query_doc.insert(field_name, Bson::Null);
                },
                QueryOperator::IsNotNull => {
                    query_doc.insert(field_name, doc! { "$ne": Bson::Null });
                },
            }
        }
        
        println!("[MongoDB] 最终查询文档: {:?}", query_doc);
        Ok(query_doc)
    }

    /// 构建条件组合查询文档
    fn build_condition_groups_document(&self, condition_groups: &[QueryConditionGroup]) -> QuickDbResult<Document> {
        println!("[MongoDB] 开始构建条件组查询文档，组数量: {}", condition_groups.len());
        if condition_groups.is_empty() {
            println!("[MongoDB] 条件组为空，返回空文档");
            return Ok(Document::new());
        }
        
        if condition_groups.len() == 1 {
            // 单个条件组，直接构建
            println!("[MongoDB] 单个条件组，直接构建");
            let group = &condition_groups[0];
            return self.build_single_condition_group_document(group);
        }
        
        // 多个条件组，使用 $and 连接
        println!("[MongoDB] 多个条件组，使用$and连接");
        let mut group_docs = Vec::new();
        for (index, group) in condition_groups.iter().enumerate() {
            println!("[MongoDB] 处理条件组[{}]: {:?}", index, group);
            let group_doc = self.build_single_condition_group_document(group)?;
            println!("[MongoDB] 条件组[{}]生成的文档: {:?}", index, group_doc);
            if !group_doc.is_empty() {
                group_docs.push(group_doc);
            }
        }
        
        let final_doc = if group_docs.is_empty() {
            println!("[MongoDB] 所有条件组都为空，返回空文档");
            Document::new()
        } else if group_docs.len() == 1 {
            println!("[MongoDB] 只有一个有效条件组，直接返回");
            group_docs.into_iter().next().unwrap()
        } else {
            println!("[MongoDB] 多个有效条件组，使用$and连接");
            doc! { "$and": group_docs }
        };
        
        println!("[MongoDB] 条件组最终文档: {:?}", final_doc);
        Ok(final_doc)
    }
    
    /// 构建单个条件组文档
    fn build_single_condition_group_document(&self, group: &QueryConditionGroup) -> QuickDbResult<Document> {
        println!("[MongoDB] 构建单个条件组文档: {:?}", group);
        match group {
            QueryConditionGroup::Single(condition) => {
                println!("[MongoDB] 处理单个条件: {:?}", condition);
                self.build_query_document(&[condition.clone()])
             },
            QueryConditionGroup::Group { conditions, operator } => {
                println!("[MongoDB] 处理条件组，操作符: {:?}, 条件数量: {}", operator, conditions.len());
                if conditions.is_empty() {
                    println!("[MongoDB] 条件组为空");
                    return Ok(Document::new());
                }
                
                if conditions.len() == 1 {
                     // 单个条件组，递归处理
                     println!("[MongoDB] 条件组只有一个条件，递归处理");
                     return self.build_single_condition_group_document(&conditions[0]);
                 }
                
                // 多个条件组，根据逻辑操作符连接
                println!("[MongoDB] 处理多个条件，使用{:?}操作符", operator);
                 let condition_docs: Result<Vec<Document>, QuickDbError> = conditions
                     .iter()
                     .enumerate()
                     .map(|(i, condition_group)| {
                         println!("[MongoDB] 处理子条件[{}]: {:?}", i, condition_group);
                         let doc = self.build_single_condition_group_document(condition_group)?;
                         println!("[MongoDB] 子条件[{}]生成文档: {:?}", i, doc);
                         Ok(doc)
                     })
                     .collect();
                
                let condition_docs = condition_docs?;
                let non_empty_docs: Vec<Document> = condition_docs.into_iter()
                    .filter(|doc| !doc.is_empty())
                    .collect();
                
                println!("[MongoDB] 有效文档数量: {}", non_empty_docs.len());
                
                if non_empty_docs.is_empty() {
                    println!("[MongoDB] 没有有效文档");
                    return Ok(Document::new());
                }
                
                if non_empty_docs.len() == 1 {
                    println!("[MongoDB] 只有一个有效文档，直接返回");
                    return Ok(non_empty_docs.into_iter().next().unwrap());
                }
                
                let result_doc = match operator {
                    LogicalOperator::And => {
                        println!("[MongoDB] 使用$and连接文档");
                        doc! { "$and": non_empty_docs }
                    },
                    LogicalOperator::Or => {
                        println!("[MongoDB] 使用$or连接文档");
                        doc! { "$or": non_empty_docs }
                    },
                };
                
                println!("[MongoDB] 条件组最终结果: {:?}", result_doc);
                Ok(result_doc)
            }
        }
    }

    /// 构建更新文档
    fn build_update_document(&self, data: &HashMap<String, DataValue>) -> Document {
        let mut update_doc = Document::new();
        let mut set_doc = Document::new();
        
        // 映射字段名（id -> _id）
        let mapped_data = self.map_data_fields(data);
        for (key, value) in &mapped_data {
            if key != "_id" { // MongoDB的_id字段不能更新
                set_doc.insert(key, self.data_value_to_bson(value));
            }
        }
        
        if !set_doc.is_empty() {
            update_doc.insert("$set", set_doc);
        }
        
        update_doc
    }

    /// 获取集合引用
    fn get_collection(&self, db: &mongodb::Database, table: &str) -> Collection<Document> {
        db.collection::<Document>(table)
    }
    
    /// 将用户字段名映射到MongoDB字段名（id -> _id）
    fn map_field_name(&self, field_name: &str) -> String {
        if field_name == "id" {
            "_id".to_string()
        } else {
            field_name.to_string()
        }
    }
    
    /// 将数据映射中的id字段转换为_id字段
    fn map_data_fields(&self, data: &HashMap<String, DataValue>) -> HashMap<String, DataValue> {
        let mut mapped_data = HashMap::new();
        for (key, value) in data {
            let mapped_key = self.map_field_name(key);
            mapped_data.insert(mapped_key, value.clone());
        }
        mapped_data
    }

    /// 直接根据ID查询数据库（无缓存，内部方法）
    async fn query_database_by_id(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        id: &DataValue,
    ) -> QuickDbResult<Option<DataValue>> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);

            let query = match id {
                DataValue::String(id_str) => {
                    // 处理ObjectId格式：ObjectId("xxx") 或直接是ObjectId字符串
                    let actual_id = if id_str.starts_with("ObjectId(\"") && id_str.ends_with("\")") {
                        // 提取ObjectId字符串部分
                        &id_str[10..id_str.len()-2]
                    } else {
                        id_str
                    };
                    // 尝试解析为ObjectId，如果失败则作为字符串查询
                    if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(actual_id) {
                        doc! { "_id": object_id }
                    } else {
                        doc! { "_id": actual_id }
                    }
                },
                DataValue::Int(i) => doc! { "_id": *i },
                DataValue::Float(f) => doc! { "_id": *f },
                DataValue::Bool(b) => doc! { "_id": *b },
                DataValue::Uuid(uuid) => doc! { "_id": uuid.to_string() },
                _ => doc! { "_id": format!("{:?}", id) },
            };

            debug!("执行MongoDB根据ID查询: {:?}", query);

            let result = collection.find_one(query, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行MongoDB查询失败: {}", e),
                })?;

            match result {
                Some(doc) => {
                    let data_value = self.bson_to_data_value(&Bson::Document(doc))?;
                    match data_value {
                        DataValue::Object(data_map) => Ok(Some(DataValue::Object(data_map))),
                        _ => Ok(Some(data_value)),
                    }
                },
                None => Ok(None),
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);

            // 构建MongoDB查询文档
            let query_doc = self.build_condition_groups_document(condition_groups)
                .unwrap_or_else(|_| Document::new());

            debug!("执行MongoDB条件组查询: {:?}", query_doc);

            let mut cursor = collection.find(query_doc, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("执行MongoDB查询失败: {}", e),
                })?;

            // 使用MongoDB原生的cursor遍历方法
            let mut results = Vec::new();
            while cursor.advance().await.map_err(|e| QuickDbError::QueryError {
                message: format!("MongoDB游标遍历失败: {}", e),
            })? {
                let doc = cursor.deserialize_current().map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB文档反序列化失败: {}", e),
                })?;
                let data_value = self.bson_to_data_value(&Bson::Document(doc))?;
                match data_value {
                    DataValue::Object(data_map) => results.push(DataValue::Object(data_map)),
                    _ => results.push(data_value),
                }
            }

            Ok(results)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }
}

#[async_trait]
impl DatabaseAdapter for MongoAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<DataValue> {
        if let DatabaseConnection::MongoDB(db) = connection {
            // 自动建表逻辑：检查集合是否存在，如果不存在则创建
            if !self.table_exists(connection, table).await? {
                info!("集合 {} 不存在，正在自动创建", table);
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
                info!("自动创建MongoDB集合 '{}' 成功", table);
            }
            
            let collection = self.get_collection(db, table);
            
            // 映射字段名（id -> _id）
            let mapped_data = self.map_data_fields(data);
            let mut doc = Document::new();
            for (key, value) in &mapped_data {
                // 跳过_id字段如果值为Null，让MongoDB自动生成
                if key == "_id" && matches!(value, DataValue::Null) {
                    continue;
                }
                doc.insert(key, self.data_value_to_bson(value));
            }

            info!("执行MongoDB插入到集合 {}: {:?}", table, doc);
            
            let result = collection.insert_one(doc, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB插入失败: {}", e),
                })?;
            
            let mut result_map = HashMap::new();
            result_map.insert("_id".to_string(), DataValue::String(result.inserted_id.to_string()));
            Ok(DataValue::Object(result_map))
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
            DataValue::Float(f) => f.to_string(),
            DataValue::Bool(b) => b.to_string(),
            DataValue::Uuid(u) => u.to_string(),
            _ => format!("{:?}", id),
        };
        let cache_key = format!("mongodb:{}:record:{}", table, id_str);

        // 先检查缓存
        match CacheOps::get(&cache_key).await {
            Ok((true, Some(vec_data))) => {
                // 缓存命中
                if vec_data.len() == 1 {
                    info!("🔥 MongoDB缓存命中: {} (结果数量: {})", cache_key, vec_data.len());
                    return Ok(Some(vec_data.into_iter().next().unwrap()));
                } else {
                    info!("🔥 MongoDB缓存命中: {} (结果数量: {})", cache_key, vec_data.len());
                    return Ok(None);
                }
            }
            Ok((true, None)) => {
                // 缓存命中，但结果为空
                info!("🔥 MongoDB缓存命中: {} (空结果)", cache_key);
                return Ok(None);
            }
            Ok((false, _)) => {
                // 缓存未命中
                info!("❌ MongoDB缓存未命中: {}", cache_key);
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
                info!("📦 MongoDB设置缓存: {} (数据量: 1)", cache_key);
            }
        } else {
            // 缓存空结果
            if let Err(e) = CacheOps::set(&cache_key, None, Some(600)).await {
                warn!("设置空缓存失败: {}", e);
            } else {
                info!("📦 MongoDB设置空缓存: {}", cache_key);
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
        let cache_key = format!("mongodb:{}:query:{}", table, query_hash);

        // 先检查缓存
        match CacheOps::get(&cache_key).await {
            Ok((true, Some(vec_data))) => {
                // 缓存命中
                info!("🔥 MongoDB缓存命中: {} (结果数量: {})", cache_key, vec_data.len());
                return Ok(vec_data);
            }
            Ok((true, None)) => {
                // 缓存命中，但结果为空
                info!("🔥 MongoDB缓存命中: {} (空结果)", cache_key);
                return Ok(Vec::new());
            }
            Ok((false, _)) => {
                // 缓存未命中
                info!("❌ MongoDB缓存未命中: {}", cache_key);
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
            info!("📦 MongoDB设置缓存: {} (数据量: {})", cache_key, result.len());
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);

            let query = self.build_query_document(conditions)?;
            let update = self.build_update_document(data);

            debug!("执行MongoDB更新: 查询={:?}, 更新={:?}", query, update);

            let result = collection.update_many(query, update, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB更新失败: {}", e),
                })?;

            // 更新成功后清理相关缓存
            if result.modified_count > 0 {
                // 清理表相关的所有缓存
                if let Err(e) = CacheOps::clear_table("mongodb", table).await {
                    warn!("清理MongoDB表缓存失败: {}", e);
                } else {
                    info!("✅ MongoDB更新后清理表缓存: mongodb:{}", table);
                }
            }

            Ok(result.modified_count)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
            field: "_id".to_string(), // MongoDB使用_id作为主键
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);

            let query = self.build_query_document(conditions)?;

            debug!("执行MongoDB删除: {:?}", query);

            let result = collection.delete_many(query, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB删除失败: {}", e),
                })?;

            // 删除成功后清理相关缓存
            if result.deleted_count > 0 {
                // 清理表相关的所有缓存
                if let Err(e) = CacheOps::clear_table("mongodb", table).await {
                    warn!("清理MongoDB表缓存失败: {}", e);
                } else {
                    info!("✅ MongoDB删除后清理表缓存: mongodb:{}", table);
                }
            }

            Ok(result.deleted_count)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
            field: "_id".to_string(), // MongoDB使用_id作为主键
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = self.build_query_document(conditions)?;
            
            println!("执行MongoDB计数: {:?}", query);
            
            let count = collection.count_documents(query, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB计数失败: {}", e),
                })?;
            
            Ok(count)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        _fields: &HashMap<String, FieldType>,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MongoDB(db) = connection {
            // MongoDB是无模式的，集合会在第一次插入时自动创建
            // 这里我们可以创建集合并设置一些选项
            let options = mongodb::options::CreateCollectionOptions::default();
            
            println!("创建MongoDB集合: {}", table);
            
            match db.create_collection(table, options).await {
                Ok(_) => {},
                Err(e) => {
                    // 如果集合已存在，忽略错误
                    if !e.to_string().contains("already exists") {
                        return Err(QuickDbError::QueryError {
                            message: format!("创建MongoDB集合失败: {}", e),
                        });
                    }
                }
            }
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let mut index_doc = Document::new();
            for field in fields {
                index_doc.insert(field, 1); // 1表示升序索引
            }
            
            let mut index_options = mongodb::options::IndexOptions::default();
            index_options.name = Some(index_name.to_string());
            index_options.unique = Some(unique);
            
            let index_model = mongodb::IndexModel::builder()
                .keys(index_doc)
                .options(index_options)
                .build();
            
            println!("创建MongoDB索引: {} 在集合 {}", index_name, table);
            
            collection.create_index(index_model, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("创建MongoDB索引失败: {}", e),
                })?;
            
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn table_exists(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<bool> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection_names = db.list_collection_names(None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("检查MongoDB集合是否存在失败: {}", e),
                })?;
            
            Ok(collection_names.contains(&table.to_string()))
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn drop_table(
        &self,
        connection: &DatabaseConnection,
        table: &str,
    ) -> QuickDbResult<()> {
        if let DatabaseConnection::MongoDB(db) = connection {
            println!("执行MongoDB删除集合: {}", table);
            
            let collection = db.collection::<mongodb::bson::Document>(table);
            collection.drop(None).await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("删除MongoDB集合失败: {}", e),
                })?;
            
            info!("成功删除MongoDB集合: {}", table);
            Ok(())
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }
}