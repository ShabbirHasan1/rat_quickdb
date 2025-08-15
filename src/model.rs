//! 模型定义系统
//! 
//! 参考mongoengine的设计，支持通过结构体定义数据表结构
//! 提供字段类型、验证、索引等功能

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::*;
use crate::odm::{self, OdmOperations};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::marker::PhantomData;
use zerg_creep::{debug, error, info, warn};

/// 字段类型枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// 字符串类型
    String {
        max_length: Option<usize>,
        min_length: Option<usize>,
        regex: Option<String>,
    },
    /// 整数类型
    Integer {
        min_value: Option<i64>,
        max_value: Option<i64>,
    },
    /// 浮点数类型
    Float {
        min_value: Option<f64>,
        max_value: Option<f64>,
    },
    /// 布尔类型
    Boolean,
    /// 日期时间类型
    DateTime,
    /// UUID类型
    Uuid,
    /// JSON类型
    Json,
    /// 数组类型
    Array {
        item_type: Box<FieldType>,
        max_items: Option<usize>,
        min_items: Option<usize>,
    },
    /// 对象类型
    Object {
        fields: HashMap<String, FieldDefinition>,
    },
    /// 引用类型（外键）
    Reference {
        target_collection: String,
    },
}

/// 字段定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// 字段类型
    pub field_type: FieldType,
    /// 是否必填
    pub required: bool,
    /// 默认值
    pub default: Option<DataValue>,
    /// 是否唯一
    pub unique: bool,
    /// 是否建立索引
    pub indexed: bool,
    /// 字段描述
    pub description: Option<String>,
    /// 自定义验证函数名
    pub validator: Option<String>,
}

impl FieldDefinition {
    /// 创建新的字段定义
    pub fn new(field_type: FieldType) -> Self {
        Self {
            field_type,
            required: false,
            default: None,
            unique: false,
            indexed: false,
            description: None,
            validator: None,
        }
    }

    /// 设置为必填字段
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// 设置默认值
    pub fn default_value(mut self, value: DataValue) -> Self {
        self.default = Some(value);
        self
    }

    /// 设置为唯一字段
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// 设置为索引字段
    pub fn indexed(mut self) -> Self {
        self.indexed = true;
        self
    }

    /// 设置字段描述
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// 设置验证函数
    pub fn validator(mut self, validator_name: &str) -> Self {
        self.validator = Some(validator_name.to_string());
        self
    }

