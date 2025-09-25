#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MySQL JSON字段调试测试
用于诊断MySQL Object字段解析失败的具体原因
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def debug_mysql_json():
    """调试MySQL JSON字段处理"""
    print("🚀 开始MySQL JSON字段调试测试")

    try:
        # 创建数据库桥接器
        bridge = rq.create_db_queue_bridge()
        print("✅ 桥接器创建成功")

        # 初始化日志以便查看详细信息
        try:
            rq.init_logging_with_level("debug")
            print("✅ 日志初始化成功")
        except:
            print("⚠️ 日志初始化失败")

        # 添加MySQL数据库
        result = bridge.add_mysql_database(
            alias="debug_mysql",
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

        # 创建测试表
        table_name = f"debug_mysql_json_{int(time.time())}"

        # 创建简单的测试数据
        test_data = {
            "id": 1,
            "name": "调试测试",
            "simple_obj": {"key": "value", "number": 42},
            "metadata": {
                "profile": {
                    "name": "测试用户",
                    "settings": {"theme": "dark"}
                }
            }
        }

        print(f"📝 插入测试数据到表 {table_name}...")
        print(f"   原始数据: {test_data}")

        # 插入数据
        insert_result = bridge.create(table_name, json.dumps(test_data), "debug_mysql")
        insert_data = json.loads(insert_result)

        if insert_data.get("success"):
            print("✅ 数据插入成功")
            print(f"   返回的ID: {insert_data.get('data')}")
        else:
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return

        # 查询数据
        print("🔍 查询数据...")
        query_result = bridge.find_by_id(table_name, "1", "debug_mysql")
        query_data = json.loads(query_result)

        if query_data.get("success"):
            record = query_data.get("data")
            if record:
                print("✅ 数据查询成功")
                print(f"   记录类型: {type(record)}")
                print(f"   完整记录: {record}")

                # 检查每个字段
                for field_name, field_value in record.items():
                    print(f"   字段 '{field_name}': {field_value} (类型: {type(field_value)})")

                    # 如果是字符串，检查是否是JSON格式
                    if isinstance(field_value, str):
                        if field_value.startswith('{') or field_value.startswith('['):
                            print(f"     ⚠️ 这个字段是JSON字符串但未被解析!")
                            try:
                                parsed = json.loads(field_value)
                                print(f"     解析后: {parsed} (类型: {type(parsed)})")
                            except json.JSONDecodeError as e:
                                print(f"     JSON解析失败: {e}")
                        else:
                            print(f"     ✅ 普通字符串")
                    elif isinstance(field_value, dict):
                        print(f"     ✅ 正确解析为字典")
                    else:
                        print(f"     ✅ 其他类型")

            else:
                print("❌ 查询结果为空")
        else:
            print(f"❌ 数据查询失败: {query_data.get('error')}")

        print("\n🎉 MySQL JSON字段调试测试完成")

    except Exception as e:
        print(f"❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

    finally:
        # 清理
        try:
            if 'bridge' in locals():
                drop_result = bridge.drop_table(table_name, "debug_mysql")
                print(f"🧹 清理测试表: {json.loads(drop_result).get('success')}")
        except:
            pass

if __name__ == "__main__":
    debug_mysql_json()