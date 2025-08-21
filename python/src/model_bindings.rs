//! ODM 模型系统的 Python 绑定
//! 
//! 提供类似 MongoEngine 的模型定义方式

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rat_quickdb::model::{FieldType, FieldDefinition, ModelMeta, IndexDefinition};
use rat_quickdb::types::DataValue;
use std::collections::HashMap;
use zerg_creep::{debug, error, info, warn};

/// Python 字段类型包装器
#[pyclass(name = "FieldType")]
#[derive(Debug, Clone)]
pub struct PyFieldType {
    pub inner: FieldType,
}

#[pymethods]
impl PyFieldType {
    /// 创建字符串字段类型
    #[staticmethod]
    pub fn string(max_length: Option<usize>, min_length: Option<usize>) -> Self {
        Self {
            inner: FieldType::String {
                max_length,
                min_length,
                regex: None,
            },
        }
    }

    /// 创建整数字段类型
    #[staticmethod]
    pub fn integer(min_value: Option<i64>, max_value: Option<i64>) -> Self {
        Self {
            inner: FieldType::Integer {
                min_value,
                max_value,
            },
        }
    }

    /// 创建浮点数字段类型
    #[staticmethod]
    pub fn float(min_value: Option<f64>, max_value: Option<f64>) -> Self {
        Self {
            inner: FieldType::Float {
                min_value,
                max_value,
            },
        }
    }

    /// 创建布尔字段类型
    #[staticmethod]
    pub fn boolean() -> Self {
        Self {
            inner: FieldType::Boolean,
        }
    }

    /// 创建日期时间字段类型
    #[staticmethod]
    pub fn datetime() -> Self {
        Self {
            inner: FieldType::DateTime,
        }
    }

    /// 创建UUID字段类型
    #[staticmethod]
    pub fn uuid() -> Self {
        Self {
            inner: FieldType::Uuid,
        }
    }

    /// 创建JSON字段类型
    #[staticmethod]
    pub fn json() -> Self {
        Self {
            inner: FieldType::Json,
        }
    }

    /// 创建数组字段类型
    #[staticmethod]
    pub fn array(item_type: &PyFieldType, max_items: Option<usize>, min_items: Option<usize>) -> Self {
        Self {
            inner: FieldType::Array {
                item_type: Box::new(item_type.inner.clone()),
                max_items,
                min_items,
            },
        }
    }

    /// 创建对象字段类型
    #[staticmethod]
    pub fn object(fields: &PyDict) -> PyResult<Self> {
        let mut field_map = HashMap::new();
        
        // 转换字段定义
        for (key, value) in fields.iter() {
            let field_name = key.extract::<String>()?;
            let field_def = value.extract::<PyFieldDefinition>()?;
            field_map.insert(field_name, field_def.inner);
        }
        
        Ok(Self {
            inner: FieldType::Object { fields: field_map },
        })
    }

    /// 创建引用字段类型
    #[staticmethod]
    pub fn reference(target_collection: String) -> Self {
        Self {
            inner: FieldType::Reference {
                target_collection,
            },
        }
    }

    /// 获取字段类型的字符串表示
    pub fn __str__(&self) -> String {
        format!("{:?}", self.inner)
    }

    /// 获取字段类型的调试表示
    pub fn __repr__(&self) -> String {
        format!("FieldType({:?})", self.inner)
    }
}

/// Python 字段定义包装器
#[pyclass(name = "FieldDefinition")]
#[derive(Debug, Clone)]
pub struct PyFieldDefinition {
    pub inner: FieldDefinition,
}

#[pymethods]
impl PyFieldDefinition {
    /// 创建新的字段定义
    #[new]
    pub fn new(field_type: &PyFieldType) -> Self {
        Self {
            inner: FieldDefinition::new(field_type.inner.clone()),
        }
    }

    /// 设置字段为必填
    pub fn required(&self) -> Self {
        Self {
            inner: self.inner.clone().required(),
        }
    }

    /// 设置字段为唯一
    pub fn unique(&self) -> Self {
        Self {
            inner: self.inner.clone().unique(),
        }
    }

