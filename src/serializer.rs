//! JSON序列化层
//! 
//! 提供灵活的序列化选项，支持返回JSON字符串或对象
//! 兼容PyO3调用，可根据调用者需求选择返回格式

use crate::error::{QuickDbError, QuickDbResult};
use crate::types::DataValue;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, Map as JsonMap};
use std::collections::HashMap;
use zerg_creep::{debug, error, info, warn};

/// 序列化输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON字符串格式（默认，兼容PyO3）
    JsonString,
    /// JSON对象格式（Rust原生调用）
    JsonObject,
    /// 原始数据格式（内部使用）
    RawData,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::JsonString
    }
}

/// 序列化配置
#[derive(Debug, Clone)]
pub struct SerializerConfig {
    /// 输出格式
    pub format: OutputFormat,
    /// 是否美化输出（仅对JSON字符串有效）
    pub pretty: bool,
    /// 是否包含空值字段
    pub include_null: bool,
    /// 日期时间格式
    pub datetime_format: Option<String>,
    /// 数字精度（浮点数小数位数）
    pub float_precision: Option<usize>,
}

impl Default for SerializerConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::JsonString,
            pretty: false,
            include_null: true,
            datetime_format: None,
            float_precision: None,
        }
    }
}

impl SerializerConfig {
    /// 创建新的序列化配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置输出格式
    pub fn format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    /// 设置美化输出
    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// 设置是否包含空值
    pub fn include_null(mut self, include_null: bool) -> Self {
        self.include_null = include_null;
        self
    }

    /// 设置日期时间格式
    pub fn datetime_format(mut self, format: &str) -> Self {
        self.datetime_format = Some(format.to_string());
        self
    }

    /// 设置浮点数精度
    pub fn float_precision(mut self, precision: usize) -> Self {
        self.float_precision = Some(precision);
        self
    }

    /// 创建PyO3兼容配置
    pub fn for_pyo3() -> Self {
        Self {
            format: OutputFormat::JsonString,
            pretty: false,
            include_null: true,
            datetime_format: Some("%Y-%m-%dT%H:%M:%S%.3fZ".to_string()),
            float_precision: Some(6),
        }
    }

    /// 创建Rust原生配置
    pub fn for_rust() -> Self {
        Self {
            format: OutputFormat::JsonObject,
            pretty: false,
            include_null: false,
            datetime_format: None,
            float_precision: None,
        }
    }

    /// 创建调试配置
    pub fn for_debug() -> Self {
        Self {
            format: OutputFormat::JsonString,
            pretty: true,
            include_null: true,
            datetime_format: Some("%Y-%m-%d %H:%M:%S".to_string()),
            float_precision: Some(2),
        }
    }
}

/// 序列化结果
#[derive(Debug, Clone)]
pub enum SerializationResult {
    /// JSON字符串
    JsonString(String),
    /// JSON对象
    JsonObject(JsonValue),
    /// 原始数据
    RawData(HashMap<String, DataValue>),
}

impl SerializationResult {
    /// 转换为JSON字符串
    pub fn to_json_string(&self) -> QuickDbResult<String> {
        match self {
            SerializationResult::JsonString(s) => Ok(s.clone()),
            SerializationResult::JsonObject(obj) => {
                serde_json::to_string(obj)
                    .map_err(|e| QuickDbError::SerializationError { message: format!("序列化为JSON字符串失败: {}", e) })
            }
            SerializationResult::RawData(data) => {
                let json_obj = data_map_to_json_value(data)?;
                serde_json::to_string(&json_obj)
                    .map_err(|e| QuickDbError::SerializationError { message: format!("序列化为JSON字符串失败: {}", e) })
            }
        }
    }

    /// 转换为JSON对象
    pub fn to_json_object(&self) -> QuickDbResult<JsonValue> {
        match self {
            SerializationResult::JsonString(s) => {
                serde_json::from_str(s)
                    .map_err(|e| QuickDbError::SerializationError { message: format!("解析JSON字符串失败: {}", e) })
            }
            SerializationResult::JsonObject(obj) => Ok(obj.clone()),
            SerializationResult::RawData(data) => {
                data_map_to_json_value(data)
            }
        }
    }

    /// 转换为原始数据
    pub fn to_raw_data(&self) -> QuickDbResult<HashMap<String, DataValue>> {
        match self {
            SerializationResult::JsonString(s) => {
                let json_obj: JsonValue = serde_json::from_str(s)
                    .map_err(|e| QuickDbError::SerializationError { message: format!("解析JSON字符串失败: {}", e) })?;
                json_value_to_data_map(&json_obj)
            }
            SerializationResult::JsonObject(obj) => {
                json_value_to_data_map(obj)
            }
            SerializationResult::RawData(data) => Ok(data.clone()),
        }
    }

