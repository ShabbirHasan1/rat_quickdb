""
"""
rat_quickdb_py - RAT QuickDB Python Bindings

跨数据库ORM库的Python绑定，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

Version: 0.1.3
"""

__version__ = "0.1.3"

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
