""
"""
rat_quickdb_py - RAT QuickDB Python Bindings

跨数据库ORM库的Python绑定，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口

Version: 0.1.6
"""

__version__ = "0.1.6"

# 从Rust编译的模块中导入主要类
# 这些类由maturin在构建时自动注册
try:
    from .rat_quickdb_py import (
        PyDbQueueBridge, create_db_queue_bridge,
        init_logging, init_logging_with_level,
        log_info, log_error, log_warn, log_debug, log_trace
    )
    __all__ = [
        "PyDbQueueBridge", "create_db_queue_bridge",
        "init_logging", "init_logging_with_level",
        "log_info", "log_error", "log_warn", "log_debug", "log_trace"
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
