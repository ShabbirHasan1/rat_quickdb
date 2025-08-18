# RatQuickDB Python API 参考文档

## 概述

RatQuickDB 是一个高性能的数据库抽象层，支持多种数据库后端，提供统一的 API 接口和强大的缓存功能。本文档详细介绍 Python 绑定的所有 API 接口。

## 安装

```bash
# 开发环境安装
maturin develop

# 生产环境安装
pip install rat-quickdb-py
```

## 快速开始

```python
import json
from rat_quickdb_py import create_db_queue_bridge, PyCacheConfig, PyL1CacheConfig

# 创建缓存配置
cache_config = PyCacheConfig.builder() \
    .l1_cache(PyL1CacheConfig.builder() \
        .capacity(1000) \
        .memory_limit_mb(50) \
        .build()) \
    .build()

# 创建数据库桥接器
bridge = create_db_queue_bridge(cache_config)

# 添加数据库
db_config = {
    "type": "sqlite",
    "connection_string": "./example.db"
}
bridge.add_database("main_db", json.dumps(db_config))

# 执行数据库操作
user_data = {"name": "张三", "age": 25}
result = bridge.create("users", json.dumps(user_data), "main_db")
print(result)
```

## 核心 API

### 1. 创建数据库桥接器

#### create_db_queue_bridge(cache_config)

创建数据库队列桥接器实例。

**参数：**
- `cache_config` (PyCacheConfig): 缓存配置对象

**返回：**
- `PyDbQueueBridge`: 数据库桥接器实例

**示例：**
```python
from rat_quickdb_py import create_db_queue_bridge, PyCacheConfig

cache_config = PyCacheConfig.builder().build()
bridge = create_db_queue_bridge(cache_config)
```

### 2. 数据库管理

#### add_database(alias, config_json)

添加数据库连接。

**参数：**
- `alias` (str): 数据库别名
- `config_json` (str): 数据库配置 JSON 字符串

**返回：**
- `str`: 操作结果 JSON 字符串

**支持的数据库类型：**

##### SQLite
```python
config = {
    "type": "sqlite",
    "connection_string": "./database.db"
}
bridge.add_database("sqlite_db", json.dumps(config))
```

##### MySQL
```python
config = {
    "type": "mysql",
    "connection_string": "mysql://user:password@localhost:3306/database"
}
bridge.add_database("mysql_db", json.dumps(config))
```

##### PostgreSQL
```python
config = {
    "type": "postgresql",
    "connection_string": "postgresql://user:password@localhost:5432/database"
}
bridge.add_database("pg_db", json.dumps(config))
```

##### MongoDB
```python
config = {
    "type": "mongodb",
    "connection_string": "mongodb://localhost:27017/database"
}
bridge.add_database("mongo_db", json.dumps(config))
```

### 3. CRUD 操作

#### create(collection, data_json, database_alias)

创建新记录。

**参数：**
- `collection` (str): 集合/表名
- `data_json` (str): 数据 JSON 字符串
- `database_alias` (str): 数据库别名

**返回：**
- `str`: 操作结果 JSON 字符串

**示例：**
```python
user_data = {
    "name": "张三",
    "age": 25,
    "email": "zhangsan@example.com",
    "department": "技术部"
}
result = bridge.create("users", json.dumps(user_data), "main_db")
```

#### find(collection, conditions_json, database_alias)

查询记录。

**参数：**
- `collection` (str): 集合/表名
- `conditions_json` (str): 查询条件 JSON 字符串
- `database_alias` (str): 数据库别名

**返回：**
- `str`: 查询结果 JSON 字符串

**示例：**
```python
# 简单查询
query = json.dumps({"name": "张三"})
result = bridge.find("users", query, "main_db")

# 复杂查询
query = json.dumps([
    {"field": "age", "operator": "Gte", "value": 25},
    {"field": "department", "operator": "Eq", "value": "技术部"}
])
result = bridge.find("users", query, "main_db")
```

#### update(collection, conditions_json, update_data_json, database_alias)

更新记录。

**参数：**
- `collection` (str): 集合/表名
- `conditions_json` (str): 查询条件 JSON 字符串
- `update_data_json` (str): 更新数据 JSON 字符串
- `database_alias` (str): 数据库别名

**返回：**
- `str`: 操作结果 JSON 字符串

