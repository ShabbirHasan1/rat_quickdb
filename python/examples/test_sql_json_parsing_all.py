#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
SQL类型数据库JSON字段解析完整验证
分别测试SQLite、MySQL和PostgreSQL三种数据库的JSON字段解析功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def test_sqlite_json_parsing():
    """测试SQLite JSON字段解析"""
    print("\n" + "="*60)
    print("🚀 测试 SQLite JSON字段解析")
    print("="*60)

    bridge = rq.create_db_queue_bridge()

    # 添加SQLite数据库
    result = bridge.add_sqlite_database(
        alias="test_sqlite_json",
        path=":memory:",
        max_connections=5,
        min_connections=1,
        connection_timeout=30,
        idle_timeout=600,
        max_lifetime=3600
    )

    if not json.loads(result).get("success"):
        print(f"❌ SQLite数据库添加失败")
        return False

    print("✅ SQLite数据库添加成功")

    # 创建字段定义
    id_field = rq.integer_field(True, True, None, None, "主键ID")
    name_field = rq.string_field(True, False, None, None, "名称")
    json_field = rq.json_field(False, "JSON数据字段")

    # 创建数组字段 - 需要指定元素类型
    string_field_type = rq.FieldType.string(None, None)
    array_field = rq.array_field(string_field_type, False, None, None, "数组字段")

    # 创建索引
    index_def = rq.IndexDefinition(["id"], True, "idx_id")

    # 创建字段字典
    fields_dict = {
        "id": id_field,
        "name": name_field,
        "json_field": json_field,
        "array_field": array_field
    }

    # 创建模型元数据
    table_name = f"sqlite_json_test_{int(time.time())}"
    model_meta = rq.ModelMeta(
        table_name,
        fields_dict,
        [index_def],
        "test_sqlite_json",
        "SQLite JSON测试"
    )

    # 注册模型
    register_result = bridge.register_model(model_meta)
    if not json.loads(register_result).get("success"):
        print(f"❌ ODM模型注册失败")
        return False

    print("✅ ODM模型注册成功")

    # 测试数据
    test_data = {
        "name": "SQLite JSON测试",
        "json_field": {
            "user": {
                "name": "张三",
                "age": 30,
                "active": True
            },
            "config": {
                "theme": "dark",
                "notifications": {
                    "email": True,
                    "sms": False
                }
            }
        },
        "array_field": ["苹果", "香蕉", "橙子", {"type": "混合", "count": 2}]
    }

    # 插入数据
    insert_result = bridge.create(table_name, json.dumps(test_data), "test_sqlite_json")
    if not json.loads(insert_result).get("success"):
        print(f"❌ 数据插入失败")
        return False

    print("✅ 数据插入成功")

    # 查询数据
    query_result = bridge.find(table_name, '{}', "test_sqlite_json")
    query_data = json.loads(query_result)

    if not query_data.get("success"):
        print(f"❌ 数据查询失败")
        return False

    records = query_data.get("data")
    if not records or len(records) == 0:
        print("❌ 查询结果为空")
        return False

    record = records[0]
    print(f"✅ 数据查询成功")
    print(f"   记录类型: {type(record)}")

    # 验证JSON字段
    json_field = record.get('json_field')
    print(f"   json_field: {json_field}")
    print(f"   json_field类型: {type(json_field)}")

    if isinstance(json_field, dict):
        print("✅ JSON字段正确解析为dict")

        # 验证嵌套结构
        user = json_field.get('user', {})
        if isinstance(user, dict):
            print(f"✅ user字段: {user}")
            print(f"   user.name: {user.get('name')}")
            print(f"   user.age: {user.get('age')}")
            print(f"   user.active: {user.get('active')}")

        config = json_field.get('config', {})
        if isinstance(config, dict):
            notifications = config.get('notifications', {})
            if isinstance(notifications, dict):
                print(f"✅ config.notifications: {notifications}")
    else:
        print(f"❌ JSON字段解析失败: {type(json_field)}")
        return False

    # 验证数组字段
    array_field = record.get('array_field')
    print(f"   array_field: {array_field}")
    print(f"   array_field类型: {type(array_field)}")

    if isinstance(array_field, list):
        print("✅ 数组字段正确解析为list")
    else:
        print(f"❌ 数组字段解析失败: {type(array_field)}")
        return False

    # 清理
    bridge.drop_table(table_name, "test_sqlite_json")
    print("✅ SQLite测试完成")
    return True

