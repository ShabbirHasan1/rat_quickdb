#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
简化的SQL数据库JSON字段解析验证
专注于JSON字段功能，避免复杂的问题
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def test_sqlite_json():
    """测试SQLite JSON字段解析"""
    print("\n" + "="*50)
    print("🚀 测试 SQLite JSON字段解析")
    print("="*50)

    try:
        bridge = rq.create_db_queue_bridge()

        # 添加SQLite数据库
        result = bridge.add_sqlite_database(
            alias="sqlite_test",
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

        # 创建简单的字段定义
        id_field = rq.integer_field(True, True, None, None, "主键ID")
        name_field = rq.string_field(True, False, None, None, "名称")
        json_field = rq.json_field(False, "JSON数据")

        # 创建索引
        index_def = rq.IndexDefinition(["id"], True, "idx_id")

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_data": json_field
        }

        # 创建模型元数据
        table_name = f"test_json_{int(time.time())}"
        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "sqlite_test",
            "JSON测试表"
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
            "json_data": {
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
                },
                "tags": ["test", "sqlite", "json"]
            }
        }

        # 插入数据
        insert_result = bridge.create(table_name, json.dumps(test_data), "sqlite_test")
        insert_data = json.loads(insert_result)

        if not insert_data.get("success"):
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return False

        print("✅ 数据插入成功")

        # 查询数据
        query_result = bridge.find(table_name, '{}', "sqlite_test")
        query_data = json.loads(query_result)

        if not query_data.get("success"):
            print(f"❌ 数据查询失败: {query_data.get('error')}")
            return False

        records = query_data.get("data")
        if not records or len(records) == 0:
            print("❌ 查询结果为空")
            return False

        record = records[0]
        print(f"✅ 数据查询成功")

        # 验证JSON字段
        json_field = record.get('json_data')
        print(f"   json_data: {json_field}")
        print(f"   json_data类型: {type(json_field)}")

        if isinstance(json_field, dict):
            print("✅ JSON字段正确解析为dict")

            # 验证嵌套结构
            user = json_field.get('user', {})
            if isinstance(user, dict):
                print(f"✅ user.name: {user.get('name')}")
                print(f"✅ user.age: {user.get('age')}")
                print(f"✅ user.active: {user.get('active')}")

            config = json_field.get('config', {})
            if isinstance(config, dict):
                notifications = config.get('notifications', {})
                if isinstance(notifications, dict):
                    print(f"✅ config.notifications.email: {notifications.get('email')}")
                    print(f"✅ config.notifications.sms: {notifications.get('sms')}")

            tags = json_field.get('tags', [])
            if isinstance(tags, list):
                print(f"✅ tags数组: {tags}")
        else:
            print(f"❌ JSON字段解析失败: {type(json_field)}")
            return False

        # 清理
        bridge.drop_table(table_name, "sqlite_test")
        print("✅ SQLite测试完成")
        return True

    except Exception as e:
        print(f"❌ SQLite测试异常: {e}")
        return False

def test_mysql_json():
    """测试MySQL JSON字段解析"""
    print("\n" + "="*50)
    print("🚀 测试 MySQL JSON字段解析")
    print("="*50)

    try:
        bridge = rq.create_db_queue_bridge()

        # 添加MySQL数据库
        result = bridge.add_mysql_database(
            alias="mysql_test",
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
        json_field = rq.json_field(False, "JSON数据")

        # 创建索引
        index_def = rq.IndexDefinition(["id"], True, "idx_id")

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_data": json_field
        }

        # 创建模型元数据
        table_name = f"mysql_json_{int(time.time())}"
        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "mysql_test",
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
            "json_data": {
                "product": {
                    "id": "P001",
                    "name": "测试产品",
                    "price": 99.99,
                    "in_stock": True
                },
                "metadata": {
                    "category": "电子产品",
                    "tags": ["电脑", "测试"],
                    "created": "2025-01-01"
                },
                "specs": {
                    "cpu": "Intel i7",
                    "memory": "16GB",
                    "storage": "512GB"
                }
            }
        }

        # 插入数据
        insert_result = bridge.create(table_name, json.dumps(test_data), "mysql_test")
        insert_data = json.loads(insert_result)

        if not insert_data.get("success"):
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return False

        print("✅ 数据插入成功")

        # 查询数据
        query_result = bridge.find(table_name, '{}', "mysql_test")
        query_data = json.loads(query_result)

        if not query_data.get("success"):
            print(f"❌ 数据查询失败: {query_data.get('error')}")
            return False

        records = query_data.get("data")
        if not records or len(records) == 0:
            print("❌ 查询结果为空")
            return False

        record = records[0]
        print(f"✅ 数据查询成功")

        # 验证JSON字段
        json_field = record.get('json_data')
        print(f"   json_data: {json_field}")
        print(f"   json_data类型: {type(json_field)}")

        if isinstance(json_field, dict):
            print("✅ JSON字段正确解析为dict")

            # 验证嵌套结构
            product = json_field.get('product', {})
            if isinstance(product, dict):
                print(f"✅ product.name: {product.get('name')}")
                print(f"✅ product.price: {product.get('price')}")
                print(f"✅ product.in_stock: {product.get('in_stock')}")

            metadata = json_field.get('metadata', {})
            if isinstance(metadata, dict):
                print(f"✅ metadata.category: {metadata.get('category')}")
                print(f"✅ metadata.tags: {metadata.get('tags')}")

            specs = json_field.get('specs', {})
            if isinstance(specs, dict):
                print(f"✅ specs.cpu: {specs.get('cpu')}")
                print(f"✅ specs.memory: {specs.get('memory')}")
        else:
            print(f"❌ JSON字段解析失败: {type(json_field)}")
            return False

        # 清理
        bridge.drop_table(table_name, "mysql_test")
        print("✅ MySQL测试完成")
        return True

    except Exception as e:
        print(f"❌ MySQL测试异常: {e}")
        return False