**示例：**
```python
conditions = json.dumps({"name": "张三"})
update_data = json.dumps({"age": 26, "department": "产品部"})
result = bridge.update("users", conditions, update_data, "main_db")
```

#### delete(collection, conditions_json, database_alias)

删除记录。

**参数：**
- `collection` (str): 集合/表名
- `conditions_json` (str): 查询条件 JSON 字符串
- `database_alias` (str): 数据库别名

**返回：**
- `str`: 操作结果 JSON 字符串

**示例：**
```python
conditions = json.dumps({"name": "张三"})
result = bridge.delete("users", conditions, "main_db")
```

### 4. 批量操作

#### batch_create(collection, data_list_json, database_alias)

批量创建记录。

**参数：**
- `collection` (str): 集合/表名
- `data_list_json` (str): 数据列表 JSON 字符串
- `database_alias` (str): 数据库别名

**返回：**
- `str`: 操作结果 JSON 字符串

**示例：**
```python
users_data = [
    {"name": "张三", "age": 25, "department": "技术部"},
    {"name": "李四", "age": 28, "department": "产品部"},
    {"name": "王五", "age": 30, "department": "设计部"}
]
result = bridge.batch_create("users", json.dumps(users_data), "main_db")
```

### 5. 聚合操作

#### count(collection, conditions_json, database_alias)

统计记录数量。

**参数：**
- `collection` (str): 集合/表名
- `conditions_json` (str): 查询条件 JSON 字符串
- `database_alias` (str): 数据库别名

**返回：**
- `str`: 统计结果 JSON 字符串

**示例：**
```python
# 统计所有用户
result = bridge.count("users", json.dumps({}), "main_db")

# 统计特定条件的用户
conditions = json.dumps({"department": "技术部"})
result = bridge.count("users", conditions, "main_db")
```

#### exists(collection, conditions_json, database_alias)

检查记录是否存在。

**参数：**
- `collection` (str): 集合/表名
- `conditions_json` (str): 查询条件 JSON 字符串
- `database_alias` (str): 数据库别名

**返回：**
- `str`: 检查结果 JSON 字符串

**示例：**
```python
conditions = json.dumps({"email": "zhangsan@example.com"})
result = bridge.exists("users", conditions, "main_db")
```

### 6. 资源管理

#### cleanup()

清理资源，关闭所有连接。

**示例：**
```python
bridge.cleanup()
```

## 缓存配置

### PyCacheConfig

缓存配置构建器。

#### 方法

##### builder()

创建缓存配置构建器。

**返回：**
- `PyCacheConfigBuilder`: 配置构建器实例

##### l1_cache(config)

设置 L1 缓存配置。

**参数：**
- `config` (PyL1CacheConfig): L1 缓存配置

**返回：**
- `PyCacheConfigBuilder`: 构建器实例

##### l2_cache(config)

设置 L2 缓存配置。

**参数：**
- `config` (PyL2CacheConfig): L2 缓存配置

**返回：**
- `PyCacheConfigBuilder`: 构建器实例

##### build()

构建缓存配置。

**返回：**
- `PyCacheConfig`: 缓存配置实例

### PyL1CacheConfig

L1 缓存配置。

#### 方法

##### builder()

创建 L1 缓存配置构建器。

##### capacity(size)

设置缓存容量。

**参数：**
- `size` (int): 缓存条目数量

##### memory_limit_mb(limit)

设置内存限制。

**参数：**
- `limit` (int): 内存限制（MB）

##### ttl_config(config)

设置 TTL 配置。

**参数：**
- `config` (PyTtlConfig): TTL 配置

##### build()

构建 L1 缓存配置。

**示例：**
```python
l1_config = PyL1CacheConfig.builder() \
    .capacity(1000) \
    .memory_limit_mb(50) \
    .ttl_config(PyTtlConfig.builder() \
        .default_ttl_seconds(300) \
        .max_ttl_seconds(3600) \
        .build()) \
    .build()
```

### PyL2CacheConfig

L2 缓存配置（Redis）。

#### 方法

##### builder()

创建 L2 缓存配置构建器。

##### redis_url(url)

设置 Redis 连接 URL。

**参数：**
- `url` (str): Redis 连接字符串

##### key_prefix(prefix)

设置键前缀。

**参数：**
- `prefix` (str): 键前缀

##### compression_config(config)

