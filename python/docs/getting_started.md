# RatQuickDB Python 快速入门指南

## 简介

RatQuickDB 是一个高性能的数据库抽象层，提供统一的 API 接口支持多种数据库后端，并内置强大的多级缓存系统。本指南将帮助您快速上手使用 RatQuickDB Python 绑定。

## 特性

- 🚀 **高性能**：内置 L1/L2 两级缓存，显著提升查询性能
- 🔧 **多数据库支持**：SQLite、MySQL、PostgreSQL、MongoDB
- 🎯 **统一 API**：一套 API 操作所有支持的数据库
- 🛡️ **类型安全**：基于 Rust 构建，提供类型安全的操作
- ⚡ **异步支持**：底层异步架构，支持高并发
- 🔍 **强大查询**：15种查询操作符，支持复杂查询条件

## 安装

### 开发环境安装

```bash
# 克隆项目
git clone <repository-url>
cd rat/rat_quickdb/python

# 安装开发依赖
pip install maturin

# 编译并安装
maturin develop
```

### 生产环境安装

```bash
pip install rat-quickdb-py
```

## 5分钟快速体验

### 1. 基础设置

```python
#!/usr/bin/env python3
# -*- coding: utf-8 -*-

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

print("✅ RatQuickDB 初始化完成")
```

### 2. 连接数据库

```python
# SQLite 数据库配置（推荐用于快速体验）
db_config = {
    "type": "sqlite",
    "connection_string": "./quickstart.db"
}

# 添加数据库连接
result = bridge.add_database("demo_db", json.dumps(db_config))
print(f"📦 数据库连接结果: {result}")
```

### 3. 创建数据

```python
# 创建单条记录
user_data = {
    "name": "张三",
    "age": 28,
    "email": "zhangsan@example.com",
    "department": "技术部",
    "salary": 8000,
    "city": "北京"
}

result = bridge.create("users", json.dumps(user_data), "demo_db")
print(f"👤 创建用户结果: {result}")

# 批量创建记录
users_data = [
    {"name": "李四", "age": 25, "department": "产品部", "salary": 7500, "city": "上海"},
    {"name": "王五", "age": 30, "department": "设计部", "salary": 7000, "city": "广州"},
    {"name": "赵六", "age": 26, "department": "技术部", "salary": 8500, "city": "深圳"}
]

result = bridge.batch_create("users", json.dumps(users_data), "demo_db")
print(f"👥 批量创建结果: {result}")
```

### 4. 查询数据

```python
# 简单查询 - 查找特定用户
query = json.dumps({"name": "张三"})
result = bridge.find("users", query, "demo_db")
print(f"🔍 简单查询结果: {result}")

# 条件查询 - 查找技术部员工
query = json.dumps({"department": "技术部"})
result = bridge.find("users", query, "demo_db")
print(f"🔍 部门查询结果: {result}")

# 复杂查询 - 年龄大于25且薪资大于7500的员工
query = json.dumps([
    {"field": "age", "operator": "Gt", "value": 25},
    {"field": "salary", "operator": "Gt", "value": 7500}
])
result = bridge.find("users", query, "demo_db")
print(f"🔍 复杂查询结果: {result}")
```

### 5. 更新数据

```python
# 更新张三的薪资
conditions = json.dumps({"name": "张三"})
update_data = json.dumps({"salary": 9000, "city": "杭州"})
result = bridge.update("users", conditions, update_data, "demo_db")
print(f"✏️ 更新结果: {result}")
```

### 6. 统计数据

```python
# 统计总用户数
result = bridge.count("users", json.dumps({}), "demo_db")
print(f"📊 总用户数: {result}")

# 统计技术部人数
result = bridge.count("users", json.dumps({"department": "技术部"}), "demo_db")
print(f"📊 技术部人数: {result}")
```

### 7. 清理资源

```python
# 清理资源
bridge.cleanup()
print("🧹 资源清理完成")
```

## 完整示例代码

将以上代码片段组合成完整的示例：

