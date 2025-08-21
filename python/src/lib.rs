use pyo3::prelude::*;

// 模块声明
mod config;
mod message;
mod bridge;
mod async_handler;
mod parser;
mod model_bindings;

// 导入模块内容
use config::*;
use message::*;
use bridge::*;
use model_bindings::*;

/// 获取版本信息
#[pyfunction]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 获取包信息
#[pyfunction]
fn get_info() -> String {
    "Rat QuickDB Python Binding".to_string()
}

/// 获取包名称
#[pyfunction]
fn get_name() -> String {
    env!("CARGO_PKG_NAME").to_string()
}



/// Python模块定义
#[pymodule]
fn rat_quickdb_py(_py: Python, m: &PyModule) -> PyResult<()> {
    // 基础函数
    m.add_function(wrap_pyfunction!(get_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_info, m)?)?;
    m.add_function(wrap_pyfunction!(get_name, m)?)?;


    // 数据库桥接器
    m.add_class::<PyDbQueueBridge>()?;
    m.add_function(wrap_pyfunction!(create_db_queue_bridge, m)?)?;

    // 配置类
    m.add_class::<PyCacheConfig>()?;
    m.add_class::<PyL1CacheConfig>()?;
    m.add_class::<PyL2CacheConfig>()?;
    m.add_class::<PyTtlConfig>()?;
    m.add_class::<PyCompressionConfig>()?;
    m.add_class::<PyTlsConfig>()?;
    m.add_class::<PyZstdConfig>()?;

    // ODM模型系统类
    m.add_class::<PyFieldType>()?;
    m.add_class::<PyFieldDefinition>()?;
    m.add_class::<PyIndexDefinition>()?;
    m.add_class::<PyModelMeta>()?;

    // 便捷字段创建函数
    m.add_function(wrap_pyfunction!(string_field, m)?)?;
    m.add_function(wrap_pyfunction!(integer_field, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_field, m)?)?;
    m.add_function(wrap_pyfunction!(datetime_field, m)?)?;
    m.add_function(wrap_pyfunction!(uuid_field, m)?)?;
    m.add_function(wrap_pyfunction!(reference_field, m)?)?;
    m.add_function(wrap_pyfunction!(array_field, m)?)?;
    m.add_function(wrap_pyfunction!(json_field, m)?)?;
    m.add_function(wrap_pyfunction!(list_field, m)?)?;
    m.add_function(wrap_pyfunction!(float_field, m)?)?;
    m.add_function(wrap_pyfunction!(dict_field, m)?)?;

    Ok(())
}