    /// 验证字段值
    pub fn validate(&self, value: &DataValue) -> QuickDbResult<()> {
        // 检查必填字段
        if self.required && matches!(value, DataValue::Null) {
            return Err(QuickDbError::ValidationError { field: "unknown".to_string(), message: "必填字段不能为空".to_string() });
        }

        // 如果值为空且不是必填字段，则跳过验证
        if matches!(value, DataValue::Null) {
            return Ok(());
        }

        // 根据字段类型进行验证
        match &self.field_type {
            FieldType::String { max_length, min_length, regex } => {
                if let DataValue::String(s) = value {
                    if let Some(max_len) = max_length {
                        if s.len() > *max_len {
                            return Err(QuickDbError::ValidationError {
                                field: "string_length".to_string(),
                                message: format!("字符串长度不能超过{}", max_len)
                            });
                        }
                    }
                    if let Some(min_len) = min_length {
                        if s.len() < *min_len {
                            return Err(QuickDbError::ValidationError {
                                field: "string_length".to_string(),
                                message: format!("字符串长度不能少于{}", min_len)
                            });
                        }
                    }
                    if let Some(pattern) = regex {
                        let regex = regex::Regex::new(pattern)
                            .map_err(|e| QuickDbError::ValidationError {
                                field: "regex".to_string(),
                                message: format!("正则表达式无效: {}", e)
                            })?;
                        if !regex.is_match(s) {
                            return Err(QuickDbError::ValidationError {
                                field: "regex_match".to_string(),
                                message: "字符串不匹配正则表达式".to_string()
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望字符串类型".to_string()
                    });
                }
            }
            FieldType::Integer { min_value, max_value } => {
                if let DataValue::Int(i) = value {
                    if let Some(min_val) = min_value {
                        if *i < *min_val {
                            return Err(QuickDbError::ValidationError {
                                field: "integer_range".to_string(),
                                message: format!("整数值不能小于{}", min_val)
                            });
                        }
                    }
                    if let Some(max_val) = max_value {
                        if *i > *max_val {
                            return Err(QuickDbError::ValidationError {
                                field: "integer_range".to_string(),
                                message: format!("整数值不能大于{}", max_val)
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望整数类型".to_string()
                    });
                }
            }
            FieldType::Float { min_value, max_value } => {
                if let DataValue::Float(f) = value {
                    if let Some(min_val) = min_value {
                        if *f < *min_val {
                            return Err(QuickDbError::ValidationError {
                                field: "float_range".to_string(),
                                message: format!("浮点数值不能小于{}", min_val)
                            });
                        }
                    }
                    if let Some(max_val) = max_value {
                        if *f > *max_val {
                            return Err(QuickDbError::ValidationError {
                                field: "float_range".to_string(),
                                message: format!("浮点数值不能大于{}", max_val)
                            });
                        }
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望浮点数类型".to_string()
                    });
                }
            }
            FieldType::Boolean => {
                if !matches!(value, DataValue::Bool(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望布尔类型".to_string()
                    });
                }
            }
            FieldType::DateTime => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望日期时间类型".to_string()
                    });
                }
            }
            FieldType::Uuid => {
                if let DataValue::String(s) = value {
                    if uuid::Uuid::parse_str(s).is_err() {
                        return Err(QuickDbError::ValidationError {
                            field: "uuid_format".to_string(),
                            message: "无效的UUID格式".to_string()
                        });
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望UUID字符串".to_string()
                    });
                }
            }
            FieldType::Json => {
                // JSON类型可以接受任何值
            }
            FieldType::Array { item_type, max_items, min_items } => {
                if let DataValue::Array(arr) = value {
                    if let Some(max_items) = max_items {
                        if arr.len() > *max_items {
                            return Err(QuickDbError::ValidationError {
                                field: "array_size".to_string(),
                                message: format!("数组元素数量不能超过{}", max_items)
                            });
                        }
                    }
                    if let Some(min_items) = min_items {
                        if arr.len() < *min_items {
                            return Err(QuickDbError::ValidationError {
                                field: "array_size".to_string(),
                                message: format!("数组元素数量不能少于{}", min_items)
                            });
                        }
                    }
                    // 验证数组中的每个元素
                    let item_field = FieldDefinition::new((**item_type).clone());
                    for item in arr {
                        item_field.validate(item)?;
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望数组类型".to_string()
                    });
                }
            }
            FieldType::Object { fields } => {
                if let DataValue::Object(obj) = value {
                    // 验证对象中的每个字段
                    for (field_name, field_def) in fields {
                        let field_value = obj.get(field_name).unwrap_or(&DataValue::Null);
                        field_def.validate(field_value)?;
                    }
                } else {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望对象类型".to_string()
                    });
                }
            }
            FieldType::Reference { target_collection: _ } => {
                // 引用类型通常是字符串ID
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "reference_type".to_string(),
                        message: "引用字段必须是字符串ID".to_string()
                    });
                }
            }
        }

        Ok(())
    }
}

/// 模型元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMeta {
    /// 集合/表名
    pub collection_name: String,
    /// 数据库别名
    pub database_alias: Option<String>,
    /// 字段定义
    pub fields: HashMap<String, FieldDefinition>,
    /// 索引定义
    pub indexes: Vec<IndexDefinition>,
    /// 模型描述
    pub description: Option<String>,
}

/// 索引定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// 索引字段
    pub fields: Vec<String>,
    /// 是否唯一索引
    pub unique: bool,
    /// 索引名称
    pub name: Option<String>,
}

/// 模型特征
/// 
/// 所有模型都必须实现这个特征
pub trait Model: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// 获取模型元数据
    fn meta() -> ModelMeta;
    
    /// 获取集合/表名
    fn collection_name() -> String {
        Self::meta().collection_name
    }
    
    /// 获取数据库别名
    fn database_alias() -> Option<String> {
        Self::meta().database_alias
    }
    
    /// 验证模型数据
    fn validate(&self) -> QuickDbResult<()> {
        let meta = Self::meta();
        let data = self.to_data_map()?;
        
        for (field_name, field_def) in &meta.fields {
            let field_value = data.get(field_name).unwrap_or(&DataValue::Null);
            field_def.validate(field_value)?;
        }
        
        Ok(())
    }
    
    /// 转换为数据映射
    fn to_data_map(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        let json_str = serde_json::to_string(self)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?;
        let json_value: JsonValue = serde_json::from_str(&json_str)
            .map_err(|e| QuickDbError::SerializationError { message: format!("解析JSON失败: {}", e) })?;
        
        let mut data_map = HashMap::new();
        if let JsonValue::Object(obj) = json_value {
            for (key, value) in obj {
                data_map.insert(key, DataValue::from_json(value));
            }
        }
        
        Ok(data_map)
    }
    
    /// 从数据映射创建模型实例
    fn from_data_map(data: HashMap<String, DataValue>) -> QuickDbResult<Self> {
        let mut json_obj = serde_json::Map::new();
        for (key, value) in data {
            json_obj.insert(key, value.to_json());
        }
        
        let json_value = JsonValue::Object(json_obj);
        let result = serde_json::from_value(json_value)
            .map_err(|e| QuickDbError::SerializationError { message: format!("反序列化失败: {}", e) })?;
        Ok(result)
    }
}