```python
#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RatQuickDB 快速入门示例"""

import json
from rat_quickdb_py import create_db_queue_bridge, PyCacheConfig, PyL1CacheConfig

def main():
    print("🚀 RatQuickDB 快速入门示例")
    print("=" * 50)
    
    # 1. 初始化
    cache_config = PyCacheConfig.builder() \
        .l1_cache(PyL1CacheConfig.builder() \
            .capacity(1000) \
            .memory_limit_mb(50) \
            .build()) \
        .build()
    
    bridge = create_db_queue_bridge(cache_config)
    print("✅ RatQuickDB 初始化完成")
    
    try:
        # 2. 连接数据库
        db_config = {
            "type": "sqlite",
            "connection_string": "./quickstart.db"
        }
        
        result = bridge.add_database("demo_db", json.dumps(db_config))
        print(f"📦 数据库连接结果: {result}")
        
        # 3. 创建数据
        print("\n📝 创建数据...")
        
        # 单条创建
        user_data = {
            "name": "张三",
            "age": 28,
            "email": "zhangsan@example.com",
            "department": "技术部",
            "salary": 8000,
            "city": "北京"
        }
        
        result = bridge.create("users", json.dumps(user_data), "demo_db")
        print(f"👤 创建用户: {json.loads(result).get('success', False)}")
        
        # 批量创建
        users_data = [
            {"name": "李四", "age": 25, "department": "产品部", "salary": 7500, "city": "上海"},
            {"name": "王五", "age": 30, "department": "设计部", "salary": 7000, "city": "广州"},
            {"name": "赵六", "age": 26, "department": "技术部", "salary": 8500, "city": "深圳"}
        ]
        
        result = bridge.batch_create("users", json.dumps(users_data), "demo_db")
        print(f"👥 批量创建: {json.loads(result).get('success', False)}")
        
        # 4. 查询数据
        print("\n🔍 查询数据...")
        
        # 简单查询
        query = json.dumps({"name": "张三"})
        result = bridge.find("users", query, "demo_db")
        result_data = json.loads(result)
        if result_data.get('success'):
            users = result_data.get('data', [])
            print(f"👤 找到用户: {users[0]['name']} (年龄: {users[0]['age']})")
        
        # 部门查询
        query = json.dumps({"department": "技术部"})
        result = bridge.find("users", query, "demo_db")
        result_data = json.loads(result)
        if result_data.get('success'):
            users = result_data.get('data', [])
            print(f"🏢 技术部员工数量: {len(users)}")
        
        # 复杂查询
        query = json.dumps([
            {"field": "age", "operator": "Gt", "value": 25},
            {"field": "salary", "operator": "Gt", "value": 7500}
        ])
        result = bridge.find("users", query, "demo_db")
        result_data = json.loads(result)
        if result_data.get('success'):
            users = result_data.get('data', [])
            print(f"💰 高薪资员工数量: {len(users)}")
        
        # 5. 更新数据
        print("\n✏️ 更新数据...")
        conditions = json.dumps({"name": "张三"})
        update_data = json.dumps({"salary": 9000, "city": "杭州"})
        result = bridge.update("users", conditions, update_data, "demo_db")
        print(f"📝 更新张三信息: {json.loads(result).get('success', False)}")
        
        # 6. 统计数据
        print("\n📊 统计数据...")
        result = bridge.count("users", json.dumps({}), "demo_db")
        result_data = json.loads(result)
        if result_data.get('success'):
            total = result_data.get('data', 0)
            print(f"👥 总用户数: {total}")
        
        result = bridge.count("users", json.dumps({"department": "技术部"}), "demo_db")
        result_data = json.loads(result)
        if result_data.get('success'):
            tech_count = result_data.get('data', 0)
            print(f"🏢 技术部人数: {tech_count}")
        
        print("\n🎉 快速入门示例完成！")
        
    except Exception as e:
        print(f"❌ 发生错误: {e}")
    
    finally:
        # 7. 清理资源
        bridge.cleanup()
        print("🧹 资源清理完成")

if __name__ == "__main__":
    main()
```

## 运行示例

保存上述代码为 `quickstart.py`，然后运行：

```bash
python quickstart.py
```

预期输出：

```
🚀 RatQuickDB 快速入门示例
==================================================
✅ RatQuickDB 初始化完成
📦 数据库连接结果: {"success":true,"data":null,"error":null}

📝 创建数据...
👤 创建用户: True
👥 批量创建: True

🔍 查询数据...
👤 找到用户: 张三 (年龄: 28)
🏢 技术部员工数量: 2
💰 高薪资员工数量: 2

✏️ 更新数据...
📝 更新张三信息: True

📊 统计数据...
👥 总用户数: 4
🏢 技术部人数: 2

🎉 快速入门示例完成！
🧹 资源清理完成
```

## 下一步

恭喜！您已经成功完成了 RatQuickDB 的快速入门。接下来您可以：

### 1. 深入学习查询操作符

查看 [查询操作符指南](query_operators_guide.md) 了解所有15种查询操作符的详细用法。