    /// 获取结果类型
    pub fn result_type(&self) -> &'static str {
        match self {
            SerializationResult::JsonString(_) => "json_string",
            SerializationResult::JsonObject(_) => "json_object",
            SerializationResult::RawData(_) => "raw_data",
        }
    }
}

/// 数据序列化器
pub struct DataSerializer {
    config: SerializerConfig,
}

impl DataSerializer {
    /// 创建新的数据序列化器
    pub fn new(config: SerializerConfig) -> Self {
        debug!("创建数据序列化器: format={:?}", config.format);
        Self { config }
    }

    /// 使用默认配置创建序列化器
    pub fn default() -> Self {
        Self::new(SerializerConfig::default())
    }

    /// 序列化单个数据记录
    pub fn serialize_record(&self, data: HashMap<String, DataValue>) -> QuickDbResult<SerializationResult> {
        debug!("序列化单个记录: {} 个字段", data.len());
        
        let processed_data = self.process_data(data)?;
        
        match self.config.format {
            OutputFormat::JsonString => {
                let json_obj = data_map_to_json_value(&processed_data)?;
                let json_str = if self.config.pretty {
                    serde_json::to_string_pretty(&json_obj)
                } else {
                    serde_json::to_string(&json_obj)
                }.map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?;
                
                Ok(SerializationResult::JsonString(json_str))
            }
            OutputFormat::JsonObject => {
                let json_obj = data_map_to_json_value(&processed_data)?;
                Ok(SerializationResult::JsonObject(json_obj))
            }
            OutputFormat::RawData => {
                Ok(SerializationResult::RawData(processed_data))
            }
        }
    }

    /// 序列化多个数据记录
    pub fn serialize_records(&self, records: Vec<HashMap<String, DataValue>>) -> QuickDbResult<SerializationResult> {
        debug!("序列化多个记录: {} 条记录", records.len());
        
        let mut processed_records = Vec::new();
        for record in records {
            let processed_data = self.process_data(record)?;
            processed_records.push(processed_data);
        }
        
        match self.config.format {
            OutputFormat::JsonString => {
                let mut json_array = Vec::new();
                for record in processed_records {
                    let json_obj = data_map_to_json_value(&record)?;
                    json_array.push(json_obj);
                }
                
                let json_str = if self.config.pretty {
                    serde_json::to_string_pretty(&json_array)
                } else {
                    serde_json::to_string(&json_array)
                }.map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?;
                
                Ok(SerializationResult::JsonString(json_str))
            }
            OutputFormat::JsonObject => {
                let mut json_array = Vec::new();
                for record in processed_records {
                    let json_obj = data_map_to_json_value(&record)?;
                    json_array.push(json_obj);
                }
                Ok(SerializationResult::JsonObject(JsonValue::Array(json_array)))
            }
            OutputFormat::RawData => {
                // 对于原始数据格式，我们返回一个特殊的数据结构
                let mut result_data = HashMap::new();
                result_data.insert("records".to_string(), DataValue::Array(
                    processed_records.into_iter()
                        .map(|record| DataValue::Object(record))
                        .collect()
                ));
                Ok(SerializationResult::RawData(result_data))
            }
        }
    }