设置压缩配置。

**参数：**
- `config` (PyCompressionConfig): 压缩配置

##### build()

构建 L2 缓存配置。

**示例：**
```python
l2_config = PyL2CacheConfig.builder() \
    .redis_url("redis://localhost:6379") \
    .key_prefix("ratquickdb:") \
    .compression_config(PyCompressionConfig.builder() \
        .algorithm("zstd") \
        .level(3) \
        .build()) \
    .build()
```

### PyTtlConfig

TTL（生存时间）配置。

#### 方法

##### builder()

创建 TTL 配置构建器。

##### default_ttl_seconds(seconds)

设置默认 TTL。

**参数：**
- `seconds` (int): 默认 TTL 秒数

##### max_ttl_seconds(seconds)

设置最大 TTL。

**参数：**
- `seconds` (int): 最大 TTL 秒数

##### build()

构建 TTL 配置。

### PyCompressionConfig

压缩配置。

#### 方法

##### builder()

创建压缩配置构建器。

##### algorithm(algo)

设置压缩算法。

**参数：**
- `algo` (str): 压缩算法（"zstd", "gzip", "lz4"）

##### level(level)

设置压缩级别。

**参数：**
- `level` (int): 压缩级别（1-22）

##### build()

构建压缩配置。

## 工具函数

### get_version()

获取版本信息。

**返回：**
- `str`: 版本字符串

### get_info()

获取库信息。

**返回：**
- `str`: 库信息 JSON 字符串

### get_name()

获取库名称。

**返回：**
- `str`: 库名称

## 错误处理

### 常见错误类型

#### 1. 连接错误

```python
try:
    bridge.add_database("test_db", json.dumps(config))
except Exception as e:
    print(f"数据库连接失败: {e}")
```

#### 2. 查询错误

```python
try:
    result = bridge.find("users", query, "test_db")
    result_data = json.loads(result)
    if not result_data.get("success"):
        print(f"查询失败: {result_data.get('error')}")
except Exception as e:
    print(f"查询异常: {e}")
```

#### 3. JSON 解析错误

```python
try:
    data = json.dumps(user_data)
    result = bridge.create("users", data, "test_db")
except json.JSONEncodeError as e:
    print(f"JSON 编码错误: {e}")
```

## 最佳实践

### 1. 连接管理

```python
# 使用上下文管理器确保资源清理
class DatabaseManager:
    def __init__(self, cache_config):
        self.bridge = create_db_queue_bridge(cache_config)
    
    def __enter__(self):
        return self.bridge
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.bridge.cleanup()

# 使用示例
with DatabaseManager(cache_config) as bridge:
    bridge.add_database("main_db", json.dumps(db_config))
    result = bridge.find("users", query, "main_db")
```

### 2. 错误处理

```python
def safe_database_operation(bridge, operation, *args):
    """安全的数据库操作包装器"""
    try:
        result = operation(*args)
        result_data = json.loads(result)
        
        if result_data.get("success"):
            return result_data.get("data")
        else:
            raise Exception(result_data.get("error", "未知错误"))
    
    except json.JSONDecodeError as e:
        raise Exception(f"JSON 解析错误: {e}")
    except Exception as e:
        raise Exception(f"数据库操作失败: {e}")

# 使用示例
try:
    users = safe_database_operation(
        bridge, 
        bridge.find, 
        "users", 
        json.dumps(query), 
        "main_db"
    )
    print(f"查询到 {len(users)} 个用户")
except Exception as e:
    print(f"操作失败: {e}")
```

### 3. 性能优化

```python
# 1. 合理配置缓存
cache_config = PyCacheConfig.builder() \
    .l1_cache(PyL1CacheConfig.builder() \
        .capacity(10000) \
        .memory_limit_mb(100) \
        .ttl_config(PyTtlConfig.builder() \
            .default_ttl_seconds(300) \
            .max_ttl_seconds(3600) \
            .build()) \
        .build()) \
    .build()

# 2. 使用批量操作
users_data = []
for i in range(1000):
    users_data.append({
        "name": f"用户{i}",
        "age": 20 + (i % 40),
        "department": ["技术部", "产品部", "设计部"][i % 3]
    })

# 批量插入比单条插入效率更高
result = bridge.batch_create("users", json.dumps(users_data), "main_db")

# 3. 优化查询条件
# 使用索引字段进行查询
query = json.dumps({"id": user_id})  # 假设 id 有索引

# 避免全表扫描
query = json.dumps([
    {"field": "department", "operator": "Eq", "value": "技术部"},  # 先过滤
    {"field": "age", "operator": "Gte", "value": 25}  # 再细化
])
```