    /// 设置字段建立索引
    pub fn indexed(&self) -> Self {
        Self {
            inner: self.inner.clone().indexed(),
        }
    }

    /// 设置描述
    pub fn set_description(&self, desc: String) -> Self {
        Self {
            inner: self.inner.clone().description(&desc),
        }
    }

    /// 设置默认值
    pub fn default_value(&self, value: &PyAny) -> PyResult<Self> {
        // 这里需要将 Python 值转换为 DataValue
        // 简化实现，暂时只支持基本类型
        let data_value = if let Ok(s) = value.extract::<String>() {
            DataValue::String(s)
        } else if let Ok(i) = value.extract::<i64>() {
            DataValue::Int(i)
        } else if let Ok(f) = value.extract::<f64>() {
            DataValue::Float(f)
        } else if let Ok(b) = value.extract::<bool>() {
            DataValue::Bool(b)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "不支持的默认值类型",
            ));
        };
        
        Ok(Self {
            inner: self.inner.clone().default_value(data_value),
        })
    }

    /// 检查字段是否必填
    #[getter]
    pub fn is_required(&self) -> bool {
        self.inner.required
    }

    /// 检查字段是否唯一
    #[getter]
    pub fn is_unique(&self) -> bool {
        self.inner.unique
    }

    /// 检查字段是否建立索引
    #[getter]
    pub fn is_indexed(&self) -> bool {
        self.inner.indexed
    }

    /// 获取字段描述
    #[getter]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// 获取字段定义的字符串表示
    pub fn __str__(&self) -> String {
        format!("{:?}", self.inner)
    }

    /// 获取字段定义的调试表示
    pub fn __repr__(&self) -> String {
        format!("FieldDefinition({:?})", self.inner)
    }
}

/// Python 索引定义包装器
#[pyclass(name = "IndexDefinition")]
#[derive(Debug, Clone)]
pub struct PyIndexDefinition {
    pub inner: IndexDefinition,
}

#[pymethods]
impl PyIndexDefinition {
    /// 创建新的索引定义
    #[new]
    pub fn new(fields: Vec<String>, unique: bool, name: Option<String>) -> Self {
        Self {
            inner: IndexDefinition {
                fields,
                unique,
                name,
            },
        }
    }

    /// 获取索引字段
    #[getter]
    pub fn fields(&self) -> Vec<String> {
        self.inner.fields.clone()
    }

    /// 检查索引是否唯一
    #[getter]
    pub fn unique(&self) -> bool {
        self.inner.unique
    }

    /// 获取索引名称
    #[getter]
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// 获取索引定义的字符串表示
    pub fn __str__(&self) -> String {
        format!("{:?}", self.inner)
    }

    /// 获取索引定义的调试表示
    pub fn __repr__(&self) -> String {
        format!("IndexDefinition({:?})", self.inner)
    }
}

/// Python 模型元数据包装器
#[pyclass(name = "ModelMeta")]
#[derive(Debug, Clone)]
pub struct PyModelMeta {
    pub inner: ModelMeta,
}

#[pymethods]
impl PyModelMeta {
    /// 创建新的模型元数据
    #[new]
    pub fn new(
        collection_name: String,
        fields: &PyDict,
        indexes: Vec<PyIndexDefinition>,
        database_alias: Option<String>,
        description: Option<String>,
    ) -> PyResult<Self> {
        let mut field_map = HashMap::new();
        
        // 转换字段定义
        for (key, value) in fields.iter() {
            let field_name = key.extract::<String>()?;
            let field_def = value.extract::<PyFieldDefinition>()?;
            field_map.insert(field_name, field_def.inner);
        }
        
        // 转换索引定义
        let index_vec = indexes.into_iter().map(|idx| idx.inner).collect();
        
        Ok(Self {
            inner: ModelMeta {
                collection_name,
                database_alias,
                fields: field_map,
                indexes: index_vec,
                description,
            },
        })
    }

    /// 获取集合名称
    #[getter]
    pub fn collection_name(&self) -> String {
        self.inner.collection_name.clone()
    }

    /// 获取数据库别名
    #[getter]
    pub fn database_alias(&self) -> Option<String> {
        self.inner.database_alias.clone()
    }

