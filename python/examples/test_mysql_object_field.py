#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MySQL Object字段简单测试
验证MySQL中JSON字段的存储和查询功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json

def test_mysql_object_field():
    """测试MySQL Object字段功能"""
    print("🚀 开始MySQL Object字段测试")

    try:
        # 创建数据库桥接器
        bridge = rq.create_db_queue_bridge()
        print("✅ 桥接器创建成功")

        # 添加MySQL数据库
        result = bridge.add_mysql_database(
            alias="test_mysql",
            host="localhost",
            port=3306,
            database="testdb",
            username="testuser",
            password="testpass",
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )

        result_data = json.loads(result)
        if not result_data.get("success"):
            print(f"❌ MySQL数据库添加失败: {result_data.get('error')}")
            print("注意：如果MySQL服务不可用，这是正常的")
            return

        print("✅ MySQL数据库添加成功")

        # 创建测试表
        table_name = "test_mysql_object"

        # 清理已存在的表
        try:
            drop_result = bridge.drop_table(table_name, "test_mysql")
            print(f"🧹 清理已存在的表: {json.loads(drop_result).get('success')}")
        except:
            pass

        # 创建测试数据
        test_data = {
            "id": "mysql_test_001",
            "name": "MySQL Object字段测试",
            "metadata": {
                "level": "advanced",
                "topics": ["mysql", "json", "objects"],
                "profile": {
                    "theme": "dark",
                    "preferences": {
                        "email": True,
                        "sms": False
                    }
                }
            },
            "config": {
                "database_type": "mysql",
                "settings": {
                    "max_connections": 10,
                    "timeout": 30
                }
            }
        }

        # 插入数据
        print("📝 插入测试数据...")
        insert_result = bridge.create(table_name, json.dumps(test_data), "test_mysql")
        insert_data = json.loads(insert_result)

        if insert_data.get("success"):
            print("✅ 数据插入成功")
            print(f"  - 记录ID: {insert_data.get('data')}")
        else:
            print(f"❌ 数据插入失败: {insert_data.get('error')}")
            return

        # 查询数据
        print("🔍 查询数据...")
        query_result = bridge.find_by_id(table_name, "mysql_test_001", "test_mysql")
        query_data = json.loads(query_result)

        if query_data.get("success"):
            record = query_data.get("data")
            if record:
                print("✅ 数据查询成功")
                print(f"  - 记录类型: {type(record)}")
                print(f"  - metadata字段: {record.get('metadata')} (类型: {type(record.get('metadata'))})")
                print(f"  - config字段: {record.get('config')} (类型: {type(record.get('config'))})")

                # 验证Object字段
                metadata = record.get('metadata')
                config = record.get('config')

                if isinstance(metadata, dict):
                    print("✅ metadata字段正确解析为dict")

                    # 检查嵌套字段
                    if isinstance(metadata.get('profile'), dict):
                        print("✅ metadata.profile字段正确解析为dict")

                        profile = metadata['profile']
                        if isinstance(profile.get('preferences'), dict):
                            print("✅ metadata.profile.preferences字段正确解析为dict")
                        else:
                            print(f"❌ metadata.profile.preferences字段解析失败: {type(profile.get('preferences'))}")
                    else:
                        print(f"❌ metadata.profile字段解析失败: {type(metadata.get('profile'))}")
                else:
                    print(f"❌ metadata字段解析失败: {type(metadata)}")

                if isinstance(config, dict):
                    print("✅ config字段正确解析为dict")

                    if isinstance(config.get('settings'), dict):
                        print("✅ config.settings字段正确解析为dict")
                    else:
                        print(f"❌ config.settings字段解析失败: {type(config.get('settings'))}")
                else:
                    print(f"❌ config字段解析失败: {type(config)}")

                # 显示完整的嵌套结构
                print("\n📋 完整的嵌套结构:")
                print(f"  metadata.profile.preferences: {metadata.get('profile', {}).get('preferences')}")
                print(f"  config.settings: {config.get('settings')}")

            else:
                print("❌ 查询结果为空")
        else:
            print(f"❌ 数据查询失败: {query_data.get('error')}")

        print("\n🎉 MySQL Object字段测试完成")

    except Exception as e:
        print(f"❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

    finally:
        # 清理
        try:
            if 'bridge' in locals():
                drop_result = bridge.drop_table(table_name, "test_mysql")
                print(f"🧹 清理测试表: {json.loads(drop_result).get('success')}")
        except:
            pass

if __name__ == "__main__":
    test_mysql_object_field()