## 完整示例

```python
#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import time
from rat_quickdb_py import (
    create_db_queue_bridge,
    PyCacheConfig,
    PyL1CacheConfig,
    PyTtlConfig
)

def main():
    # 1. 创建缓存配置
    cache_config = PyCacheConfig.builder() \
        .l1_cache(PyL1CacheConfig.builder() \
            .capacity(1000) \
            .memory_limit_mb(50) \
            .ttl_config(PyTtlConfig.builder() \
                .default_ttl_seconds(300) \
                .max_ttl_seconds(3600) \
                .build()) \
            .build()) \
        .build()
    
    # 2. 创建数据库桥接器
    bridge = create_db_queue_bridge(cache_config)
    
    try:
        # 3. 添加数据库
        db_config = {
            "type": "sqlite",
            "connection_string": "./example.db"
        }
        result = bridge.add_database("main_db", json.dumps(db_config))
        print(f"数据库添加结果: {result}")
        
        # 4. 创建测试数据
        users_data = [
            {"name": "张三", "age": 25, "department": "技术部", "salary": 8000},
            {"name": "李四", "age": 28, "department": "产品部", "salary": 9000},
            {"name": "王五", "age": 30, "department": "设计部", "salary": 7500},
            {"name": "赵六", "age": 26, "department": "技术部", "salary": 8500}
        ]
        
        # 5. 批量插入数据
        result = bridge.batch_create("users", json.dumps(users_data), "main_db")
        print(f"批量创建结果: {result}")
        
        # 6. 执行各种查询
        
        # 简单查询
        query1 = json.dumps({"name": "张三"})
        result1 = bridge.find("users", query1, "main_db")
        print(f"简单查询结果: {result1}")
        
        # 范围查询
        query2 = json.dumps([
            {"field": "age", "operator": "Gte", "value": 25},
            {"field": "age", "operator": "Lte", "value": 30}
        ])
        result2 = bridge.find("users", query2, "main_db")
        print(f"范围查询结果: {result2}")
        
        # 复杂查询
        query3 = json.dumps([
            {"field": "department", "operator": "Eq", "value": "技术部"},
            {"field": "salary", "operator": "Gt", "value": 8000}
        ])
        result3 = bridge.find("users", query3, "main_db")
        print(f"复杂查询结果: {result3}")
        
        # 7. 更新数据
        update_conditions = json.dumps({"name": "张三"})
        update_data = json.dumps({"salary": 8500, "department": "高级技术部"})
        result4 = bridge.update("users", update_conditions, update_data, "main_db")
        print(f"更新结果: {result4}")
        
        # 8. 统计查询
        count_result = bridge.count("users", json.dumps({}), "main_db")
        print(f"总用户数: {count_result}")
        
        # 9. 存在性检查
        exists_result = bridge.exists("users", json.dumps({"name": "张三"}), "main_db")
        print(f"用户存在性检查: {exists_result}")
        
        # 10. 性能测试
        print("\n性能测试...")
        start_time = time.time()
        for i in range(100):
            bridge.find("users", query3, "main_db")
        end_time = time.time()
        print(f"100次查询耗时: {(end_time - start_time) * 1000:.2f}ms")
        
    except Exception as e:
        print(f"操作异常: {e}")
    
    finally:
        # 11. 清理资源
        bridge.cleanup()
        print("资源清理完成")

if __name__ == "__main__":
    main()
```

## 总结

RatQuickDB Python API 提供了完整的数据库抽象层功能，包括：

1. **多数据库支持**：SQLite、MySQL、PostgreSQL、MongoDB
2. **强大的查询功能**：15种查询操作符，3种查询格式
3. **高性能缓存**：L1/L2 两级缓存，支持 TTL 和压缩
4. **完整的 CRUD 操作**：创建、查询、更新、删除、批量操作
5. **聚合功能**：统计、存在性检查
6. **资源管理**：自动连接池管理，优雅关闭

通过合理使用这些 API，可以构建高性能、可扩展的数据库应用程序。