def test_mysql_json_parsing():
    """测试MySQL JSON字段解析"""
    print("\n" + "="*60)
    print("🚀 测试 MySQL JSON字段解析")
    print("="*60)

    bridge = rq.create_db_queue_bridge()

    # 添加MySQL数据库
    result = bridge.add_mysql_database(
        alias="test_mysql_json",
        host="172.16.0.21",
        port=3306,
        database="testdb",
        username="testdb",
        password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
        max_connections=5,
        min_connections=1,
        connection_timeout=30,
        idle_timeout=600,
        max_lifetime=3600
    )

    if not json.loads(result).get("success"):
        print(f"❌ MySQL数据库添加失败")
        return False

    print("✅ MySQL数据库添加成功")

    # 创建字段定义
    id_field = rq.integer_field(True, True, None, None, "主键ID")
    name_field = rq.string_field(True, False, None, None, "名称")
    json_field = rq.json_field(False, "JSON数据字段")

    # 创建数组字段 - 需要指定元素类型
    string_field_type = rq.FieldType.string(None, None)
    array_field = rq.array_field(string_field_type, False, None, None, "数组字段")

    # 创建索引
    index_def = rq.IndexDefinition(["id"], True, "idx_id")

    # 创建字段字典
    fields_dict = {
        "id": id_field,
        "name": name_field,
        "json_field": json_field,
        "array_field": array_field
    }

    # 创建模型元数据
    table_name = f"mysql_json_test_{int(time.time())}"
    model_meta = rq.ModelMeta(
        table_name,
        fields_dict,
        [index_def],
        "test_mysql_json",
        "MySQL JSON测试"
    )

    # 注册模型
    register_result = bridge.register_model(model_meta)
    if not json.loads(register_result).get("success"):
        print(f"❌ ODM模型注册失败")
        return False

    print("✅ ODM模型注册成功")

    # 测试数据
    test_data = {
        "name": "MySQL JSON测试",
        "json_field": {
            "product": {
                "id": "P001",
                "name": "笔记本电脑",
                "specs": {
                    "cpu": "Intel i7",
                    "memory": "16GB",
                    "storage": "512GB SSD"
                },
                "price": 5999.99,
                "in_stock": True
            },
            "metadata": {
                "category": "电子产品",
                "tags": ["电脑", "笔记本", "办公"],
                "created_at": "2025-01-01T00:00:00Z"
            }
        },
        "array_field": [
            {"id": 1, "name": "红色", "code": "#FF0000"},
            {"id": 2, "name": "绿色", "code": "#00FF00"},
            {"id": 3, "name": "蓝色", "code": "#0000FF"}
        ]
    }

    # 插入数据
    insert_result = bridge.create(table_name, json.dumps(test_data), "test_mysql_json")
    if not json.loads(insert_result).get("success"):
        print(f"❌ 数据插入失败")
        return False

    print("✅ 数据插入成功")

    # 查询数据
    query_result = bridge.find(table_name, '{}', "test_mysql_json")
    query_data = json.loads(query_result)

    if not query_data.get("success"):
        print(f"❌ 数据查询失败")
        return False

    records = query_data.get("data")
    if not records or len(records) == 0:
        print("❌ 查询结果为空")
        return False

    record = records[0]
    print(f"✅ 数据查询成功")
    print(f"   记录类型: {type(record)}")

    # 验证JSON字段
    json_field = record.get('json_field')
    print(f"   json_field: {json_field}")
    print(f"   json_field类型: {type(json_field)}")

    if isinstance(json_field, dict):
        print("✅ JSON字段正确解析为dict")

        # 验证嵌套结构
        product = json_field.get('product', {})
        if isinstance(product, dict):
            print(f"✅ product字段: {product.get('name')}")
            specs = product.get('specs', {})
            if isinstance(specs, dict):
                print(f"✅ product.specs: {specs}")
                print(f"   cpu: {specs.get('cpu')}")
                print(f"   memory: {specs.get('memory')}")

        metadata = json_field.get('metadata', {})
        if isinstance(metadata, dict):
            print(f"✅ metadata.tags: {metadata.get('tags')}")
    else:
        print(f"❌ JSON字段解析失败: {type(json_field)}")
        return False

    # 验证数组字段
    array_field = record.get('array_field')
    print(f"   array_field: {array_field}")
    print(f"   array_field类型: {type(array_field)}")

    if isinstance(array_field, list):
        print("✅ 数组字段正确解析为list")
        if len(array_field) > 0 and isinstance(array_field[0], dict):
            print(f"✅ 数组元素也是dict: {array_field[0]}")
    else:
        print(f"❌ 数组字段解析失败: {type(array_field)}")
        return False

    # 清理
    bridge.drop_table(table_name, "test_mysql_json")
    print("✅ MySQL测试完成")
    return True