    /// 获取字段定义
    #[getter]
    pub fn fields(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            for (name, field_def) in &self.inner.fields {
                let py_field_def = PyFieldDefinition {
                    inner: field_def.clone(),
                };
                let py_obj = Py::new(py, py_field_def)?;
                dict.set_item(name, py_obj)?;
            }
            Ok(dict.into())
        })
    }

    /// 获取索引定义
    #[getter]
    pub fn indexes(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let list = PyList::empty(py);
            for idx in &self.inner.indexes {
                let py_index_def = PyIndexDefinition {
                    inner: idx.clone(),
                };
                let py_obj = Py::new(py, py_index_def)?;
                list.append(py_obj)?;
            }
            Ok(list.into())
        })
    }

    /// 获取模型描述
    #[getter]
    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    /// 获取模型元数据的字符串表示
    pub fn __str__(&self) -> String {
        format!(
            "ModelMeta(collection={}, fields={}, indexes={})",
            self.inner.collection_name,
            self.inner.fields.len(),
            self.inner.indexes.len()
        )
    }

    /// 获取模型元数据的调试表示
    pub fn __repr__(&self) -> String {
        format!("ModelMeta({:?})", self.inner)
    }
}

/// 便捷字段创建函数

/// 创建字符串字段
#[pyfunction]
pub fn string_field(
    required: Option<bool>,
    unique: Option<bool>,
    max_length: Option<usize>,
    min_length: Option<usize>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::string(max_length, min_length);
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if unique.unwrap_or(false) {
        field_def = field_def.unique();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建整数字段
#[pyfunction]
pub fn integer_field(
    required: Option<bool>,
    unique: Option<bool>,
    min_value: Option<i64>,
    max_value: Option<i64>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::integer(min_value, max_value);
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if unique.unwrap_or(false) {
        field_def = field_def.unique();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建布尔字段
#[pyfunction]
pub fn boolean_field(
    required: Option<bool>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::boolean();
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建日期时间字段
#[pyfunction]
pub fn datetime_field(
    required: Option<bool>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::datetime();
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建UUID字段
#[pyfunction]
pub fn uuid_field(
    required: Option<bool>,
    unique: Option<bool>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::uuid();
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if unique.unwrap_or(false) {
        field_def = field_def.unique();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建引用字段
#[pyfunction]
pub fn reference_field(
    target_collection: String,
    required: Option<bool>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::reference(target_collection);
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建数组字段
#[pyfunction]
pub fn array_field(
    item_type: &PyFieldType,
    required: Option<bool>,
    max_items: Option<usize>,
    min_items: Option<usize>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::array(item_type, max_items, min_items);
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建JSON字段
#[pyfunction]
pub fn json_field(
    required: Option<bool>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::json();
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建列表字段（array_field 的别名）
/// 在 MongoDB 中使用原生数组，在 SQL 数据库中使用 JSON 存储
#[pyfunction]
pub fn list_field(
    item_type: &PyFieldType,
    required: Option<bool>,
    max_items: Option<usize>,
    min_items: Option<usize>,
    description: Option<String>,
) -> PyFieldDefinition {
    // list_field 是 array_field 的别名，提供更直观的命名
    array_field(item_type, required, max_items, min_items, description)
}

/// 创建字典字段（基于Object类型）
#[pyfunction]
pub fn float_field(
    required: Option<bool>,
    unique: Option<bool>,
    min_value: Option<f64>,
    max_value: Option<f64>,
    description: Option<String>,
) -> PyFieldDefinition {
    let field_type = PyFieldType::float(min_value, max_value);
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if unique.unwrap_or(false) {
        field_def = field_def.unique();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    field_def
}

/// 创建字典字段定义
#[pyfunction]
pub fn dict_field(
    fields: &PyDict,
    required: Option<bool>,
    description: Option<String>,
) -> PyResult<PyFieldDefinition> {
    let field_type = PyFieldType::object(fields)?;
    let mut field_def = PyFieldDefinition::new(&field_type);
    
    if required.unwrap_or(false) {
        field_def = field_def.required();
    }
    if let Some(desc) = description {
        field_def = field_def.set_description(desc);
    }
    
    Ok(field_def)
}