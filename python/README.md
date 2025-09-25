# rat_quickdb Python 绑定

高性能跨数据库ORM库的Python绑定，基于Rust和PyO3构建，支持SQLite、PostgreSQL、MySQL、MongoDB的统一接口。

## 特性

- 🚀 **高性能**: 基于Rust的零拷贝设计，无锁队列通信
- 🔄 **数据库无关性**: 统一接口支持多种数据库后端
- 📝 **ODM模型系统**: 强类型字段定义和自动验证
- 🛡️ **类型安全**: 编译时类型检查，运行时零开销
- 📊 **完整CRUD**: 支持创建、查询、更新、删除操作
- 🎯 **调用者控制日志**: 灵活的日志初始化和配置选项

## 安装

```bash
# 开发模式安装
cd python
maturin develop

# 或者构建发布版本
maturin build --release
pip install target/wheels/*.whl
```

## 快速开始

### 1. 日志系统初始化（推荐）

rat_quickdb遵循"调用者控制"的设计理念，日志系统由您完全控制：

```python
from rat_quickdb_py import init_logging, init_logging_with_level, init_logging_advanced

# 方式1: 基本初始化
init_logging()

# 方式2: 指定日志级别
init_logging_with_level("info")  # trace, debug, info, warn, error

# 方式3: 高级配置
init_logging_advanced(
    level="debug",
    enable_color=True,
    timestamp_format="%Y-%m-%d %H:%M:%S",
    custom_format_template="[{timestamp}] {level} PYTHON - {message}"
)
```

### 2. 基础使用

```python
from rat_quickdb_py import create_db_queue_bridge, log_info, log_error
import json

# 初始化日志（推荐）
init_logging_with_level("info")

log_info("开始使用rat_quickdb")

# 创建数据库桥接器
bridge = create_db_queue_bridge()

# 创建记录
user_data = json.dumps({
    "name": "张三",
    "age": 25,
    "email": "zhangsan@example.com",
    "active": True
})

record_id = bridge.create("users", user_data)
log_info(f"记录创建成功，ID: {record_id}")

# 查询记录
query = json.dumps([
    {"field": "name", "operator": "Eq", "value": "张三"}
])
found_records = bridge.find("users", query)
log_info(f"查询结果: {found_records}")

# 更新记录
update_data = json.dumps({"age": 26})
updated_count = bridge.update("users", query, update_data)
log_info(f"更新结果: {updated_count}")

# 删除记录
deleted_count = bridge.delete("users", query)
log_info(f"删除结果: {deleted_count}")
```

### 3. 多数据库支持

```python
from rat_quickdb_py import DbQueueBridge, log_info
import json

# 初始化日志
init_logging_with_level("info")

# 创建桥接器
bridge = DbQueueBridge()

# 添加SQLite数据库
bridge.add_sqlite_database(
    alias="sqlite_db",
    path="./app.db",
    pool_config='{"max_connections": 10, "min_connections": 1}'
)

# 添加PostgreSQL数据库
bridge.add_postgresql_database(
    alias="postgres_db",
    host="localhost",
    port=5432,
    database="testdb",
    username="user",
    password="password",
    pool_config='{"max_connections": 20, "min_connections": 2}'
)

# 添加MySQL数据库
bridge.add_mysql_database(
    alias="mysql_db",
    host="localhost",
    port=3306,
    database="testdb",
    username="user",
    password="password",
    pool_config='{"max_connections": 15, "min_connections": 2}'
)

# 添加MongoDB数据库
bridge.add_mongodb_database(
    alias="mongo_db",
    host="localhost",
    port=27017,
    database="testdb",
    username="user",
    password="password",
    pool_config='{"max_connections": 10, "min_connections": 1}'
)

# 设置默认数据库
bridge.set_default_alias("sqlite_db")

# 在不同数据库中操作
user_data = json.dumps({"name": "李四", "age": 30})

for db_alias in ["sqlite_db", "postgres_db", "mysql_db", "mongo_db"]:
    try:
        result = bridge.create("users", user_data, db_alias)
        log_info(f"在 {db_alias} 中创建用户: {result}")
    except Exception as e:
        log_error(f"操作 {db_alias} 失败: {e}")
```

### 4. 高级查询操作

```python
import json

# 简单等值查询
simple_query = json.dumps({"name": "张三"})
results = bridge.find("users", simple_query)

# 多条件AND查询
and_query = json.dumps([
    {"field": "age", "operator": "Gte", "value": 25},
    {"field": "active", "operator": "Eq", "value": True}
])
results = bridge.find("users", and_query)

# OR逻辑查询
or_query = json.dumps({
    "operator": "or",
    "conditions": [
        {"field": "age", "operator": "Gt", "value": 35},
        {"field": "department", "operator": "Eq", "value": "管理部"}
    ]
})
results = bridge.find("users", or_query)

# 复杂嵌套查询
complex_query = json.dumps({
    "operator": "or",
    "conditions": [
        {
            "operator": "and",
            "conditions": [
                {"field": "age", "operator": "Gte", "value": 25},
                {"field": "department", "operator": "Eq", "value": "技术部"}
            ]
        },
        {
            "operator": "and",
            "conditions": [
                {"field": "salary", "operator": "Gt", "value": 12000},
                {"field": "department", "operator": "Eq", "value": "销售部"}
            ]
        }
    ]
})
results = bridge.find("users", complex_query)
```