def test_postgresql_json_parsing():
    """测试PostgreSQL JSON字段解析"""
    print("\n" + "="*60)
    print("🚀 测试 PostgreSQL JSON字段解析")
    print("="*60)

    bridge = rq.create_db_queue_bridge()

    # 添加PostgreSQL数据库
    result = bridge.add_postgresql_database(
        alias="test_postgresql_json",
        host="172.16.0.21",
        port=5432,
        database="testdb",
        username="testdb",
        password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
        max_connections=5,
        min_connections=1,
        connection_timeout=30,
        idle_timeout=600,
        max_lifetime=3600
    )

    if not json.loads(result).get("success"):
        print(f"❌ PostgreSQL数据库添加失败")
        return False

    print("✅ PostgreSQL数据库添加成功")

    # 创建字段定义
    id_field = rq.integer_field(True, True, None, None, "主键ID")
    name_field = rq.string_field(True, False, None, None, "名称")
    json_field = rq.json_field(False, "JSON数据字段")

    # 创建数组字段 - 需要指定元素类型
    string_field_type = rq.FieldType.string(None, None)
    array_field = rq.array_field(string_field_type, False, None, None, "数组字段")

    # 创建索引
    index_def = rq.IndexDefinition(["id"], True, "idx_id")

    # 创建字段字典
    fields_dict = {
        "id": id_field,
        "name": name_field,
        "json_field": json_field,
        "array_field": array_field
    }

    # 创建模型元数据
    table_name = f"postgresql_json_test_{int(time.time())}"
    model_meta = rq.ModelMeta(
        table_name,
        fields_dict,
        [index_def],
        "test_postgresql_json",
        "PostgreSQL JSON测试"
    )

    # 注册模型
    register_result = bridge.register_model(model_meta)
    if not json.loads(register_result).get("success"):
        print(f"❌ ODM模型注册失败")
        return False

    print("✅ ODM模型注册成功")

    # 测试数据 - PostgreSQL特有的JSONB功能测试
    test_data = {
        "name": "PostgreSQL JSON测试",
        "json_field": {
            "document": {
                "title": "PostgreSQL JSONB功能",
                "content": "测试JSONB字段的高级功能",
                "metadata": {
                    "author": "测试用户",
                    "published": True,
                    "stats": {
                        "views": 1000,
                        "likes": 50,
                        "shares": 25
                    }
                }
            },
            "search_config": {
                "full_text_search": True,
                "indexing": {
                    "enabled": True,
                    "fields": ["title", "content"]
                }
            }
        },
        "array_field": [
            "tag1",
            "tag2",
            "tag3",
            {"nested": "object", "in": "array"},
            [1, 2, 3]
        ]
    }

    # 插入数据
    insert_result = bridge.create(table_name, json.dumps(test_data), "test_postgresql_json")
    if not json.loads(insert_result).get("success"):
        print(f"❌ 数据插入失败")
        return False

    print("✅ 数据插入成功")

    # 查询数据
    query_result = bridge.find(table_name, '{}', "test_postgresql_json")
    query_data = json.loads(query_result)

    if not query_data.get("success"):
        print(f"❌ 数据查询失败")
        return False

    records = query_data.get("data")
    if not records or len(records) == 0:
        print("❌ 查询结果为空")
        return False

    record = records[0]
    print(f"✅ 数据查询成功")
    print(f"   记录类型: {type(record)}")

    # 验证JSON字段
    json_field = record.get('json_field')
    print(f"   json_field: {json_field}")
    print(f"   json_field类型: {type(json_field)}")

    if isinstance(json_field, dict):
        print("✅ JSON字段正确解析为dict")

        # 验证深度嵌套结构
        document = json_field.get('document', {})
        if isinstance(document, dict):
            print(f"✅ document.title: {document.get('title')}")

            metadata = document.get('metadata', {})
            if isinstance(metadata, dict):
                stats = metadata.get('stats', {})
                if isinstance(stats, dict):
                    print(f"✅ metadata.stats: {stats}")
                    print(f"   views: {stats.get('views')}")
                    print(f"   likes: {stats.get('likes')}")

        search_config = json_field.get('search_config', {})
        if isinstance(search_config, dict):
            indexing = search_config.get('indexing', {})
            if isinstance(indexing, dict):
                print(f"✅ search_config.indexing.fields: {indexing.get('fields')}")
    else:
        print(f"❌ JSON字段解析失败: {type(json_field)}")
        return False

    # 验证复杂数组字段
    array_field = record.get('array_field')
    print(f"   array_field: {array_field}")
    print(f"   array_field类型: {type(array_field)}")

    if isinstance(array_field, list):
        print("✅ 数组字段正确解析为list")
        print(f"   数组长度: {len(array_field)}")

        # 检查数组中的不同类型元素
        for i, item in enumerate(array_field):
            print(f"   元素[{i}]: {item} (类型: {type(item)})")
    else:
        print(f"❌ 数组字段解析失败: {type(array_field)}")
        return False

    # 清理
    bridge.drop_table(table_name, "test_postgresql_json")
    print("✅ PostgreSQL测试完成")
    return True

