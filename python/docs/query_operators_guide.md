# RatQuickDB Python 查询操作符使用指南

## 概述

RatQuickDB 提供了丰富的查询操作符，支持多种查询条件格式，满足不同场景的数据查询需求。本文档详细介绍所有支持的查询操作符及其使用方法。

## 查询条件格式

RatQuickDB 支持三种查询条件格式：

### 1. 单个查询条件对象格式

```python
import json

# 格式：{"field": "字段名", "operator": "操作符", "value": "值"}
query = json.dumps({
    "field": "name",
    "operator": "Eq",
    "value": "张三"
})

result = bridge.find("users", query, "database_alias")
```

### 2. 多条件数组格式（AND 逻辑）

```python
# 格式：[{条件1}, {条件2}, ...]
query = json.dumps([
    {"field": "age", "operator": "Gte", "value": 25},
    {"field": "department", "operator": "Eq", "value": "技术部"}
])

result = bridge.find("users", query, "database_alias")
```

### 3. 简化键值对格式（默认使用 Eq 操作符）

```python
# 格式：{"字段1": "值1", "字段2": "值2"}
query = json.dumps({
    "name": "张三",
    "age": 30
})

result = bridge.find("users", query, "database_alias")
```

## 支持的查询操作符

### 1. 相等性操作符

#### Eq - 等于

```python
# 查找姓名为"张三"的用户
query = json.dumps({
    "field": "name",
    "operator": "Eq",
    "value": "张三"
})
```

#### Ne - 不等于

```python
# 查找部门不是"管理部"的用户
query = json.dumps({
    "field": "department",
    "operator": "Ne",
    "value": "管理部"
})
```

### 2. 比较操作符

#### Gt - 大于

```python
# 查找年龄大于30的用户
query = json.dumps({
    "field": "age",
    "operator": "Gt",
    "value": 30
})
```

#### Gte - 大于等于

```python
# 查找薪资大于等于8000的用户
query = json.dumps({
    "field": "salary",
    "operator": "Gte",
    "value": 8000
})
```

#### Lt - 小于

```python
# 查找年龄小于35的用户
query = json.dumps({
    "field": "age",
    "operator": "Lt",
    "value": 35
})
```

#### Lte - 小于等于

```python
# 查找薪资小于等于12000的用户
query = json.dumps({
    "field": "salary",
    "operator": "Lte",
    "value": 12000
})
```

### 3. 字符串操作符

#### Contains - 包含

```python
# 查找城市名包含"京"的用户
query = json.dumps({
    "field": "city",
    "operator": "Contains",
    "value": "京"
})
```

#### StartsWith - 开始于

```python
# 查找姓名以"张"开头的用户
query = json.dumps({
    "field": "name",
    "operator": "StartsWith",
    "value": "张"
})
```

#### EndsWith - 结束于

```python
# 查找邮箱以"@company.com"结尾的用户
query = json.dumps({
    "field": "email",
    "operator": "EndsWith",
    "value": "@company.com"
})
```

### 4. 列表操作符

#### In - 在列表中

```python
# 查找部门在指定列表中的用户
query = json.dumps({
    "field": "department",
    "operator": "In",
    "value": ["技术部", "产品部", "设计部"]
})
```

#### NotIn - 不在列表中

```python
# 查找部门不在指定列表中的用户
query = json.dumps({
    "field": "department",
    "operator": "NotIn",
    "value": ["管理部", "财务部"]
})
```

### 5. 高级操作符

#### Regex - 正则表达式匹配

```python
# 查找手机号符合特定格式的用户
query = json.dumps({
    "field": "phone",
    "operator": "Regex",
    "value": "^1[3-9]\\d{9}$"
})
```

#### Exists - 字段存在

```python
# 查找存在头像字段的用户
query = json.dumps({
    "field": "avatar",
    "operator": "Exists",
    "value": null  # 该操作符不需要具体值
})
```

#### IsNull - 为空

```python
# 查找头像字段为空的用户
query = json.dumps({
    "field": "avatar",
    "operator": "IsNull",
    "value": null  # 该操作符不需要具体值
})
```

#### IsNotNull - 不为空

```python
# 查找头像字段不为空的用户
query = json.dumps({
    "field": "avatar",
    "operator": "IsNotNull",
    "value": null  # 该操作符不需要具体值
})
```

## 复杂查询示例

### 范围查询

```python
# 查找年龄在25-35之间的用户
query = json.dumps([
    {"field": "age", "operator": "Gte", "value": 25},
    {"field": "age", "operator": "Lte", "value": 35}
])
```

### 多条件组合查询

```python
# 查找年龄在25-35之间，薪资大于8000，且部门不是管理部的技术人员
query = json.dumps([
    {"field": "age", "operator": "Gte", "value": 25},
    {"field": "age", "operator": "Lt", "value": 35},
    {"field": "salary", "operator": "Gt", "value": 8000},
    {"field": "department", "operator": "Ne", "value": "管理部"},
    {"field": "role", "operator": "Contains", "value": "工程师"}
])
```

### 字符串模糊匹配

