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
use rat_logger::{debug, error, info, warn};

/// 支持直接转换为 DataValue 的 trait
/// 避免 JSON 序列化的性能开销
pub trait ToDataValue {
    fn to_data_value(&self) -> DataValue;
}

/// 为基础类型实现 ToDataValue
impl ToDataValue for String {
    fn to_data_value(&self) -> DataValue {
        DataValue::String(self.clone())
    }
}

impl ToDataValue for &str {
    fn to_data_value(&self) -> DataValue {
        DataValue::String(self.to_string())
    }
}

impl ToDataValue for i32 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Int(*self as i64)
    }
}

impl ToDataValue for i64 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Int(*self)
    }
}

impl ToDataValue for f32 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Float(*self as f64)
    }
}

impl ToDataValue for f64 {
    fn to_data_value(&self) -> DataValue {
        DataValue::Float(*self)
    }
}

impl ToDataValue for bool {
    fn to_data_value(&self) -> DataValue {
        DataValue::Bool(*self)
    }
}

// 为Vec<String>提供特定的实现，确保字符串数组被正确转换为DataValue::Array
impl ToDataValue for Vec<String> {
    fn to_data_value(&self) -> DataValue {
        // 将字符串数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter()
            .map(|s| DataValue::String(s.clone()))
            .collect();
        DataValue::Array(data_values)
    }
}

// 为Vec<i32>提供特定的实现
impl ToDataValue for Vec<i32> {
    fn to_data_value(&self) -> DataValue {
        // 将整数数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter()
            .map(|&i| DataValue::Int(i as i64))
            .collect();
        DataValue::Array(data_values)
    }
}

// 为Vec<i64>提供特定的实现
impl ToDataValue for Vec<i64> {
    fn to_data_value(&self) -> DataValue {
        // 将整数数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter()
            .map(|&i| DataValue::Int(i))
            .collect();
        DataValue::Array(data_values)
    }
}

// 为Vec<f64>提供特定的实现
impl ToDataValue for Vec<f64> {
    fn to_data_value(&self) -> DataValue {
        // 将浮点数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter()
            .map(|&f| DataValue::Float(f))
            .collect();
        DataValue::Array(data_values)
    }
}

// 为Vec<bool>提供特定的实现
impl ToDataValue for Vec<bool> {
    fn to_data_value(&self) -> DataValue {
        // 将布尔数组转换为DataValue::Array
        let data_values: Vec<DataValue> = self.iter()
            .map(|&b| DataValue::Bool(b))
            .collect();
        DataValue::Array(data_values)
    }
}

// 为HashMap<String, DataValue>提供特定的实现
impl ToDataValue for HashMap<String, DataValue> {
    fn to_data_value(&self) -> DataValue {
        // 将字典转换为DataValue::Object
        DataValue::Object(self.clone())
    }
}

// 注意：不能同时有泛型和特定类型的实现，所以移除了通用的Vec<T>实现
// 如果需要支持其他Vec类型，请添加特定的实现