def main():
    """主测试函数"""
    print("🧪 SQL类型数据库JSON字段解析完整验证")
    print("测试SQLite、MySQL、PostgreSQL三种数据库")

    # 初始化日志
    try:
        rq.init_logging_with_level("info")
        print("✅ 日志初始化成功")
    except:
        print("⚠️ 日志初始化失败")

    results = {
        "SQLite": False,
        "MySQL": False,
        "PostgreSQL": False
    }

    try:
        results["SQLite"] = test_sqlite_json_parsing()
    except Exception as e:
        print(f"❌ SQLite测试异常: {e}")

    try:
        results["MySQL"] = test_mysql_json_parsing()
    except Exception as e:
        print(f"❌ MySQL测试异常: {e}")

    try:
        results["PostgreSQL"] = test_postgresql_json_parsing()
    except Exception as e:
        print(f"❌ PostgreSQL测试异常: {e}")

    # 汇总结果
    print("\n" + "="*60)
    print("🎯 测试结果汇总")
    print("="*60)

    for db, success in results.items():
        status = "✅ 通过" if success else "❌ 失败"
        print(f"{db:12}: {status}")

    total_passed = sum(1 for success in results.values() if success)
    total_count = len(results)

    print(f"\n总计: {total_passed}/{total_count} 个数据库通过测试")

    if total_passed == total_count:
        print("🎉 所有SQL数据库的JSON字段解析功能都正常工作！")
        return True
    else:
        print("⚠️ 部分数据库的JSON字段解析功能存在问题")
        return False

if __name__ == "__main__":
    main()