    /// 序列化查询结果
    pub fn serialize_query_result(
        &self,
        records: Vec<HashMap<String, DataValue>>,
        total_count: Option<u64>,
        has_more: Option<bool>,
    ) -> QuickDbResult<SerializationResult> {
        debug!("序列化查询结果: {} 条记录", records.len());
        
        let mut result_data = HashMap::new();
        
        // 序列化记录
        let records_result = self.serialize_records(records)?;
        match records_result {
            SerializationResult::JsonObject(JsonValue::Array(arr)) => {
                result_data.insert("data".to_string(), DataValue::Array(
                    arr.into_iter()
                        .map(|v| DataValue::from_json(v))
                        .collect()
                ));
            }
            SerializationResult::RawData(raw) => {
                if let Some(DataValue::Array(records)) = raw.get("records") {
                    result_data.insert("data".to_string(), DataValue::Array(records.clone()));
                }
            }
            _ => {
                return Err(QuickDbError::SerializationError {
                    message: "无法处理记录序列化结果".to_string()
                });
            }
        }
        
        // 添加元数据
        if let Some(count) = total_count {
            result_data.insert("total_count".to_string(), DataValue::Int(count as i64));
        }
        
        if let Some(more) = has_more {
            result_data.insert("has_more".to_string(), DataValue::Bool(more));
        }
        
        result_data.insert("count".to_string(), DataValue::Int(
            if let Some(DataValue::Array(data)) = result_data.get("data") {
                data.len() as i64
            } else {
                0
            }
        ));
        
        // 根据配置格式返回结果
        match self.config.format {
            OutputFormat::JsonString => {
                let json_obj = data_map_to_json_value(&result_data)?;
                let json_str = if self.config.pretty {
                    serde_json::to_string_pretty(&json_obj)
                } else {
                    serde_json::to_string(&json_obj)
                }.map_err(|e| QuickDbError::SerializationError { message: format!("序列化失败: {}", e) })?;
                
                Ok(SerializationResult::JsonString(json_str))
            }
            OutputFormat::JsonObject => {
                let json_obj = data_map_to_json_value(&result_data)?;
                Ok(SerializationResult::JsonObject(json_obj))
            }
            OutputFormat::RawData => {
                Ok(SerializationResult::RawData(result_data))
            }
        }
    }

    /// 处理数据（应用配置选项）
    fn process_data(&self, mut data: HashMap<String, DataValue>) -> QuickDbResult<HashMap<String, DataValue>> {
        // 移除空值字段（如果配置要求）
        if !self.config.include_null {
            data.retain(|_, v| !matches!(v, DataValue::Null));
        }
        
        // 处理日期时间格式
        if let Some(ref format) = self.config.datetime_format {
            for (_, value) in data.iter_mut() {
                if let DataValue::DateTime(dt) = value {
                    // 这里可以根据需要格式化日期时间
                    // 暂时保持原样
                }
            }
        }
        
        // 处理浮点数精度
        if let Some(precision) = self.config.float_precision {
            for (_, value) in data.iter_mut() {
                if let DataValue::Float(f) = value {
                    let multiplier = 10_f64.powi(precision as i32);
                    *f = (*f * multiplier).round() / multiplier;
                }
            }
        }
        
        Ok(data)
    }
}

/// 将DataValue映射转换为JsonValue
fn data_map_to_json_value(data: &HashMap<String, DataValue>) -> QuickDbResult<JsonValue> {
    let mut json_map = JsonMap::new();
    
    for (key, value) in data {
        json_map.insert(key.clone(), value.to_json_value());
    }
    
    Ok(JsonValue::Object(json_map))
}

/// 将JsonValue转换为DataValue映射
fn json_value_to_data_map(json: &JsonValue) -> QuickDbResult<HashMap<String, DataValue>> {
    let mut data_map = HashMap::new();
    
    if let JsonValue::Object(obj) = json {
        for (key, value) in obj {
            data_map.insert(key.clone(), DataValue::from_json_value(value.clone()));
        }
    } else {
        return Err(QuickDbError::SerializationError {
            message: "JSON值不是对象类型".to_string()
        });
    }
    
    Ok(data_map)
}

/// 全局序列化器实例
static DEFAULT_SERIALIZER: once_cell::sync::Lazy<DataSerializer> = 
    once_cell::sync::Lazy::new(|| {
        DataSerializer::default()
    });

static PYO3_SERIALIZER: once_cell::sync::Lazy<DataSerializer> = 
    once_cell::sync::Lazy::new(|| {
        DataSerializer::new(SerializerConfig::for_pyo3())
    });

static RUST_SERIALIZER: once_cell::sync::Lazy<DataSerializer> = 
    once_cell::sync::Lazy::new(|| {
        DataSerializer::new(SerializerConfig::for_rust())
    });

/// 便捷函数：使用默认配置序列化记录
pub fn serialize_record(data: HashMap<String, DataValue>) -> QuickDbResult<String> {
    let result = DEFAULT_SERIALIZER.serialize_record(data)?;
    result.to_json_string()
}

/// 便捷函数：使用默认配置序列化多个记录
pub fn serialize_records(records: Vec<HashMap<String, DataValue>>) -> QuickDbResult<String> {
    let result = DEFAULT_SERIALIZER.serialize_records(records)?;
    result.to_json_string()
}

/// 便捷函数：使用PyO3兼容配置序列化记录
pub fn serialize_record_for_pyo3(data: HashMap<String, DataValue>) -> QuickDbResult<String> {
    let result = PYO3_SERIALIZER.serialize_record(data)?;
    result.to_json_string()
}