/// 模型操作特征
/// 
/// 提供模型的CRUD操作
#[async_trait]
pub trait ModelOperations<T: Model> {
    /// 保存模型
    async fn save(&self) -> QuickDbResult<String>;
    
    /// 根据ID查找模型
    async fn find_by_id(id: &str) -> QuickDbResult<Option<T>>;
    
    /// 查找多个模型
    async fn find(conditions: Vec<QueryCondition>, options: Option<QueryOptions>) -> QuickDbResult<Vec<T>>;
    
    /// 更新模型
    async fn update(&self, updates: HashMap<String, DataValue>) -> QuickDbResult<bool>;
    
    /// 删除模型
    async fn delete(&self) -> QuickDbResult<bool>;
    
    /// 统计模型数量
    async fn count(conditions: Vec<QueryCondition>) -> QuickDbResult<u64>;
    
    /// 检查模型是否存在
    async fn exists(conditions: Vec<QueryCondition>) -> QuickDbResult<bool>;
}

/// 模型管理器
/// 
/// 提供模型的通用操作实现
pub struct ModelManager<T: Model> {
    _phantom: PhantomData<T>,
}

impl<T: Model> ModelManager<T> {
    /// 创建新的模型管理器
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<T: Model> ModelOperations<T> for ModelManager<T> {
    async fn save(&self) -> QuickDbResult<String> {
        // 这个方法需要模型实例，应该在具体的模型实现中调用
        Err(QuickDbError::ValidationError {
            field: "save".to_string(),
            message: "save方法需要在模型实例上调用".to_string()
        })
    }
    
    async fn find_by_id(id: &str) -> QuickDbResult<Option<T>> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();
        
        debug!("根据ID查找模型: collection={}, id={}", collection_name, id);
        
        let result = odm::find_by_id(
            &collection_name,
            id,
            database_alias.as_deref(),
        ).await?;
        
        if let Some(json_str) = result {
            let json_value: JsonValue = serde_json::from_str(&json_str)
                .map_err(|e| QuickDbError::SerializationError { message: format!("解析JSON失败: {}", e) })?;
            
            let model: T = serde_json::from_value(json_value)
                .map_err(|e| QuickDbError::SerializationError { message: format!("反序列化模型失败: {}", e) })?;
            
            Ok(Some(model))
        } else {
            Ok(None)
        }
    }
    
    async fn find(conditions: Vec<QueryCondition>, options: Option<QueryOptions>) -> QuickDbResult<Vec<T>> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();
        
        debug!("查找模型: collection={}", collection_name);
        
        let result = odm::find(
            &collection_name,
            conditions,
            options,
            database_alias.as_deref(),
        ).await?;
        
        let json_value: JsonValue = serde_json::from_str(&result)
            .map_err(|e| QuickDbError::SerializationError { message: format!("解析JSON失败: {}", e) })?;
        
        if let JsonValue::Array(arr) = json_value {
            let mut models = Vec::new();
            for item in arr {
                let model: T = serde_json::from_value(item)
                    .map_err(|e| QuickDbError::SerializationError { message: format!("反序列化模型失败: {}", e) })?;
                models.push(model);
            }
            Ok(models)
        } else {
            Err(QuickDbError::SerializationError {
                message: "查询结果不是数组格式".to_string()
            })
        }
    }
    
    async fn update(&self, _updates: HashMap<String, DataValue>) -> QuickDbResult<bool> {
        // 这个方法需要模型实例，应该在具体的模型实现中调用
        Err(QuickDbError::ValidationError {
            field: "update".to_string(),
            message: "update方法需要在模型实例上调用".to_string()
        })
    }
    
    async fn delete(&self) -> QuickDbResult<bool> {
        // 这个方法需要模型实例，应该在具体的模型实现中调用
        Err(QuickDbError::ValidationError {
            field: "delete".to_string(),
            message: "delete方法需要在模型实例上调用".to_string()
        })
    }
    
    async fn count(conditions: Vec<QueryCondition>) -> QuickDbResult<u64> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();
        
        debug!("统计模型数量: collection={}", collection_name);
        
        odm::count(
            &collection_name,
            conditions,
            database_alias.as_deref(),
        ).await
    }
    
    async fn exists(conditions: Vec<QueryCondition>) -> QuickDbResult<bool> {
        let collection_name = T::collection_name();
        let database_alias = T::database_alias();
        
        debug!("检查模型是否存在: collection={}", collection_name);
        
        odm::exists(
            &collection_name,
            conditions,
            database_alias.as_deref(),
        ).await
    }
}

