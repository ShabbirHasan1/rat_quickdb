# rat_quickdb Python 绑定

一个高性能的跨数据库 ODM 库的 Python 绑定，基于 Rust 和 PyO3 构建。

## 特性

- 🚀 **高性能**: 基于 Rust 的零拷贝设计
- 🔄 **数据库无关性**: 支持 SQLite、PostgreSQL、MySQL、MongoDB
- 📝 **仿 MongoEngine ODM**: 类似 MongoEngine 的模型定义方式
- 🛡️ **类型安全**: 强类型字段定义和验证
- 📊 **完整 CRUD**: 支持创建、查询、更新、删除操作
- 🔧 **自动启动**: 无需手动启动，创建即可使用

## 安装

```bash
# 开发模式安装
cd python
maturin develop
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

### 2. 创建数据库桥接器

```python
from rat_quickdb_py import create_db_queue_bridge
import json

# 创建数据库队列桥接器（自动启动）
bridge = create_db_queue_bridge()

# 添加 SQLite 数据库
response = bridge.add_sqlite_database(
    alias="default",
    path="./demo.db",
    max_connections=10,
    min_connections=1,
    connection_timeout=30,
    idle_timeout=600,
    max_lifetime=3600
)
result = json.loads(response)
print(f"数据库添加结果: {result}")
```

### 3. 基础 CRUD 操作

```python
# 创建记录
user_data = json.dumps({
    "name": "张三",
    "age": 25,
    "email": "zhangsan@example.com",
    "active": True
})
record_id = bridge.create("users", user_data)
print(f"记录创建成功，ID: {record_id}")

# 查询记录（使用 JSON 格式查询条件）
query_conditions = json.dumps([
    {"field": "name", "operator": "Eq", "value": "张三"}
])
found_records = bridge.find("users", query_conditions)
print(f"查询结果: {found_records}")

# 根据 ID 查询
user_by_id = bridge.find_by_id("users", record_id)
print(f"根据ID查询: {user_by_id}")

# 更新记录
update_data = json.dumps({"age": 26})
updated_count = bridge.update("users", query_conditions, update_data)
print(f"更新结果: {updated_count}")

# 删除记录
deleted_count = bridge.delete("users", query_conditions)
print(f"删除结果: {deleted_count}")
```

### 4. 多数据库支持

```python
# 添加 PostgreSQL 数据库
pg_response = bridge.add_postgresql_database(
    alias="postgres",
    host="localhost",
    port=5432,
    database="testdb",
    username="user",
    password="password",
    max_connections=20,
    min_connections=2
)

# 添加 MySQL 数据库
mysql_response = bridge.add_mysql_database(
    alias="mysql",
    host="localhost",
    port=3306,
    database="testdb",
    username="user",
    password="password",
    max_connections=15,
    min_connections=2
)

# 添加 MongoDB 数据库
mongo_response = bridge.add_mongodb_database(
    alias="mongodb",
    host="localhost",
    port=27017,
    database="testdb",
    username="user",
    password="password",
    max_connections=10,
    min_connections=1
)

# 设置默认数据库别名
bridge.set_default_alias("postgres")

# 在指定数据库中操作
record_id = bridge.create("users", user_data, alias="mysql")
found_records = bridge.find("users", query_conditions, alias="mongodb")
```

### 5. ODM 模型系统（仿 MongoEngine）

```python
from rat_quickdb_py import (
    string_field,
    integer_field, 
    boolean_field,
    datetime_field,
    uuid_field,
    reference_field,
    IndexDefinition, 
    ModelMeta
)

# 使用便捷函数定义字段（类似 MongoEngine）
name_field = string_field(
    required=True,
    unique=True,
    max_length=50,
    min_length=2,
    description="用户名字段"
)

age_field = integer_field(
    required=False,
    min_value=0,
    max_value=150,
    description="年龄字段"
)

email_field = string_field(
    required=True,
    unique=True,
    max_length=255,
    description="邮箱字段"
)

active_field = boolean_field(
    required=True,
    description="激活状态字段"
)

created_at_field = datetime_field(
    required=True,
    description="创建时间字段"
)

# 定义索引
name_index = IndexDefinition(["name"], unique=True, name="idx_name_unique")
email_index = IndexDefinition(["email"], unique=True, name="idx_email_unique")
age_index = IndexDefinition(["age"], unique=False, name="idx_age")
compound_index = IndexDefinition(["active", "created_at"], unique=False, name="idx_active_created")

# 创建模型元数据
fields = {
    "name": name_field,
    "age": age_field,
    "email": email_field,
    "active": active_field,
    "created_at": created_at_field
}
indexes = [name_index, email_index, age_index, compound_index]

user_meta = ModelMeta(
    collection_name="users",
    fields=fields,
    indexes=indexes,
    database_alias="default",
    description="用户信息模型"
)

print(f"模型集合名: {user_meta.collection_name}")
print(f"数据库别名: {user_meta.database_alias}")
print(f"模型描述: {user_meta.description}")
```

## 数据类型和字段

### 支持的字段类型

```python
from rat_quickdb_py import (
    string_field, integer_field, boolean_field, 
    datetime_field, uuid_field, json_field, 
    reference_field, array_field
)

# 字符串字段
name_field = string_field(
    required=True,
    unique=False,
    max_length=100,
    min_length=1,
    description="姓名字段"
)

# 整数字段
age_field = integer_field(
    required=False,
    min_value=0,
    max_value=150,
    description="年龄字段"
)

# 布尔字段
active_field = boolean_field(
    required=True,
    description="激活状态"
)

# 日期时间字段
created_field = datetime_field(
    required=True,
    description="创建时间"
)