/// 便捷函数：使用PyO3兼容配置序列化多个记录
pub fn serialize_records_for_pyo3(records: Vec<HashMap<String, DataValue>>) -> QuickDbResult<String> {
    let result = PYO3_SERIALIZER.serialize_records(records)?;
    result.to_json_string()
}

/// 便捷函数：使用Rust原生配置序列化记录
pub fn serialize_record_for_rust(data: HashMap<String, DataValue>) -> QuickDbResult<JsonValue> {
    let result = RUST_SERIALIZER.serialize_record(data)?;
    result.to_json_object()
}

/// 便捷函数：使用Rust原生配置序列化多个记录
pub fn serialize_records_for_rust(records: Vec<HashMap<String, DataValue>>) -> QuickDbResult<JsonValue> {
    let result = RUST_SERIALIZER.serialize_records(records)?;
    result.to_json_object()
}

/// 便捷函数：序列化查询结果
pub fn serialize_query_result(
    records: Vec<HashMap<String, DataValue>>,
    total_count: Option<u64>,
    has_more: Option<bool>,
) -> QuickDbResult<String> {
    let result = DEFAULT_SERIALIZER.serialize_query_result(records, total_count, has_more)?;
    result.to_json_string()
}

/// 便捷函数：为PyO3序列化查询结果
pub fn serialize_query_result_for_pyo3(
    records: Vec<HashMap<String, DataValue>>,
    total_count: Option<u64>,
    has_more: Option<bool>,
) -> QuickDbResult<String> {
    let result = PYO3_SERIALIZER.serialize_query_result(records, total_count, has_more)?;
    result.to_json_string()
}

/// 便捷函数：为Rust序列化查询结果
pub fn serialize_query_result_for_rust(
    records: Vec<HashMap<String, DataValue>>,
    total_count: Option<u64>,
    has_more: Option<bool>,
) -> QuickDbResult<JsonValue> {
    let result = RUST_SERIALIZER.serialize_query_result(records, total_count, has_more)?;
    result.to_json_object()
}

#[cfg(feature = "python")]
mod pyo3_support {
    use super::*;
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList, PyString};
    
    /// PyO3序列化器
    #[pyclass]
    pub struct PyDataSerializer {
        inner: DataSerializer,
    }
    
    #[pymethods]
    impl PyDataSerializer {
        /// 创建新的PyO3序列化器
        #[new]
        pub fn new() -> Self {
            Self {
                inner: DataSerializer::new(SerializerConfig::for_pyo3()),
            }
        }
        
        /// 序列化Python字典为JSON字符串
        pub fn serialize_dict(&self, py: Python, data: &PyDict) -> PyResult<String> {
            let mut data_map = HashMap::new();
            
            for (key, value) in data.iter() {
                let key_str: String = key.extract()?;
                let data_value = python_value_to_data_value(py, value)?;
                data_map.insert(key_str, data_value);
            }
            
            let result = self.inner.serialize_record(data_map)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            
            result.to_json_string()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        }
        
        /// 序列化Python列表为JSON字符串
        pub fn serialize_list(&self, py: Python, data: &PyList) -> PyResult<String> {
            let mut records = Vec::new();
            
            for item in data.iter() {
                if let Ok(dict) = item.downcast::<PyDict>() {
                    let mut data_map = HashMap::new();
                    
                    for (key, value) in dict.iter() {
                        let key_str: String = key.extract()?;
                        let data_value = python_value_to_data_value(py, value)?;
                        data_map.insert(key_str, data_value);
                    }
                    
                    records.push(data_map);
                }
            }
            
            let result = self.inner.serialize_records(records)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            
            result.to_json_string()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        }
    }
    
    /// 将Python值转换为DataValue
    fn python_value_to_data_value(py: Python, value: &PyAny) -> PyResult<DataValue> {
        if value.is_none() {
            Ok(DataValue::Null)
        } else if let Ok(s) = value.extract::<String>() {
            Ok(DataValue::String(s))
        } else if let Ok(i) = value.extract::<i64>() {
            Ok(DataValue::Int(i))
        } else if let Ok(f) = value.extract::<f64>() {
            Ok(DataValue::Float(f))
        } else if let Ok(b) = value.extract::<bool>() {
            Ok(DataValue::Bool(b))
        } else {
            // 对于其他类型，尝试转换为JSON字符串
            let json_str = value.str()?.to_string();
            Ok(DataValue::String(json_str))
        }
    }
}

#[cfg(feature = "python")]
pub use pyo3_support::*;