def test_postgresql_json():
    """测试PostgreSQL JSON字段解析"""
    print("\n" + "="*50)
    print("🚀 测试 PostgreSQL JSON字段解析")
    print("="*50)

    try:
        bridge = rq.create_db_queue_bridge()

        # 添加PostgreSQL数据库
        result = bridge.add_postgresql_database(
            alias="postgresql_test",
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
        json_field = rq.json_field(False, "JSON数据")

        # 创建索引
        index_def = rq.IndexDefinition(["id"], True, "idx_id")

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_data": json_field
        }

        # 创建模型元数据
        table_name = f"pg_json_{int(time.time())}"
        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "postgresql_test",
            "PostgreSQL JSON测试"
        )

        # 注册模型
        register_result = bridge.register_model(model_meta)
        if not json.loads(register_result).get("success"):
            print(f"❌ ODM模型注册失败")
            return False

        print("✅ ODM模型注册成功")

        # 测试数据
        test_data = {
            "name": "PostgreSQL JSON测试",
            "json_data": {
                "document": {
                    "title": "PostgreSQL JSONB测试",
                    "content": "测试JSONB字段功能",
                    "published": True
                },
                "stats": {
                    "views": 1000,
                    "likes": 50,
                    "comments": 10
                },
                "author": {
                    "name": "测试作者",
                    "email": "test@example.com"
                },
                "keywords": ["postgresql", "jsonb", "test"]
            }
        }

        # 插入数据
        insert_result = bridge.create(table_name, json.dumps(test_data), "postgresql_test")
        insert_data = json.loads(insert_result)

        if not insert_data.get("success"):
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return False

        print("✅ 数据插入成功")

        # 查询数据
        query_result = bridge.find(table_name, '{}', "postgresql_test")
        query_data = json.loads(query_result)

        if not query_data.get("success"):
            print(f"❌ 数据查询失败: {query_data.get('error')}")
            return False

        records = query_data.get("data")
        if not records or len(records) == 0:
            print("❌ 查询结果为空")
            return False

        record = records[0]
        print(f"✅ 数据查询成功")

        # 验证JSON字段
        json_field = record.get('json_data')
        print(f"   json_data: {json_field}")
        print(f"   json_data类型: {type(json_field)}")

        if isinstance(json_field, dict):
            print("✅ JSON字段正确解析为dict")

            # 验证嵌套结构
            document = json_field.get('document', {})
            if isinstance(document, dict):
                print(f"✅ document.title: {document.get('title')}")
                print(f"✅ document.published: {document.get('published')}")

            stats = json_field.get('stats', {})
            if isinstance(stats, dict):
                print(f"✅ stats.views: {stats.get('views')}")
                print(f"✅ stats.likes: {stats.get('likes')}")
                print(f"✅ stats.comments: {stats.get('comments')}")

            author = json_field.get('author', {})
            if isinstance(author, dict):
                print(f"✅ author.name: {author.get('name')}")
                print(f"✅ author.email: {author.get('email')}")

            keywords = json_field.get('keywords', [])
            if isinstance(keywords, list):
                print(f"✅ keywords: {keywords}")
        else:
            print(f"❌ JSON字段解析失败: {type(json_field)}")
            return False

        # 清理
        bridge.drop_table(table_name, "postgresql_test")
        print("✅ PostgreSQL测试完成")
        return True

    except Exception as e:
        print(f"❌ PostgreSQL测试异常: {e}")
        return False

def main():
    """主测试函数"""
    print("🧪 SQL数据库JSON字段解析验证")
    print("分别测试SQLite、MySQL、PostgreSQL")

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

    # 测试各个数据库
    results["SQLite"] = test_sqlite_json()
    results["MySQL"] = test_mysql_json()
    results["PostgreSQL"] = test_postgresql_json()

    # 汇总结果
    print("\n" + "="*50)
    print("🎯 测试结果汇总")
    print("="*50)

    for db, success in results.items():
        status = "✅ 通过" if success else "❌ 失败"
        print(f"{db:12}: {status}")

    total_passed = sum(1 for success in results.values() if success)
    total_count = len(results)

    print(f"\n总计: {total_passed}/{total_count} 个数据库通过测试")

    if total_passed == total_count:
        print("🎉 所有SQL数据库的JSON字段解析功能都正常工作！")
        print("✅ register_model功能在所有数据库中都正常工作！")
        print("✅ ODM模型注册让系统能正确识别和解析JSON字段！")
        return True
    else:
        print("⚠️ 部分数据库的JSON字段解析功能存在问题")
        return False

if __name__ == "__main__":
    main()