# UUID字段
id_field = uuid_field(
    required=True,
    unique=True,
    description="唯一标识"
)

# JSON字段
metadata_field = json_field(
    required=False,
    description="元数据"
)

# 引用字段
author_field = reference_field(
    target_collection="users",
    required=True,
    description="作者引用"
)
```

### 查询操作符

`rat_quickdb` 支持多种查询格式，提供灵活的数据查询方式：

#### 支持的操作符
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

#### 查询格式

> **智能查询路由**: `find` 方法支持自动检测查询类型。当查询包含 `operator` 和 `conditions` 字段时，会自动使用条件组合查询逻辑；否则使用普通查询条件解析。这样您只需要使用一个 `find` 方法就能处理所有类型的查询。

**1. 单个查询条件格式**
```python
import json

# 单个条件查询
query = json.dumps({
    "field": "age", 
    "operator": "Gt", 
    "value": 25
})
results = bridge.find("users", query)
```

**2. 多个查询条件数组格式（AND 逻辑）**
```python
# 多条件 AND 查询
query = json.dumps([
    {"field": "age", "operator": "Gte", "value": 25},
    {"field": "department", "operator": "Eq", "value": "技术部"}
])
results = bridge.find("users", query)
```

**3. 简化的键值对格式（默认使用 Eq 操作符）**
```python
# 简化等值查询
query = json.dumps({
    "name": "张三",
    "department": "技术部"
})
results = bridge.find("users", query)
```

**4. OR 逻辑查询格式**
```python
# OR 逻辑查询
query = json.dumps({
    "operator": "or",
    "conditions": [
        {"field": "age", "operator": "Gt", "value": 35},
        {"field": "salary", "operator": "Gt", "value": 15000}
    ]
})
results = bridge.find("users", query)
```

**5. 混合 AND/OR 查询**
```python
# 复杂逻辑查询
query = json.dumps({
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
results = bridge.find("users", query)
```

## 示例

### 基础 ODM 使用示例

```python
import json
from rat_quickdb_py import create_db_queue_bridge

# 创建数据库桥接器（自动启动）
bridge = create_db_queue_bridge()

# 添加 SQLite 数据库
response = bridge.add_sqlite_database(
    alias="default",
    path="./test.db",
    max_connections=10,
    min_connections=1,
    connection_timeout=30,
    idle_timeout=600,
    max_lifetime=3600
)

# 创建用户数据（类似 MongoEngine 的文档操作）
user_data = json.dumps({
    "name": "张三",
    "age": 25,
    "email": "zhangsan@example.com",
    "active": True,
    "tags": ["python", "rust"],
    "metadata": {"department": "engineering", "level": "senior"}
})

# 插入数据
result = bridge.create("users", user_data)
print(f"创建用户: {result}")

# 简单查询
query = json.dumps({"name": "张三"})
users = bridge.find("users", query)
print(f"查询结果: {users}")

# 条件查询
age_query = json.dumps([
    {"field": "age", "operator": "Gte", "value": 18},
    {"field": "age", "operator": "Lte", "value": 65},
    {"field": "active", "operator": "Eq", "value": True}
])
active_users = bridge.find("users", age_query)
print(f"活跃用户: {len(json.loads(active_users))}")

# 更新数据
update_query = json.dumps({"name": "张三"})
update_data = json.dumps({"age": 26, "last_login": "2024-01-15"})
bridge.update("users", update_query, update_data)
print("用户信息已更新")

# 删除数据
delete_query = json.dumps({"name": "张三"})
bridge.delete("users", delete_query)
print("用户已删除")
```

### 数据库无关性示例

```python
import json
from rat_quickdb_py import create_db_queue_bridge

# 创建桥接器并配置多种数据库
bridge = create_db_queue_bridge()

# 同时支持多种数据库后端
bridge.add_sqlite_database(
    alias="sqlite_db",
    path="./app.db",
    max_connections=10,
    min_connections=1,
    connection_timeout=30,
    idle_timeout=600,
    max_lifetime=3600
)
bridge.add_postgresql_database(
    alias="postgres_db",
    host="localhost",
    port=5432,
    database="testdb",
    username="user",
    password="password",
    max_connections=20,
    min_connections=2,
    connection_timeout=30,
    idle_timeout=600,
    max_lifetime=3600
)
bridge.add_mongodb_database(
    alias="mongo_db",
    host="localhost",
    port=27017,
    database="testdb",
    username="user",
    password="password",
    max_connections=10,
    min_connections=1,
    connection_timeout=30,
    idle_timeout=600,
    max_lifetime=3600
)
bridge.set_default_alias("sqlite_db")

# 相同的 ODM 操作，不同的数据库后端
user_data = json.dumps({
    "name": "李四",
    "age": 30,
    "skills": ["rust", "python", "javascript"],
    "profile": {"bio": "全栈工程师", "location": "北京"}
})

# 在不同数据库中执行相同操作
for db_alias in ["sqlite_db", "postgres_db", "mongo_db"]:
    try:
        # 创建用户
        result = bridge.create("users", user_data, db_alias)
        print(f"在 {db_alias} 中创建用户: {result}")
        
        # 查询用户
        query = json.dumps({"name": "李四"})
        users = bridge.find("users", query, db_alias)
        users_list = json.loads(users)
        print(f"从 {db_alias} 查询到 {len(users_list)} 个用户")
        
        # 更新用户
        update_data = json.dumps({"last_active": "2024-01-15"})
        bridge.update("users", query, update_data, db_alias)
        print(f"在 {db_alias} 中更新用户成功")
        
    except Exception as e:
        print(f"操作 {db_alias} 时出错: {e}")

print("数据库无关性演示完成")
```

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
    PoolConfig.builder()
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