```python
# 查找姓名包含"张"且邮箱包含"@example.com"的用户
query = json.dumps([
    {"field": "name", "operator": "Contains", "value": "张"},
    {"field": "email", "operator": "EndsWith", "value": "@example.com"}
])
```

### 列表查询

```python
# 查找技能包含特定技术栈的开发者
query = json.dumps([
    {"field": "skills", "operator": "In", "value": ["Python", "Rust", "JavaScript"]},
    {"field": "experience_years", "operator": "Gte", "value": 3}
])
```

## 数据类型支持

查询操作符支持以下数据类型：

### 字符串 (String)

```python
{"field": "name", "operator": "Eq", "value": "张三"}
```

### 整数 (Integer)

```python
{"field": "age", "operator": "Gt", "value": 25}
```

### 浮点数 (Float)

```python
{"field": "score", "operator": "Gte", "value": 85.5}
```

### 布尔值 (Boolean)

```python
{"field": "is_active", "operator": "Eq", "value": true}
```

### 数组 (Array)

```python
{"field": "tags", "operator": "In", "value": ["tag1", "tag2", "tag3"]}
```

### 空值 (Null)

```python
{"field": "deleted_at", "operator": "IsNull", "value": null}
```

## 性能优化建议

### 1. 索引优化

- 为经常查询的字段创建索引
- 复合查询考虑创建复合索引
- 避免在低选择性字段上创建索引

### 2. 查询优化

- 优先使用等值查询（Eq）
- 范围查询时尽量缩小范围
- 避免在大文本字段上使用 Contains 操作符
- 合理使用 Limit 限制返回结果数量

### 3. 缓存策略

- 启用 L1 缓存提升重复查询性能
- 根据数据更新频率调整 TTL 配置
- 监控缓存命中率，优化缓存策略

## 错误处理

### 常见错误及解决方案

#### 1. 操作符不支持

```
错误：不支持的操作符: InvalidOp
解决：检查操作符名称是否正确，参考支持的操作符列表
```

#### 2. 数据类型不匹配

```
错误：IN 操作符需要数组类型的值
解决：确保 In 和 NotIn 操作符使用数组类型的值
```

#### 3. JSON 格式错误

```
错误：JSON解析失败
解决：检查 JSON 格式是否正确，特别注意引号和逗号
```

## 完整示例

```python
#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
from rat_quickdb_py import create_db_queue_bridge, PyCacheConfig, PyL1CacheConfig

def main():
    # 初始化数据库桥接器
    cache_config = PyCacheConfig.builder() \
        .l1_cache(PyL1CacheConfig.builder() \
            .capacity(1000) \
            .memory_limit_mb(50) \
            .build()) \
        .build()
    
    bridge = create_db_queue_bridge(cache_config)
    
    # 添加数据库
    db_config = {
        "type": "sqlite",
        "connection_string": "./test.db"
    }
    bridge.add_database("test_db", json.dumps(db_config))
    
    # 创建测试数据
    user_data = {
        "name": "张三",
        "age": 28,
        "department": "技术部",
        "salary": 9000,
        "city": "北京",
        "email": "zhangsan@example.com",
        "is_active": True
    }
    bridge.create("users", json.dumps(user_data), "test_db")
    
    # 1. 简单等值查询
    query1 = json.dumps({
        "field": "name",
        "operator": "Eq",
        "value": "张三"
    })
    result1 = bridge.find("users", query1, "test_db")
    print(f"等值查询结果: {result1}")
    
    # 2. 范围查询
    query2 = json.dumps([
        {"field": "age", "operator": "Gte", "value": 25},
        {"field": "age", "operator": "Lte", "value": 35}
    ])
    result2 = bridge.find("users", query2, "test_db")
    print(f"范围查询结果: {result2}")
    
    # 3. 复杂多条件查询
    query3 = json.dumps([
        {"field": "department", "operator": "Eq", "value": "技术部"},
        {"field": "salary", "operator": "Gt", "value": 8000},
        {"field": "city", "operator": "Contains", "value": "京"},
        {"field": "is_active", "operator": "Eq", "value": True}
    ])
    result3 = bridge.find("users", query3, "test_db")
    print(f"复杂查询结果: {result3}")
    
    # 4. 简化键值对查询
    query4 = json.dumps({
        "department": "技术部",
        "is_active": True
    })
    result4 = bridge.find("users", query4, "test_db")
    print(f"简化查询结果: {result4}")
    
    # 清理资源
    bridge.cleanup()

if __name__ == "__main__":
    main()
```

## 总结

RatQuickDB 提供了功能强大且灵活的查询操作符系统，支持从简单的等值查询到复杂的多条件组合查询。通过合理使用不同的操作符和查询格式，可以满足各种业务场景的数据查询需求。

### 关键要点

1. **三种查询格式**：单条件对象、多条件数组、简化键值对
2. **15种操作符**：覆盖相等性、比较、字符串、列表、高级操作
3. **多种数据类型**：字符串、数字、布尔值、数组、空值
4. **性能优化**：合理使用索引、缓存和查询策略
5. **错误处理**：了解常见错误及解决方案

通过本指南，您应该能够熟练使用 RatQuickDB 的查询功能，构建高效的数据查询应用。