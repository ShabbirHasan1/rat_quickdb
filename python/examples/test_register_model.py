#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
简单测试register_model功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json

def test_register_model():
    """测试register_model功能"""
    print("🚀 开始测试register_model功能")

    try:
        # 创建数据库桥接器
        bridge = rq.create_db_queue_bridge()
        print("✅ 桥接器创建成功")

        # 初始化日志
        try:
            rq.init_logging_with_level("debug")
            print("✅ 日志初始化成功")
        except:
            print("⚠️ 日志初始化失败")

        # 添加SQLite数据库（简单测试）
        result = bridge.add_sqlite_database(
            alias="test_sqlite",
            path=":memory:",
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )

        result_data = json.loads(result)
        if not result_data.get("success"):
            print(f"❌ SQLite数据库添加失败: {result_data.get('error')}")
            return

        print("✅ SQLite数据库添加成功")

        # 创建简单的字段定义
        # 注意：这里使用位置参数而不是关键字参数
        print("📝 创建字段定义...")

        # 创建字符串字段 (required, unique, max_length, min_length, description)
        id_field = rq.string_field(
            True,           # required
            True,           # unique
            None,           # max_length
            None,           # min_length
            "主键ID"         # description
        )

        name_field = rq.string_field(
            True,           # required
            False,          # unique
            None,           # max_length
            None,           # min_length
            "名称字段"       # description
        )

        # 创建JSON字段
        json_field = rq.json_field(
            False,          # required
            "JSON字段"      # description
        )

        # 创建索引定义
        index_def = rq.IndexDefinition(
            ["id"],         # fields
            True,           # unique
            "idx_id"        # name
        )

        # 创建模型元数据
        table_name = "test_model_register"

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_field": json_field
        }

        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "test_sqlite",  # database_alias
            "测试模型注册"   # description
        )

        print("✅ 模型元数据创建成功")

        # 注册模型
        print("📝 注册ODM模型...")
        register_result = bridge.register_model(model_meta)
        register_data = json.loads(register_result)

        if register_data.get("success"):
            print("✅ ODM模型注册成功")
            print(f"   消息: {register_data.get('message')}")
        else:
            print(f"❌ ODM模型注册失败: {register_data.get('error')}")
            return

        # 测试数据插入
        test_data = {
            "id": "test_001",
            "name": "模型注册测试",
            "json_field": {"key": "value", "number": 42}
        }

        print(f"📝 插入测试数据到表 {table_name}...")
        insert_result = bridge.create(table_name, json.dumps(test_data), "test_sqlite")
        insert_data = json.loads(insert_result)

        if insert_data.get("success"):
            print("✅ 数据插入成功")
            print(f"   返回的ID: {insert_data.get('data')}")
        else:
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return

        # 查询数据
        print("🔍 查询数据...")
        query_result = bridge.find_by_id(table_name, "test_001", "test_sqlite")
        query_data = json.loads(query_result)

        if query_data.get("success"):
            record = query_data.get("data")
            if record:
                print("✅ 数据查询成功")
                print(f"   记录类型: {type(record)}")
                print(f"   完整记录: {record}")

                # 检查JSON字段是否正确解析
                json_field_value = record.get('json_field')
                if isinstance(json_field_value, dict):
                    print("✅ JSON字段正确解析为dict")
                    print(f"   json_field: {json_field_value}")
                else:
                    print(f"❌ JSON字段解析失败: {type(json_field_value)}")
                    print(f"   值: {json_field_value}")
            else:
                print("❌ 查询结果为空")
        else:
            print(f"❌ 数据查询失败: {query_data.get('error')}")

        print("\n🎉 register_model功能测试完成")

    except Exception as e:
        print(f"❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_register_model()