/// 便捷宏：定义模型字段类型
#[macro_export]
macro_rules! field_types {
    (string) => {
        $crate::model::FieldType::String {
            max_length: None,
            min_length: None,
            regex: None,
        }
    };
    (string, max_length = $max:expr) => {
        $crate::model::FieldType::String {
            max_length: Some($max),
            min_length: None,
            regex: None,
        }
    };
    (string, min_length = $min:expr) => {
        $crate::model::FieldType::String {
            max_length: None,
            min_length: Some($min),
            regex: None,
        }
    };
    (string, max_length = $max:expr, min_length = $min:expr) => {
        $crate::model::FieldType::String {
            max_length: Some($max),
            min_length: Some($min),
            regex: None,
        }
    };
    (integer) => {
        $crate::model::FieldType::Integer {
            min_value: None,
            max_value: None,
        }
    };
    (integer, min = $min:expr) => {
        $crate::model::FieldType::Integer {
            min_value: Some($min),
            max_value: None,
        }
    };
    (integer, max = $max:expr) => {
        $crate::model::FieldType::Integer {
            min_value: None,
            max_value: Some($max),
        }
    };
    (integer, min = $min:expr, max = $max:expr) => {
        $crate::model::FieldType::Integer {
            min_value: Some($min),
            max_value: Some($max),
        }
    };
    (float) => {
        $crate::model::FieldType::Float {
            min_value: None,
            max_value: None,
        }
    };
    (boolean) => {
        $crate::model::FieldType::Boolean
    };
    (datetime) => {
        $crate::model::FieldType::DateTime
    };
    (uuid) => {
        $crate::model::FieldType::Uuid
    };
    (json) => {
        $crate::model::FieldType::Json
    };
    (array, $item_type:expr) => {
        $crate::model::FieldType::Array {
            item_type: Box::new($item_type),
            max_items: None,
            min_items: None,
        }
    };
    (reference, $target:expr) => {
        $crate::model::FieldType::Reference {
            target_collection: $target.to_string(),
        }
    };
}

/// 便捷宏：定义模型
#[macro_export]
macro_rules! define_model {
    (
        $(#[$meta:meta])*
        struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field:ident: $field_type:ty,
            )*
        }
        
        collection = $collection:expr,
        $(
            database = $database:expr,
        )?
        fields = {
            $(
                $field_name:ident: $field_def:expr,
            )*
        }
        $(
            indexes = [
                $(
                    { fields: [$($index_field:expr),*], unique: $unique:expr $(, name: $index_name:expr)? },
                )*
            ],
        )?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $field_type,
            )*
        }
        
        impl $crate::model::Model for $name {
            fn meta() -> $crate::model::ModelMeta {
                let mut fields = std::collections::HashMap::new();
                $(
                    fields.insert(stringify!($field_name).to_string(), $field_def);
                )*
                
                let mut indexes = Vec::new();
                $(
                    $(
                        indexes.push($crate::model::IndexDefinition {
                            fields: vec![$($index_field.to_string()),*],
                            unique: $unique,
                            name: None $(.or(Some($index_name.to_string())))?,
                        });
                    )*
                )?
                
                $crate::model::ModelMeta {
                    collection_name: $collection.to_string(),
                    database_alias: None $(.or(Some($database.to_string())))?,
                    fields,
                    indexes,
                    description: None,
                }
            }
        }
        
        impl $name {
            /// 保存模型到数据库
            pub async fn save(&self) -> $crate::error::QuickDbResult<String> {
                self.validate()?;
                let data = self.to_data_map()?;
                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();
                
                $crate::odm::create(
                    &collection_name,
                    data,
                    database_alias.as_deref(),
                ).await
            }
            
            /// 更新模型
            pub async fn update(&self, updates: std::collections::HashMap<String, $crate::types::DataValue>) -> $crate::error::QuickDbResult<bool> {
                // 这里需要模型有ID字段才能更新
                // 实际实现中需要根据具体的ID字段来处理
                Err($crate::error::QuickDbError::ValidationError {
                    field: "update".to_string(),
                    message: "更新功能需要模型有ID字段".to_string()
                })
            }
            
            /// 删除模型
            pub async fn delete(&self) -> $crate::error::QuickDbResult<bool> {
                // 这里需要模型有ID字段才能删除
                // 实际实现中需要根据具体的ID字段来处理
                Err($crate::error::QuickDbError::ValidationError {
                    field: "delete".to_string(),
                    message: "删除功能需要模型有ID字段".to_string()
                })
            }
        }
    };
}