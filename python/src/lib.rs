use pyo3::prelude::*;
use rat_logger::{LoggerBuilder, LevelFilter, info, error, warn, debug, trace};

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

/// 初始化日志系统（使用默认级别）
///
/// 注意：此函数为可选，调用者也可以自行初始化日志系统
#[pyfunction]
fn init_logging() -> PyResult<()> {
    LoggerBuilder::new()
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("日志初始化失败");
    Ok(())
}

/// 初始化日志系统并设置级别
///
/// 参数:
/// - level: 日志级别，可选值为 "trace", "debug", "info", "warn", "error"
///
/// 注意：此函数为可选，调用者也可以自行初始化日志系统
#[pyfunction]
fn init_logging_with_level(level: &str) -> PyResult<()> {
    let level_filter = match level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid log level: {}. Must be one of: trace, debug, info, warn, error", level)
        )),
    };
    LoggerBuilder::new()
        .with_level(level_filter)
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .expect("日志初始化失败");
    Ok(())
}

/// 日志宏函数 - Info级别
#[pyfunction]
fn log_info(message: &str) {
    info!("{}", message);
}

/// 日志宏函数 - Error级别
#[pyfunction]
fn log_error(message: &str) {
    error!("{}", message);
}

/// 日志宏函数 - Warn级别
#[pyfunction]
fn log_warn(message: &str) {
    warn!("{}", message);
}

/// 日志宏函数 - Debug级别
#[pyfunction]
fn log_debug(message: &str) {
    debug!("{}", message);
}

/// 日志宏函数 - Trace级别
#[pyfunction]
fn log_trace(message: &str) {
    trace!("{}", message);
}

/// Python模块定义
#[pymodule]
fn rat_quickdb_py(_py: Python, m: &PyModule) -> PyResult<()> {
    // 基础函数
    m.add_function(wrap_pyfunction!(get_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_info, m)?)?;
    m.add_function(wrap_pyfunction!(get_name, m)?)?;
    
    // 日志初始化函数
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    m.add_function(wrap_pyfunction!(init_logging_with_level, m)?)?;
    
    // 日志宏函数
    m.add_function(wrap_pyfunction!(log_info, m)?)?;
    m.add_function(wrap_pyfunction!(log_error, m)?)?;
    m.add_function(wrap_pyfunction!(log_warn, m)?)?;
    m.add_function(wrap_pyfunction!(log_debug, m)?)?;
    m.add_function(wrap_pyfunction!(log_trace, m)?)?;


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
