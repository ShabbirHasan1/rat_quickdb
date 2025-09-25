#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
简单数组字段测试脚本
使用正确的 rat_quickdb_py API 测试数组字段功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json

def test_array_fields():
    """测试数组字段功能"""
    print("🚀 开始数组字段功能测试")

    try:
        # 创建数据库桥接器
        bridge = rq.create_db_queue_bridge()
        print("✅ 桥接器创建成功")

        # 添加SQLite数据库
        result = bridge.add_sqlite_database(
            alias="test_db",
            path="./test_arrays.db",
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )

        if json.loads(result).get("success"):
            print("✅ SQLite数据库添加成功")
        else:
            print("❌ SQLite数据库添加失败")
            return

        # 创建测试数据
        test_data = {
            "id": "test1",
            "name": "数组字段测试",
            "tags": ["python", "rust", "database"],
            "categories": ["tech", "programming", "tutorial"],
            "scores": [85, 92, 78, 90],
            "metadata": {
                "level": "advanced",
                "topics": ["arrays", "json", "storage"]
            }
        }

        # 插入数据
        insert_result = bridge.create("test_arrays", json.dumps(test_data), "test_db")
        insert_data = json.loads(insert_result)

        if insert_data.get("success"):
            print("✅ 数据插入成功")
            print(f"  - 记录ID: {insert_data.get('data')}")
        else:
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return

        # 查询数据
        query_result = bridge.find_by_id("test_arrays", "test1", "test_db")
        query_data = json.loads(query_result)

        if query_data.get("success"):
            found = query_data.get("data")
            if found:
                print("✅ 数据查询成功")
                print(f"  - tags: {found.get('tags')} (类型: {type(found.get('tags'))})")
                print(f"  - categories: {found.get('categories')} (类型: {type(found.get('categories'))})")
                print(f"  - scores: {found.get('scores')} (类型: {type(found.get('scores'))})")

                # 验证数组字段
                tags = found.get('tags')
                categories = found.get('categories')
                scores = found.get('scores')

                if isinstance(tags, list) and isinstance(categories, list) and isinstance(scores, list):
                    print("✅ 数组字段正确存储为列表类型")
                    print(f"  - tags数组长度: {len(tags)}")
                    print(f"  - categories数组长度: {len(categories)}")
                    print(f"  - scores数组长度: {len(scores)}")
                else:
                    print("❌ 数组字段存储类型不正确")
            else:
                print("❌ 查询结果为空")
        else:
            print(f"❌ 数据查询失败: {query_data.get('error')}")

        # 测试数组查询
        print("\n🔍 测试数组字段查询...")

        # 查询包含特定标签的记录
        conditions = json.dumps([
            {"field": "tags", "operator": "Contains", "value": "python"}
        ])

        search_result = bridge.find("test_arrays", conditions, "test_db")
        search_data = json.loads(search_result)

        if search_data.get("success"):
            records = search_data.get("data", [])
            print(f"✅ 标签查询成功，找到 {len(records)} 条记录")
            for record in records:
                print(f"  - {record.get('name')}: {record.get('tags')}")
        else:
            print(f"❌ 标签查询失败: {search_data.get('error')}")

        print("\n🎉 数组字段测试完成")

    except Exception as e:
        print(f"❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

    finally:
        # 清理
        try:
            if os.path.exists("./test_arrays.db"):
                os.remove("./test_arrays.db")
                print("🧹 清理测试文件完成")
        except:
            pass

if __name__ == "__main__":
    test_array_fields()