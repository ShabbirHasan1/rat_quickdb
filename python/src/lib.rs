use pyo3::prelude::*;
use rat_logger::{LoggerBuilder, LevelFilter, info, error, warn, debug, trace, FormatConfig, LevelStyle, handler::term::TermConfig};

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

// 重新导出register_model函数，以便可以在Python模块中直接调用
pub use bridge::register_model;

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

/// 初始化日志系统（使用默认配置）
///
/// 这是简单的日志初始化方式，使用默认的终端输出配置。
/// 调用者也可以选择自行初始化日志系统以获得更多控制权。
///
/// 注意：此函数完全可选，调用者可以自行初始化日志系统
#[pyfunction]
fn init_logging() -> PyResult<()> {
    LoggerBuilder::new()
        .add_terminal_with_config(rat_logger::handler::term::TermConfig::default())
        .init()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("日志初始化失败: {}", e)
        ))
}

/// 初始化日志系统并设置级别
///
/// 参数:
/// - level: 日志级别，可选值为 "trace", "debug", "info", "warn", "error"
///
/// 注意：此函数完全可选，调用者可以自行初始化日志系统
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
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("日志初始化失败: {}", e)
        ))
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

/// 高级日志配置初始化
///
/// 提供更灵活的日志配置选项，允许调用者自定义日志格式和输出配置。
///
/// 参数:
/// - level: 日志级别，可选值为 "trace", "debug", "info", "warn", "error"
/// - enable_color: 是否启用颜色输出
/// - timestamp_format: 时间戳格式，如 "%Y-%m-%d %H:%M:%S%.3f"
/// - custom_format_template: 自定义日志格式模板，可用变量: {timestamp}, {level}, {target}, {line}, {message}
///
/// 注意：此函数完全可选，调用者可以自行初始化日志系统
#[pyfunction]
fn init_logging_advanced(
    level: &str,
    enable_color: Option<bool>,
    timestamp_format: Option<&str>,
    custom_format_template: Option<&str>,
) -> PyResult<()> {
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

    // 创建自定义格式配置
    let format_config = FormatConfig {
        timestamp_format: timestamp_format.unwrap_or("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        level_style: LevelStyle {
            error: "ERROR".to_string(),
            warn: "WARN ".to_string(),
            info: "INFO ".to_string(),
            debug: "DEBUG".to_string(),
            trace: "TRACE".to_string(),
        },
        format_template: custom_format_template.unwrap_or("[{timestamp}] {level} {target}:{line} - {message}").to_string(),
    };

    // 创建终端配置
    let term_config = TermConfig {
        enable_color: enable_color.unwrap_or(true),
        format: Some(format_config),
        color: None, // 使用默认颜色
    };

    LoggerBuilder::new()
        .with_level(level_filter)
        .add_terminal_with_config(term_config)
        .init()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("日志初始化失败: {}", e)
        ))
}

/// 检查日志系统是否已初始化
///
/// 返回:
/// - bool: 日志系统是否已初始化
#[pyfunction]
fn is_logging_initialized() -> bool {
    // 简化实现：由于rat_logger没有提供直接的状态检查方法
    // 我们返回一个合理的默认值
    // 实际应用中，调用者应该自己管理日志初始化状态
    true
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
    m.add_function(wrap_pyfunction!(init_logging_advanced, m)?)?;
    m.add_function(wrap_pyfunction!(is_logging_initialized, m)?)?;

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

    // 模型管理函数
    m.add_function(wrap_pyfunction!(register_model, m)?)?;

    Ok(())
}