impl<T> ToDataValue for Option<T>
where
    T: ToDataValue,
{
    fn to_data_value(&self) -> DataValue {
        match self {
            Some(v) => v.to_data_value(),
            None => DataValue::Null,
        }
    }
}

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
    /// 大整数类型
    BigInteger,
    /// 浮点数类型
    Float {
        min_value: Option<f64>,
        max_value: Option<f64>,
    },
    /// 双精度浮点数类型
    Double,
    /// 文本类型
    Text,
    /// 布尔类型
    Boolean,
    /// 日期时间类型
    DateTime,
    /// 日期类型
    Date,
    /// 时间类型
    Time,
    /// UUID类型
    Uuid,
    /// JSON类型
    Json,
    /// 二进制类型
    Binary,
    /// 十进制类型
    Decimal {
        precision: u8,
        scale: u8,
    },
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
        self.validate_with_field_name(value, "unknown")
    }
    
    pub fn validate_with_field_name(&self, value: &DataValue, field_name: &str) -> QuickDbResult<()> {
        // 检查必填字段
        if self.required && matches!(value, DataValue::Null) {
            return Err(QuickDbError::ValidationError { field: field_name.to_string(), message: "必填字段不能为空".to_string() });
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
                match value {
                    DataValue::Array(arr) => {
                        // 处理DataValue::Array格式
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
                    },
                    DataValue::String(json_str) => {
                        // 处理JSON字符串格式的数组
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(arr) = json_value.as_array() {
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
                                for item_json in arr {
                                    let item_data_value = DataValue::from_json(item_json.clone());
                                    item_field.validate(&item_data_value)?;
                                }
                            } else {
                                return Err(QuickDbError::ValidationError {
                                    field: "type_mismatch".to_string(),
                                    message: "JSON字符串不是有效的数组格式".to_string()
                                });
                            }
                        } else {
                            return Err(QuickDbError::ValidationError {
                                field: "type_mismatch".to_string(),
                                message: "无法解析JSON字符串".to_string()
                            });
                        }
                    },
                    _ => {
                        return Err(QuickDbError::ValidationError {
                            field: "type_mismatch".to_string(),
                            message: "字段类型不匹配，期望数组类型或JSON字符串".to_string()
                        });
                    }
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
            FieldType::BigInteger => {
                if !matches!(value, DataValue::Int(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望大整数类型".to_string()
                    });
                }
            }
            FieldType::Double => {
                if !matches!(value, DataValue::Float(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望双精度浮点数类型".to_string()
                    });
                }
            }
            FieldType::Text => {
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望文本类型".to_string()
                    });
                }
            }
            FieldType::Date => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望日期类型".to_string()
                    });
                }
            }
            FieldType::Time => {
                if !matches!(value, DataValue::DateTime(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望时间类型".to_string()
                    });
                }
            }
            FieldType::Binary => {
                if !matches!(value, DataValue::String(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望二进制数据（Base64字符串）".to_string()
                    });
                }
            }
            FieldType::Decimal { precision: _, scale: _ } => {
                if !matches!(value, DataValue::Float(_)) {
                    return Err(QuickDbError::ValidationError {
                        field: "type_mismatch".to_string(),
                        message: "字段类型不匹配，期望十进制数类型".to_string()
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
        
        // 调试信息：打印序列化后的数据
        info!("🔍 验证数据映射: {:?}", data);
        
        for (field_name, field_def) in &meta.fields {
            let field_value = data.get(field_name).unwrap_or(&DataValue::Null);
            info!("🔍 验证字段 {}: {:?}", field_name, field_value);
            field_def.validate_with_field_name(field_value, field_name)?;
        }
        
        Ok(())
    }
    
    /// 转换为数据映射（直接转换，避免 JSON 序列化开销）
    /// 子类应该重写此方法以提供高性能的直接转换
    fn to_data_map_direct(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        // 默认回退到 JSON 序列化方式，但建议子类重写
        warn!("使用默认的 JSON 序列化方式，建议重写 to_data_map_direct 方法以提高性能");
        self.to_data_map_legacy()
    }
    
    /// 转换为数据映射（传统 JSON 序列化方式）
    /// 保留此方法用于向后兼容和调试
    fn to_data_map_legacy(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        let json_str = serde_json::to_string(self)
            .map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?;
        info!("🔍 序列化后的JSON字符串: {}", json_str);
        
        let json_value: JsonValue = serde_json::from_str(&json_str)
            .map_err(|e| QuickDbError::SerializationError { message: format!("解析JSON失败: {}", e) })?;
        info!("🔍 解析后的JsonValue: {:?}", json_value);
        
        let mut data_map = HashMap::new();
        if let JsonValue::Object(obj) = json_value {
            for (key, value) in obj {
                let data_value = DataValue::from_json(value.clone());
                info!("🔍 字段 {} 转换: {:?} -> {:?}", key, value, data_value);
                data_map.insert(key, data_value);
            }
        }
        
        Ok(data_map)
    }
    
    /// 将模型转换为数据映射（高性能版本）
    fn to_data_map(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        self.to_data_map_direct()
    }

    /// 从数据映射创建模型实例
    fn from_data_map(data: HashMap<String, DataValue>) -> QuickDbResult<Self> {
        // 直接将 HashMap<String, DataValue> 转换为 JsonValue，避免带类型标签的序列化
        let mut json_map = serde_json::Map::new();
        for (key, value) in data {
            let json_value = match &value {
                // 对于字符串类型的DataValue，检查是否是JSON格式
                DataValue::String(s) => {
                    // 尝试解析为JSON，如果失败则作为普通字符串处理
                    if (s.starts_with('[') && s.ends_with(']')) || (s.starts_with('{') && s.ends_with('}')) {
                        match serde_json::from_str::<serde_json::Value>(s) {
                            Ok(parsed) => parsed,
                            Err(_) => value.to_json_value(),
                        }
                    } else {
                        value.to_json_value()
                    }
                },
                _ => value.to_json_value(),
            };
            json_map.insert(key, json_value);
        }
        let json_value = JsonValue::Object(json_map);
        
        debug!("准备反序列化的JSON数据: {}", serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "无法序列化".to_string()));
        serde_json::from_value(json_value).map_err(|e| {
            error!("反序列化失败，错误详情: {}", e);
            QuickDbError::SerializationError { message: format!("反序列化失败: {}", e) }
        })
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
        
        if let Some(data_value) = result {
            // 处理 DataValue::Object 格式的数据
            match data_value {
                DataValue::Object(data_map) => {
                    let model: T = T::from_data_map(data_map)?;
                    Ok(Some(model))
                },
                _ => {
                    // 兼容其他格式，使用直接反序列化
                    let model: T = data_value.deserialize_to()?;
                    Ok(Some(model))
                }
            }
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
        
        // result 已经是 Vec<DataValue>，直接处理
        let mut models = Vec::new();
        for data_value in result {
            // 处理 DataValue::Object 格式的数据
            match data_value {
                DataValue::Object(data_map) => {
                    let model: T = T::from_data_map(data_map)?;
                    models.push(model);
                },
                _ => {
                    // 兼容其他格式，使用直接反序列化
                    let model: T = data_value.deserialize_to()?;
                    models.push(model);
                }
            }
        }
        Ok(models)
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

/// 便捷函数：创建数组字段
/// 在 MongoDB 中使用原生数组，在 SQL 数据库中使用 JSON 存储
pub fn array_field(
    item_type: FieldType,
    max_items: Option<usize>,
    min_items: Option<usize>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::Array {
        item_type: Box::new(item_type),
        max_items,
        min_items,
    })
}

/// 便捷函数：创建列表字段（array_field 的别名）
/// 在 MongoDB 中使用原生数组，在 SQL 数据库中使用 JSON 存储
pub fn list_field(
    item_type: FieldType,
    max_items: Option<usize>,
    min_items: Option<usize>,
) -> FieldDefinition {
    // list_field 是 array_field 的别名，提供更直观的命名
    array_field(item_type, max_items, min_items)
}

/// 便捷函数：创建字符串字段
pub fn string_field(
    max_length: Option<usize>,
    min_length: Option<usize>,
    regex: Option<String>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::String {
        max_length,
        min_length,
        regex,
    })
}

/// 便捷函数：创建整数字段
pub fn integer_field(
    min_value: Option<i64>,
    max_value: Option<i64>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::Integer {
        min_value,
        max_value,
    })
}

/// 便捷函数：创建浮点数字段
pub fn float_field(
    min_value: Option<f64>,
    max_value: Option<f64>,
) -> FieldDefinition {
    FieldDefinition::new(FieldType::Float {
        min_value,
        max_value,
    })
}

/// 便捷函数：创建布尔字段
pub fn boolean_field() -> FieldDefinition {
    FieldDefinition::new(FieldType::Boolean)
}

/// 便捷函数：创建日期时间字段
pub fn datetime_field() -> FieldDefinition {
    FieldDefinition::new(FieldType::DateTime)
}

/// 便捷函数：创建UUID字段
pub fn uuid_field() -> FieldDefinition {
    FieldDefinition::new(FieldType::Uuid)
}

/// 便捷函数：创建JSON字段
pub fn json_field() -> FieldDefinition {
    FieldDefinition::new(FieldType::Json)
}

/// 便捷函数：创建字典字段（基于Object类型）
pub fn dict_field(fields: HashMap<String, FieldDefinition>) -> FieldDefinition {
    FieldDefinition::new(FieldType::Object { fields })
}

/// 便捷函数：创建引用字段
pub fn reference_field(target_collection: String) -> FieldDefinition {
    FieldDefinition::new(FieldType::Reference {
        target_collection,
    })
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

                let model_meta = $crate::model::ModelMeta {
                    collection_name: $collection.to_string(),
                    database_alias: None $(.or(Some($database.to_string())))?,
                    fields,
                    indexes,
                    description: None,
                };

                // 自动注册模型元数据（仅在首次调用时注册）
                static ONCE: std::sync::Once = std::sync::Once::new();
                ONCE.call_once(|| {
                    if let Err(e) = $crate::manager::register_model(model_meta.clone()) {
                        eprintln!("⚠️  模型注册失败: {}", e);
                    } else {
                        println!("✅ 模型自动注册成功: {}", model_meta.collection_name);
                    }
                });

                model_meta
            }
            
            /// 高性能直接转换实现，避免 JSON 序列化开销
            fn to_data_map_direct(&self) -> $crate::error::QuickDbResult<std::collections::HashMap<String, $crate::types::DataValue>> {
                use $crate::model::ToDataValue;
                let mut data_map = std::collections::HashMap::new();

                $(
                    data_map.insert(stringify!($field).to_string(), self.$field.to_data_value());
                )*

                // 移除为None的_id字段，让MongoDB自动生成
                if let Some(id_value) = data_map.get("_id") {
                    if matches!(id_value, $crate::types::DataValue::Null) {
                        data_map.remove("_id");
                    }
                }

                Ok(data_map)
            }
        }
        
        impl $name {
            /// 保存模型到数据库
            pub async fn save(&self) -> $crate::error::QuickDbResult<String> {
                self.validate()?;
                let data = self.to_data_map()?;
                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();
                
                let result = $crate::odm::create(
                    &collection_name,
                    data,
                    database_alias.as_deref(),
                ).await?;
                
                // 将 DataValue 转换为 String（通常是 ID）
                match result {
                    $crate::types::DataValue::String(id) => Ok(id),
                    $crate::types::DataValue::Int(id) => Ok(id.to_string()),
                    $crate::types::DataValue::Uuid(id) => Ok(id.to_string()),
                    $crate::types::DataValue::Object(obj) => {
                        // 如果返回的是对象，尝试提取_id字段（MongoDB）或id字段（SQL）
                        if let Some(id_value) = obj.get("_id").or_else(|| obj.get("id")) {
                            match id_value {
                                $crate::types::DataValue::String(id) => Ok(id.clone()),
                                $crate::types::DataValue::Int(id) => Ok(id.to_string()),
                                $crate::types::DataValue::Uuid(id) => Ok(id.to_string()),
                                _ => Ok(format!("{:?}", id_value))
                            }
                        } else {
                            // 如果对象中没有id字段，序列化整个对象
                            match serde_json::to_string(&obj) {
                                Ok(json_str) => Ok(json_str),
                                Err(_) => Ok(format!("{:?}", obj))
                            }
                        }
                    },
                    other => {
                        // 如果返回的不是简单的 ID 类型，尝试序列化为 JSON
                        match serde_json::to_string(&other) {
                            Ok(json_str) => Ok(json_str),
                            Err(_) => Ok(format!("{:?}", other))
                        }
                    }
                }
            }
            
            /// 更新模型
            pub async fn update(&self, updates: std::collections::HashMap<String, $crate::types::DataValue>) -> $crate::error::QuickDbResult<bool> {
                // 尝试从模型中获取ID字段，兼容 MongoDB 的 _id 和 SQL 的 id
                let data_map = self.to_data_map()?;
                let (id_field_name, id_value) = data_map.get("_id")
                    .map(|v| ("_id", v))
                    .or_else(|| data_map.get("id").map(|v| ("id", v)))
                    .ok_or_else(|| $crate::error::QuickDbError::ValidationError {
                        field: "id".to_string(),
                        message: "模型缺少ID字段（id 或 _id），无法更新".to_string()
                    })?;

                // 将ID转换为字符串
                let id_str = match id_value {
                    $crate::types::DataValue::String(s) => s.clone(),
                    $crate::types::DataValue::Int(i) => i.to_string(),
                    $crate::types::DataValue::Uuid(u) => u.to_string(),
                    // MongoDB 的 ObjectId 可能存储在 Object 中
                    $crate::types::DataValue::Object(obj) => {
                        if let Some($crate::types::DataValue::String(oid)) = obj.get("$oid") {
                            oid.clone()
                        } else {
                            return Err($crate::error::QuickDbError::ValidationError {
                                field: id_field_name.to_string(),
                                message: format!("不支持的MongoDB ObjectId格式: {:?}", obj)
                            });
                        }
                    }
                    _ => return Err($crate::error::QuickDbError::ValidationError {
                        field: id_field_name.to_string(),
                        message: format!("不支持的ID类型: {:?}", id_value)
                    })
                };

                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                $crate::odm::update_by_id(&collection_name, &id_str, updates, database_alias.as_deref()).await
            }
            
            /// 删除模型
            pub async fn delete(&self) -> $crate::error::QuickDbResult<bool> {
                // 尝试从模型中获取ID字段，兼容 MongoDB 的 _id 和 SQL 的 id
                let data_map = self.to_data_map()?;
                let (id_field_name, id_value) = data_map.get("_id")
                    .map(|v| ("_id", v))
                    .or_else(|| data_map.get("id").map(|v| ("id", v)))
                    .ok_or_else(|| $crate::error::QuickDbError::ValidationError {
                        field: "id".to_string(),
                        message: "模型缺少ID字段（id 或 _id），无法删除".to_string()
                    })?;

                // 将ID转换为字符串
                let id_str = match id_value {
                    $crate::types::DataValue::String(s) => s.clone(),
                    $crate::types::DataValue::Int(i) => i.to_string(),
                    $crate::types::DataValue::Uuid(u) => u.to_string(),
                    // MongoDB 的 ObjectId 可能存储在 Object 中
                    $crate::types::DataValue::Object(obj) => {
                        if let Some($crate::types::DataValue::String(oid)) = obj.get("$oid") {
                            oid.clone()
                        } else {
                            return Err($crate::error::QuickDbError::ValidationError {
                                field: id_field_name.to_string(),
                                message: format!("不支持的MongoDB ObjectId格式: {:?}", obj)
                            });
                        }
                    }
                    _ => return Err($crate::error::QuickDbError::ValidationError {
                        field: id_field_name.to_string(),
                        message: format!("不支持的ID类型: {:?}", id_value)
                    })
                };

                let collection_name = Self::collection_name();
                let database_alias = Self::database_alias();

                $crate::odm::delete_by_id(&collection_name, &id_str, database_alias.as_deref()).await
            }
        }
    };
}