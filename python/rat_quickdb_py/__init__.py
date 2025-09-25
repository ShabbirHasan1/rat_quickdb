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
        # 基础函数
        DbQueueBridge, create_db_queue_bridge,
        init_logging, init_logging_with_level,
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
        "init_logging", "init_logging_with_level",
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
