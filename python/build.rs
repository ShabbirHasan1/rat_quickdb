use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 检查是否在根目录下运行maturin（错误的做法）
    let current_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let dir_name = current_dir.file_name().unwrap_or_default();

    // 如果当前目录是rat_quickdb，说明在根目录下运行（错误）
    if dir_name == "rat_quickdb" {
        panic!("
🚨 错误：不能在 rat_quickdb 根目录下运行 maturin develop！

📁 正确的编译目录：python/
✅ 正确的命令：cd python && maturin develop

❌ 错误的编译位置：rat_quickdb/
❌ 这会导致生成错误的包名和配置

RAT QuickDB Python 绑定位于 python/ 子目录中。
        ");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // 获取包名和版本
    let package_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "rat_quickdb_py".to_string());
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.2.0".to_string());
    
    // 创建Python包目录 (将下划线转换为包名)
    let python_package_dir = Path::new("rat_quickdb_py");
    
    if !python_package_dir.exists() {
        fs::create_dir_all(python_package_dir).expect("Failed to create Python package directory");
    }
    
    // 删除旧的__init__.py文件（如果存在）
    let init_py_path = python_package_dir.join("__init__.py");
    if init_py_path.exists() {
        fs::remove_file(&init_py_path).expect("Failed to remove old __init__.py");
    }

    // 删除旧的.so文件（如果存在）- 防止缓存问题
    if let Ok(entries) = fs::read_dir(python_package_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.contains("rat_quickdb_py") &&
                   (file_name.ends_with(".so") || file_name.ends_with(".dylib") || file_name.ends_with(".pyd")) {
                    println!("Removing old shared library: {:?}", path);
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
    
    // 生成 __init__.py 文件
    let init_py_content = format!(
r#"""
"""
{} - RAT QuickDB Python Bindings

跨数据库ODM库的Python绑定，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

Version: {}
"""

__version__ = "{}"

# 从Rust编译的模块中导入主要类
# 这些类由maturin在构建时自动注册
try:
    from .rat_quickdb_py import (
        # 基础函数
        DbQueueBridge, create_db_queue_bridge,
        init_logging, init_logging_with_level, init_logging_advanced,
        is_logging_initialized,
        log_info, log_error, log_warn, log_debug, log_trace,
        get_version, get_name, get_info,

        # 配置类
        PyCacheConfig, PyL1CacheConfig, PyL2CacheConfig, PyTtlConfig,
        PyCompressionConfig, PyTlsConfig, PyZstdConfig,

        # ODM模型系统类
        FieldType, FieldDefinition, IndexDefinition, ModelMeta,

        # 字段创建函数
        string_field, integer_field, boolean_field, datetime_field,
        uuid_field, reference_field, array_field, json_field,
        list_field, float_field, dict_field,

        # 模型管理函数
        register_model
    )
    __all__ = [
        # 基础函数
        "DbQueueBridge", "create_db_queue_bridge",
        "init_logging", "init_logging_with_level", "init_logging_advanced",
        "is_logging_initialized",
        "log_info", "log_error", "log_warn", "log_debug", "log_trace",
        "get_version", "get_name", "get_info",

        # 配置类
        "PyCacheConfig", "PyL1CacheConfig", "PyL2CacheConfig", "PyTtlConfig",
        "PyCompressionConfig", "PyTlsConfig", "PyZstdConfig",

        # ODM模型系统类
        "FieldType", "FieldDefinition", "IndexDefinition", "ModelMeta",

        # 字段创建函数
        "string_field", "integer_field", "boolean_field", "datetime_field",
        "uuid_field", "reference_field", "array_field", "json_field",
        "list_field", "float_field", "dict_field",

        # 模型管理函数
        "register_model"
    ]
except ImportError:
    # 如果Rust模块不可用（例如在开发环境中），提供友好的错误信息
    __all__ = []
    import warnings
    warnings.warn(
        "RAT QuickDB Rust扩展未正确加载。请确保已运行 'maturin develop' 或 'pip install -e .'",
        ImportWarning
    )

# 便捷的别名 (仅在成功导入时定义)
try:
    DatabaseBridge = PyDbQueueBridge
except NameError:
    DatabaseBridge = None
"#,
        package_name, version, version
    );
    
    fs::write(&init_py_path, init_py_content)
        .expect("Failed to write __init__.py");
  
    println!("Generated Python package structure for {}", package_name);
    println!("Package version: {}", version);
}