### 2. 学习完整 API

查看 [API 参考文档](api_reference.md) 了解所有可用的 API 接口。

### 3. 配置缓存系统

```python
# 配置更强大的缓存系统
from rat_quickdb_py import PyL2CacheConfig, PyTtlConfig, PyCompressionConfig

cache_config = PyCacheConfig.builder() \
    .l1_cache(PyL1CacheConfig.builder() \
        .capacity(10000) \
        .memory_limit_mb(100) \
        .ttl_config(PyTtlConfig.builder() \
            .default_ttl_seconds(300) \
            .max_ttl_seconds(3600) \
            .build()) \
        .build()) \
    .l2_cache(PyL2CacheConfig.builder() \
        .redis_url("redis://localhost:6379") \
        .key_prefix("myapp:") \
        .compression_config(PyCompressionConfig.builder() \
            .algorithm("zstd") \
            .level(3) \
            .build()) \
        .build()) \
    .build()
```

### 4. 连接其他数据库

```python
# MySQL
mysql_config = {
    "type": "mysql",
    "connection_string": "mysql://user:password@localhost:3306/database"
}

# PostgreSQL
pg_config = {
    "type": "postgresql",
    "connection_string": "postgresql://user:password@localhost:5432/database"
}

# MongoDB
mongo_config = {
    "type": "mongodb",
    "connection_string": "mongodb://localhost:27017/database"
}
```

### 5. 性能优化

```python
# 使用批量操作提升性能
batch_data = []
for i in range(1000):
    batch_data.append({
        "name": f"用户{i}",
        "age": 20 + (i % 40),
        "department": ["技术部", "产品部", "设计部"][i % 3]
    })

# 批量插入比循环单条插入快得多
result = bridge.batch_create("users", json.dumps(batch_data), "demo_db")
```

### 6. 错误处理最佳实践

```python
def safe_query(bridge, collection, query, db_alias):
    """安全的查询操作"""
    try:
        result = bridge.find(collection, query, db_alias)
        result_data = json.loads(result)
        
        if result_data.get("success"):
            return result_data.get("data", [])
        else:
            print(f"查询失败: {result_data.get('error')}")
            return []
    
    except json.JSONDecodeError as e:
        print(f"JSON 解析错误: {e}")
        return []
    except Exception as e:
        print(f"查询异常: {e}")
        return []

# 使用示例
users = safe_query(bridge, "users", json.dumps({"department": "技术部"}), "demo_db")
print(f"找到 {len(users)} 个技术部员工")
```

## 常见问题

### Q: 如何选择合适的数据库类型？

A: 
- **SQLite**: 适合开发测试、小型应用、单机部署
- **MySQL**: 适合中小型 Web 应用、成熟的生态系统
- **PostgreSQL**: 适合复杂查询、数据完整性要求高的应用
- **MongoDB**: 适合文档型数据、快速迭代的应用

### Q: 缓存配置如何选择？

A:
- **L1 缓存**: 内存缓存，速度最快，适合热点数据
- **L2 缓存**: Redis 缓存，容量更大，适合共享缓存
- **TTL 配置**: 根据数据更新频率设置，频繁更新的数据设置较短 TTL

### Q: 如何监控性能？

A: 可以通过以下方式监控：

```python
import time

# 查询性能测试
start_time = time.time()
for i in range(100):
    result = bridge.find("users", query, "demo_db")
end_time = time.time()

print(f"100次查询耗时: {(end_time - start_time) * 1000:.2f}ms")
print(f"平均单次查询: {(end_time - start_time) * 10:.2f}ms")
```

### Q: 如何处理大量数据？

A: 
1. 使用批量操作 (`batch_create`)
2. 合理设置缓存容量
3. 使用索引优化查询
4. 分页查询大结果集

## 总结

通过本快速入门指南，您已经学会了：

✅ 安装和初始化 RatQuickDB  
✅ 连接数据库  
✅ 执行 CRUD 操作  
✅ 使用查询操作符  
✅ 配置缓存系统  
✅ 处理错误和优化性能  

RatQuickDB 提供了强大而灵活的数据库抽象层，帮助您构建高性能的数据驱动应用。继续探索更多高级功能，充分发挥 RatQuickDB 的潜力！

---

📚 **相关文档**
- [查询操作符指南](query_operators_guide.md)
- [API 参考文档](api_reference.md)
- [示例代码](../examples/)

🔗 **有用链接**
- GitHub 仓库: [链接]
- 问题反馈: [链接]
- 社区讨论: [链接]