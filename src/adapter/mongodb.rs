//! MongoDB数据库适配器
//! 
//! 使用mongodb库实现真实的MongoDB文档操作

use super::DatabaseAdapter;
use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::pool::DatabaseConnection;
use crate::table::{TableManager, TableSchema, ColumnType};
use crate::model::FieldType;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use mongodb::{bson::{doc, Document, Bson}, Collection};
use zerg_creep::{info, error, warn, debug};

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

    /// 将BSON文档转换为JSON值
    fn document_to_json(&self, doc: &Document) -> QuickDbResult<Value> {
        let json_str = doc.to_string();
        serde_json::from_str(&json_str)
            .map_err(|e| QuickDbError::QueryError {
                message: format!("转换BSON文档为JSON失败: {}", e),
            })
    }

    /// 构建MongoDB查询文档
    fn build_query_document(&self, conditions: &[QueryCondition]) -> QuickDbResult<Document> {
        let mut query_doc = Document::new();
        
        for condition in conditions {
            let field_name = &condition.field;
            let bson_value = self.data_value_to_bson(&condition.value);
            
            match condition.operator {
                QueryOperator::Eq => {
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
                        query_doc.insert(field_name, doc! { "$regex": s, "$options": "i" });
                    } else {
                        query_doc.insert(field_name, doc! { "$in": [bson_value] });
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
                        query_doc.insert(field_name, doc! { "$in": arr });
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
        
        Ok(query_doc)
    }

    /// 构建条件组合查询文档
    fn build_condition_groups_document(&self, condition_groups: &[QueryConditionGroup]) -> QuickDbResult<Document> {
        if condition_groups.is_empty() {
            return Ok(Document::new());
        }
        
        if condition_groups.len() == 1 {
            // 单个条件组，直接构建
            let group = &condition_groups[0];
            return self.build_single_condition_group_document(group);
        }
        
        // 多个条件组，使用 $and 连接
        let mut group_docs = Vec::new();
        for group in condition_groups {
            let group_doc = self.build_single_condition_group_document(group)?;
            if !group_doc.is_empty() {
                group_docs.push(group_doc);
            }
        }
        
        if group_docs.is_empty() {
            Ok(Document::new())
        } else if group_docs.len() == 1 {
            Ok(group_docs.into_iter().next().unwrap())
        } else {
            Ok(doc! { "$and": group_docs })
        }
    }
    
    /// 构建单个条件组文档
    fn build_single_condition_group_document(&self, group: &QueryConditionGroup) -> QuickDbResult<Document> {
        match group {
            QueryConditionGroup::Single(condition) => {
                 self.build_query_document(&[condition.clone()])
             },
            QueryConditionGroup::Group { conditions, operator } => {
                if conditions.is_empty() {
                    return Ok(Document::new());
                }
                
                if conditions.len() == 1 {
                     // 单个条件组，递归处理
                     return self.build_single_condition_group_document(&conditions[0]);
                 }
                
                // 多个条件组，根据逻辑操作符连接
                 let condition_docs: Result<Vec<Document>, QuickDbError> = conditions
                     .iter()
                     .map(|condition_group| self.build_single_condition_group_document(condition_group))
                     .collect();
                
                let condition_docs = condition_docs?;
                let non_empty_docs: Vec<Document> = condition_docs.into_iter()
                    .filter(|doc| !doc.is_empty())
                    .collect();
                
                if non_empty_docs.is_empty() {
                    return Ok(Document::new());
                }
                
                if non_empty_docs.len() == 1 {
                    return Ok(non_empty_docs.into_iter().next().unwrap());
                }
                
                match operator {
                    LogicalOperator::And => Ok(doc! { "$and": non_empty_docs }),
                    LogicalOperator::Or => Ok(doc! { "$or": non_empty_docs }),
                }
            }
        }
    }

    /// 构建更新文档
    fn build_update_document(&self, data: &HashMap<String, DataValue>) -> Document {
        let mut update_doc = Document::new();
        let mut set_doc = Document::new();
        
        for (key, value) in data {
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
}

#[async_trait]
impl DatabaseAdapter for MongoAdapter {
    async fn create(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        data: &HashMap<String, DataValue>,
    ) -> QuickDbResult<Value> {
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
            
            let mut doc = Document::new();
            for (key, value) in data {
                doc.insert(key, self.data_value_to_bson(value));
            }
            
            debug!("执行MongoDB插入到集合 {}: {:?}", table, doc);
            
            let result = collection.insert_one(doc, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB插入失败: {}", e),
                })?;
            
            Ok(serde_json::json!({
                "_id": result.inserted_id.to_string()
            }))
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
    ) -> QuickDbResult<Option<Value>> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = match id {
                DataValue::String(id_str) => {
                    // 尝试解析为ObjectId，如果失败则作为字符串查询
                    if let Ok(object_id) = mongodb::bson::oid::ObjectId::parse_str(id_str) {
                        doc! { "_id": object_id }
                    } else {
                        doc! { "_id": id_str }
                    }
                },
                _ => doc! { "_id": self.data_value_to_bson(id) }
            };
            
            debug!("执行MongoDB根据ID查询: {:?}", query);
            
            let result = collection.find_one(query, None)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB查询失败: {}", e),
                })?;
            
            if let Some(doc) = result {
                Ok(Some(self.document_to_json(&doc)?))
            } else {
                Ok(None)
            }
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = self.build_query_document(conditions)?;
            
            debug!("执行MongoDB查询: {:?}", query);
            
            let mut find_options = mongodb::options::FindOptions::default();
            
            // 添加排序
            if !options.sort.is_empty() {
                let mut sort_doc = Document::new();
                for sort_field in &options.sort {
                    let sort_value = match sort_field.direction {
                        SortDirection::Asc => 1,
                        SortDirection::Desc => -1,
                    };
                    sort_doc.insert(&sort_field.field, sort_value);
                }
                find_options.sort = Some(sort_doc);
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                find_options.limit = Some(pagination.limit as i64);
                find_options.skip = Some(pagination.skip);
            }
            
            let mut cursor = collection.find(query, find_options)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB查询失败: {}", e),
                })?;
            
            let mut results = Vec::new();
            while cursor.advance().await.map_err(|e| QuickDbError::QueryError {
                message: format!("MongoDB游标遍历失败: {}", e),
            })? {
                let doc = cursor.deserialize_current().map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB文档反序列化失败: {}", e),
                })?;
                results.push(self.document_to_json(&doc)?);
            }
            
            Ok(results)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
            })
        }
    }

    async fn find_with_groups(
        &self,
        connection: &DatabaseConnection,
        table: &str,
        condition_groups: &[QueryConditionGroup],
        options: &QueryOptions,
    ) -> QuickDbResult<Vec<Value>> {
        if let DatabaseConnection::MongoDB(db) = connection {
            let collection = self.get_collection(db, table);
            
            let query = self.build_condition_groups_document(condition_groups)?;
            
            debug!("执行MongoDB条件组合查询: {:?}", query);
            
            let mut find_options = mongodb::options::FindOptions::default();
            
            // 添加排序
            if !options.sort.is_empty() {
                let mut sort_doc = Document::new();
                for sort_field in &options.sort {
                    let sort_value = match sort_field.direction {
                        SortDirection::Asc => 1,
                        SortDirection::Desc => -1,
                    };
                    sort_doc.insert(&sort_field.field, sort_value);
                }
                find_options.sort = Some(sort_doc);
            }
            
            // 添加分页
            if let Some(pagination) = &options.pagination {
                find_options.limit = Some(pagination.limit as i64);
                find_options.skip = Some(pagination.skip);
            }
            
            let mut cursor = collection.find(query, find_options)
                .await
                .map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB条件组合查询失败: {}", e),
                })?;
            
            let mut results = Vec::new();
            while cursor.advance().await.map_err(|e| QuickDbError::QueryError {
                message: format!("MongoDB游标遍历失败: {}", e),
            })? {
                let doc = cursor.deserialize_current().map_err(|e| QuickDbError::QueryError {
                    message: format!("MongoDB文档反序列化失败: {}", e),
                })?;
                results.push(self.document_to_json(&doc)?);
            }
            
            Ok(results)
        } else {
            Err(QuickDbError::ConnectionError {
                message: "连接类型不匹配，期望MongoDB连接".to_string(),
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
            
            debug!("执行MongoDB计数: {:?}", query);
            
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
            
            debug!("创建MongoDB集合: {}", table);
            
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
            
            debug!("创建MongoDB索引: {} 在集合 {}", index_name, table);
            
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
}