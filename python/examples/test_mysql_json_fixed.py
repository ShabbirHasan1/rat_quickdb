#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
测试MySQL JSON字段问题是否已通过register_model解决
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def test_mysql_json_fixed():
    """测试MySQL JSON字段问题修复"""
    print("🚀 开始测试MySQL JSON字段问题修复")

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

        # 添加MySQL数据库
        result = bridge.add_mysql_database(
            alias="test_mysql_fixed",
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

        result_data = json.loads(result)
        if not result_data.get("success"):
            print(f"❌ MySQL数据库添加失败: {result_data.get('error')}")
            return

        print("✅ MySQL数据库添加成功")

        # 创建表名
        table_name = f"test_mysql_json_fixed_{int(time.time())}"

        # 创建字段定义
        id_field = rq.integer_field(
            True,           # required
            True,           # unique
            None,           # min_value
            None,           # max_value
            "主键ID"         # description
        )

        name_field = rq.string_field(
            True,           # required
            False,          # unique
            None,           # max_length
            None,           # min_length
            "名称字段"       # description
        )

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

        # 创建字段字典
        fields_dict = {
            "id": id_field,
            "name": name_field,
            "json_field": json_field
        }

        # 创建模型元数据
        model_meta = rq.ModelMeta(
            table_name,
            fields_dict,
            [index_def],
            "test_mysql_fixed",  # database_alias
            "MySQL JSON字段修复测试"  # description
        )

        print("✅ 模型元数据创建成功")

        # 注册模型
        print("📝 注册ODM模型...")
        register_result = bridge.register_model(model_meta)
        register_data = json.loads(register_result)

        if register_data.get("success"):
            print("✅ ODM模型注册成功")
        else:
            print(f"❌ ODM模型注册失败: {register_data.get('error')}")
            return

        # 测试数据
        test_data = {
            "name": "MySQL JSON修复测试",
            "json_field": {
                "profile": {
                    "name": "测试用户",
                    "settings": {
                        "theme": "dark",
                        "notifications": True
                    }
                },
                "metadata": {
                    "version": "1.0",
                    "tags": ["test", "mysql", "json"]
                }
            }
        }

        print(f"📝 插入测试数据到表 {table_name}...")
        insert_result = bridge.create(table_name, json.dumps(test_data), "test_mysql_fixed")
        insert_data = json.loads(insert_result)

        if insert_data.get("success"):
            print("✅ 数据插入成功")
            print(f"   返回的ID: {insert_data.get('data')}")
        else:
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return

        # 查询数据 - 查询所有记录
        print("🔍 查询数据...")
        query_result = bridge.find(table_name, '{}', "test_mysql_fixed")
        query_data = json.loads(query_result)

        if query_data.get("success"):
            records = query_data.get("data")
            if records and len(records) > 0:
                record = records[0]  # 取第一条记录
                print("✅ 数据查询成功")
                print(f"   记录类型: {type(record)}")

                # 检查JSON字段
                json_field_value = record.get('json_field')
                print(f"   json_field: {json_field_value}")
                print(f"   json_field类型: {type(json_field_value)}")

                if isinstance(json_field_value, dict):
                    print("✅ JSON字段正确解析为dict")

                    # 检查嵌套结构
                    profile = json_field_value.get('profile', {})
                    if isinstance(profile, dict):
                        print("✅ profile字段正确解析为dict")

                        settings = profile.get('settings', {})
                        if isinstance(settings, dict):
                            print("✅ settings字段正确解析为dict")
                            print(f"   theme: {settings.get('theme')}")
                            print(f"   notifications: {settings.get('notifications')}")
                        else:
                            print(f"❌ settings字段解析失败: {type(settings)}")
                    else:
                        print(f"❌ profile字段解析失败: {type(profile)}")

                    metadata = json_field_value.get('metadata', {})
                    if isinstance(metadata, dict):
                        print("✅ metadata字段正确解析为dict")
                        print(f"   version: {metadata.get('version')}")
                        print(f"   tags: {metadata.get('tags')}")
                    else:
                        print(f"❌ metadata字段解析失败: {type(metadata)}")

                else:
                    print(f"❌ JSON字段解析失败: {type(json_field_value)}")
                    if isinstance(json_field_value, str):
                        print("   这是一个JSON字符串，说明转换逻辑没有工作")
            else:
                print("❌ 查询结果为空")
        else:
            print(f"❌ 数据查询失败: {query_data.get('error')}")

        print("\n🎉 MySQL JSON字段问题修复测试完成")

    except Exception as e:
        print(f"❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

    finally:
        # 清理
        try:
            if 'bridge' in locals():
                drop_result = bridge.drop_table(table_name, "test_mysql_fixed")
                print(f"🧹 清理测试表: {json.loads(drop_result).get('success')}")
        except:
            pass

if __name__ == "__main__":
    test_mysql_json_fixed()