### 5. ODM模型系统

```python
from rat_quickdb_py import (
    string_field, integer_field, boolean_field, datetime_field,
    IndexDefinition, ModelMeta, register_model
)

# 定义字段
name_field = string_field(max_length=50, min_length=2).required().unique()
age_field = integer_field(min_value=0, max_value=150)
email_field = string_field(max_length=255).required().unique()
active_field = boolean_field().required()
created_at_field = datetime_field().required()

# 定义索引
name_index = IndexDefinition(["name"], unique=True, name="idx_name_unique")
email_index = IndexDefinition(["email"], unique=True, name="idx_email_unique")
age_index = IndexDefinition(["age"], unique=False, name="idx_age")

# 创建模型元数据
fields = {
    "name": name_field,
    "age": age_field,
    "email": email_field,
    "active": active_field,
    "created_at": created_at_field
}
indexes = [name_index, email_index, age_index]

user_meta = ModelMeta(
    collection_name="users",
    fields=fields,
    indexes=indexes
)

# 注册模型
register_model("User", user_meta)
```

## 支持的查询操作符

- `Eq` - 等于
- `Ne` - 不等于
- `Gt` - 大于
- `Gte` - 大于等于
- `Lt` - 小于
- `Lte` - 小于等于
- `Contains` - 包含
- `StartsWith` - 开始于
- `EndsWith` - 结束于
- `In` - 在列表中
- `NotIn` - 不在列表中
- `Regex` - 正则表达式
- `Exists` - 字段存在
- `IsNull` - 为空
- `IsNotNull` - 不为空

## 日志系统

rat_quickdb提供了灵活的日志配置选项：

```python
from rat_quickdb_py import (
    init_logging, init_logging_with_level, init_logging_advanced,
    is_logging_initialized, log_info, log_error, log_warn, log_debug, log_trace
)

# 检查日志状态
if not is_logging_initialized():
    init_logging_with_level("info")

# 记录不同级别的日志
log_trace("详细跟踪信息")
log_debug("调试信息")
log_info("一般信息")
log_warn("警告信息")
log_error("错误信息")

# 高级日志配置
init_logging_advanced(
    level="debug",
    enable_color=True,
    timestamp_format="%Y-%m-%d %H:%M:%S%.3f",
    custom_format_template="[{timestamp}] {level} {target}:{line} - {message}"
)
```

## 配置选项

### 连接池配置

```python
pool_config = json.dumps({
    "max_connections": 10,
    "min_connections": 1,
    "connection_timeout": 30,
    "idle_timeout": 600,
    "max_lifetime": 3600
})
```

### 缓存配置

```python
from rat_quickdb_py import PyCacheConfig, PyL1CacheConfig, PyL2CacheConfig, PyTtlConfig

# L1缓存配置
l1_config = PyL1CacheConfig(
    max_size=1000,
    ttl_seconds=300
)

# L2缓存配置
l2_config = PyL2CacheConfig(
    max_size=10000,
    ttl_seconds=3600
)

# TTL配置
ttl_config = PyTtlConfig(
    ttl_seconds=1800
)

# 完整缓存配置
cache_config = PyCacheConfig(
    l1_config=l1_config,
    l2_config=l2_config,
    ttl_config=ttl_config,
    enabled=True
)
```

### TLS配置

```python
from rat_quickdb_py import PyTlsConfig

tls_config = PyTlsConfig(
    enabled=True,
    verify_server_cert=False,
    verify_hostname=False
)
```

## 示例代码

查看 `examples/` 目录中的完整示例：

- `caller_init_log_example.py` - 日志初始化示例
- `simple_test.py` - 基础CRUD操作
- `advanced_query_example.py` - 高级查询示例

运行示例：

```bash
python examples/caller_init_log_example.py
python examples/simple_test.py
```

## 性能特点

- **零拷贝设计**: 基于Rust的内存安全保证
- **无锁队列**: 基于crossbeam的高性能并发通信
- **类型安全**: 编译时类型检查，运行时零开销
- **连接池**: 智能连接管理和复用
- **批量操作**: 支持高效的数据批量处理

## 开发说明

### 构建Python模块

```bash
cd python
maturin develop          # 开发模式
maturin build --release  # 发布构建
```

### 运行测试

```bash
python -m pytest tests/
```

### 设计理念

1. **调用者控制**: 日志系统完全由调用者初始化和控制
2. **类型安全**: 强类型定义，编译时检查
3. **性能优先**: 零拷贝设计，最小化开销
4. **数据库无关**: 统一接口支持多种数据库后端

## 许可证

本项目采用 MIT 许可证。