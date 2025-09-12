use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // 获取包名和版本
    let package_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "rat_quickdb_py".to_string());
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.3".to_string());
    
    // 创建Python包目录 (将下划线转换为包名)
    let python_package_dir = Path::new("rat_quickdb_py");
    
    if !python_package_dir.exists() {
        fs::create_dir_all(python_package_dir).expect("Failed to create Python package directory");
    }
    
    // 生成 __init__.py 文件
    let init_py_path = python_package_dir.join("__init__.py");
    let init_py_content = format!(
r#"""
"""
{} - RAT QuickDB Python Bindings

跨数据库ORM库的Python绑定，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

Version: {}
"""

__version__ = "{}"

# 从Rust编译的模块中导入主要类
# 这些类由maturin在构建时自动注册
try:
    from .rust_bridge import PyDbQueueBridge, create_db_queue_bridge
    __all__ = ["PyDbQueueBridge", "create_db_queue_bridge"]
except ImportError:
    # 如果Rust模块不可用（例如在开发环境中），提供友好的错误信息
    __all__ = []
    import warnings
    warnings.warn(
        "RAT QuickDB Rust扩展未正确加载。请确保已运行 'maturin develop' 或 'pip install -e .'",
        ImportWarning
    )

# 便捷的别名
DatabaseBridge = PyDbQueueBridge
"#,
        package_name, version, version
    );
    
    fs::write(&init_py_path, init_py_content)
        .expect("Failed to write __init__.py");
    
    println!("Generated Python package structure for {}", package_name);
    println!("Package version: {}", version);
}