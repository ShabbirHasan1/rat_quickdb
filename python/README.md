# rat_quickdb Python 绑定

一个高性能的跨数据库 ORM 库的 Python 绑定，基于 Rust 和 PyO3 构建。

## 特性

- 🚀 **高性能**: 基于 Rust 的零拷贝设计
- 🔧 **构建器模式**: 类型安全的配置管理
- 📊 **数据库操作**: 完整的 CRUD 操作支持
- 🔄 **队列桥接**: 无锁队列通信机制
- 📝 **模型系统**: 类似 MongoEngine 的模型定义
- 🛡️ **类型安全**: 强类型字段定义和验证

## 安装

```bash
# 开发模式安装
cd python
pip install -e .
```

## 快速开始

### 1. 基础信息查询

```python
import rat_quickdb_py

# 获取库信息
print(f"库名称: {rat_quickdb_py.get_name()}")
print(f"版本号: {rat_quickdb_py.get_version()}")
print(f"库信息: {rat_quickdb_py.get_info()}")
```

### 2. 配置管理（构建器模式）

```python
from rat_quickdb_py import PoolConfigBuilder, create_default_pool_config

# 使用构建器创建配置
builder = PoolConfigBuilder()
config = (
    builder
    .max_connections(20)
    .min_connections(5)
    .connection_timeout(30)
    .idle_timeout(300)
    .max_lifetime(3600)
    .build()
)

print(f"最大连接数: {config.max_connections}")
print(f"最小连接数: {config.min_connections}")

# 使用默认配置
default_config = create_default_pool_config(min_connections=2, max_connections=10)
```

### 3. 数据库操作

```python
from rat_quickdb_py import (
    create_simple_db_manager, 
    DataValue, 
    QueryOperator, 
    QueryCondition
)

# 创建数据库管理器
db_manager = create_simple_db_manager()

# 测试连接
print(f"连接状态: {db_manager.test_connection()}")

# 创建记录
user_data = {
    "name": "张三",
    "age": "25",
    "email": "zhangsan@example.com",
    "active": "true"
}
record_id = db_manager.create_record("users", user_data)
print(f"记录创建成功，ID: {record_id}")

# 查询记录
condition = QueryCondition("name", QueryOperator.eq(), DataValue.string("张三"))
found_records = db_manager.find_records("users", [condition])
print(f"查询到 {len(found_records)} 条记录")

# 更新记录
update_data = {"age": "26"}
updated_count = db_manager.update_records("users", [condition], update_data)
print(f"更新了 {updated_count} 条记录")

# 统计记录
total_count = db_manager.count_records("users", [])
print(f"总记录数: {total_count}")
```

### 4. 队列桥接器

```python
import json
from rat_quickdb_py import create_simple_queue_bridge

# 创建队列桥接器
queue_bridge = create_simple_queue_bridge()

# 测试连接
print(f"队列连接状态: {queue_bridge.test_connection()}")

# 创建队列任务
task_data = {
    "task_id": "task_001",
    "priority": "1",
    "payload": json.dumps({"action": "process_data", "data": [1, 2, 3]})
}
task_id = queue_bridge.create_record("task_queue", json.dumps(task_data))
print(f"任务创建成功，ID: {task_id}")

# 查询队列任务
query_conditions = json.dumps([{"field": "task_id", "operator": "eq", "value": "task_001"}])
found_tasks = queue_bridge.find_records("task_queue", query_conditions)
print(f"查询到 {len(found_tasks)} 个任务")

# 获取队列统计
stats = queue_bridge.get_queue_stats()
print(f"队列统计: {stats}")
```

### 5. 模型系统

```python
from rat_quickdb_py import (
    FieldType, 
    FieldDefinition, 
    IndexDefinition, 
    ModelMeta, 
    create_model_manager
)

# 定义字段类型
string_type = FieldType.string()
integer_type = FieldType.integer()
boolean_type = FieldType.boolean()
datetime_type = FieldType.datetime()

# 定义字段
name_field = FieldDefinition(FieldType.string())
age_field = FieldDefinition(FieldType.integer())
email_field = FieldDefinition(FieldType.string())
active_field = FieldDefinition(FieldType.boolean())

# 定义索引
name_index = IndexDefinition(["name"], unique=True, name="name_unique_idx")
email_index = IndexDefinition(["email"], unique=True, name="email_unique_idx")
age_index = IndexDefinition(["age"], unique=False, name="age_idx")

# 创建模型元数据
fields = {
    "name": name_field,
    "age": age_field,
    "email": email_field,
    "active": active_field
}
indexes = [name_index, email_index, age_index]

user_meta = ModelMeta(
    "users",  # collection_name
    fields,   # fields
    indexes,  # indexes
    "default",  # database_alias
    "用户模型，包含基本用户信息"  # description
)

print(f"模型集合名: {user_meta.get_collection_name()}")
print(f"字段数量: {len(user_meta.get_fields())}")
print(f"索引数量: {len(user_meta.get_indexes())}")

# 创建模型管理器
model_manager = create_model_manager("users")
print(f"模型管理器创建成功: {type(model_manager)}")
```

## 数据类型

### DataValue 类型

```python
from rat_quickdb_py import DataValue

# 支持的数据类型
null_value = DataValue.null()
bool_value = DataValue.bool(True)
int_value = DataValue.int(42)
float_value = DataValue.float(3.14)
string_value = DataValue.string("Hello, World!")

print(f"类型名称: {string_value.type_name()}")
print(f"字符串表示: {string_value}")
```

### FieldType 类型

```python
from rat_quickdb_py import FieldType

# 支持的字段类型
string_field = FieldType.string(max_length=100, min_length=1)
integer_field = FieldType.integer(min_value=0, max_value=150)
float_field = FieldType.float(min_value=0.0, max_value=100.0)
boolean_field = FieldType.boolean()
datetime_field = FieldType.datetime()
uuid_field = FieldType.uuid()
json_field = FieldType.json()
reference_field = FieldType.reference("other_collection")
```

### QueryOperator 操作符

```python
from rat_quickdb_py import QueryOperator

# 支持的查询操作符
eq_op = QueryOperator.eq()          # 等于
ne_op = QueryOperator.ne()          # 不等于
gt_op = QueryOperator.gt()          # 大于
gte_op = QueryOperator.gte()        # 大于等于
lt_op = QueryOperator.lt()          # 小于
lte_op = QueryOperator.lte()        # 小于等于
contains_op = QueryOperator.contains()      # 包含
starts_with_op = QueryOperator.starts_with() # 开始于
ends_with_op = QueryOperator.ends_with()     # 结束于
in_list_op = QueryOperator.in_list()        # 在列表中
not_in_op = QueryOperator.not_in()          # 不在列表中
regex_op = QueryOperator.regex()            # 正则表达式
exists_op = QueryOperator.exists()          # 字段存在
is_null_op = QueryOperator.is_null()        # 为空
is_not_null_op = QueryOperator.is_not_null() # 不为空
```

## 示例

查看 `examples/comprehensive_example.py` 获取完整的使用示例，包括：

- 基础信息查询
- 配置管理
- 数据库 CRUD 操作
- 队列桥接器使用
- 模型系统定义
- 性能测试

运行示例：

```bash
python examples/comprehensive_example.py
```

## 架构特点

### 构建器模式

所有配置都使用构建器模式，确保类型安全和配置完整性：

```python
# 所有配置项必须显式设置
config = (
    PoolConfigBuilder()
    .max_connections(20)     # 必须设置
    .min_connections(5)      # 必须设置
    .connection_timeout(30)  # 必须设置
    .idle_timeout(300)       # 必须设置
    .max_lifetime(3600)      # 必须设置
    .build()                 # 构建配置
)
```

### 无锁队列通信

基于 crossbeam SegQueue 的高性能无锁队列：

```python
# 队列桥接器提供线程安全的消息传递
queue_bridge = create_simple_queue_bridge()
stats = queue_bridge.get_queue_stats()  # (request_count, response_count)
```

### 类型安全的模型系统

类似 MongoEngine 的模型定义，但具有更强的类型安全性：

```python
# 字段定义支持验证和约束
age_field = (
    FieldDefinition(FieldType.integer(min_value=0, max_value=150))
    .required()
    .indexed()
    .description("用户年龄")
)
```

## 性能特点

- **零拷贝设计**: 基于 Rust 的内存安全保证
- **无锁队列**: 高并发场景下的优异性能
- **类型安全**: 编译时类型检查，运行时零开销
- **批量操作**: 支持高效的批量数据处理

## 开发说明

### 编译 Python 模块

```bash
cd python
cargo build --release
maturin develop
```

### 运行测试

```bash
python -m pytest tests/
```

### 代码规范

- 所有注释和错误信息使用中文
- 严格遵循构建器模式
- 所有配置项必须显式设置
- 使用项目内的 zerg_creep 日志库

## 许可证

本项目采用 MIT 许可证。