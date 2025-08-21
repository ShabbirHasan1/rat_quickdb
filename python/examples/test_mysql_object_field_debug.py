#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MySQL Object字段调试测试脚本

专门用于调试MySQL数据库中Object字段解析问题
添加详细的调试日志来排查MySQL适配器层的问题
"""

import json
import os
import sys
import time
from typing import Dict, Any

try:
    import rat_quickdb_py
    from rat_quickdb_py import create_db_queue_bridge
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    sys.exit(1)


def test_mysql_object_field_debug(bridge, table_name: str, db_alias: str) -> bool:
    """
    调试MySQL数据库的Object字段解析问题
    
    Args:
        bridge: 数据库桥接器
        table_name: 表名
        db_alias: 数据库别名
    
    Returns:
        bool: 测试是否通过
    """
    print(f"\n🔍 开始调试 MySQL 数据库的 Object 字段解析问题...")
    print(f"📝 表名: {table_name}")
    print(f"🏷️ 数据库别名: {db_alias}")
    
    try:
        # 测试数据 - 包含复杂嵌套的Object字段
        test_id = 1001  # MySQL需要数字类型的ID用于AUTO_INCREMENT
        
        test_data = {
            "id": test_id,
            "name": "MySQL测试用户",
            "metadata": {
                "profile": {
                    "age": 25,
                    "city": "北京",
                    "preferences": {
                        "theme": "dark",
                        "language": "zh-CN",
                        "notifications": {
                            "email": True,
                            "sms": False,
                            "push": True
                        }
                    }
                },
                "settings": {
                    "privacy": "public",
                    "features": ["feature1", "feature2"],
                    "limits": {
                        "daily_quota": 1000,
                        "monthly_quota": 30000
                    }
                }
            },
            "tags": ["user", "test", "mysql"],
            "config": {
                "database_type": "MySQL",
                "test_timestamp": time.time(),
                "nested_arrays": [
                    {"item": "array_item_1", "value": 100},
                    {"item": "array_item_2", "value": 200}
                ]
            }
        }
        
        print(f"\n📊 测试数据结构:")
        print(f"  - ID: {test_data['id']} (类型: {type(test_data['id'])})")
        print(f"  - metadata: {type(test_data['metadata'])} (嵌套层级: 4)")
        print(f"  - config: {type(test_data['config'])} (包含数组对象)")
        print(f"  - tags: {type(test_data['tags'])} (简单数组)")
        
        # 插入测试数据
        print(f"\n📝 插入测试数据到 MySQL...")
        print(f"  🔄 调用 bridge.create('{table_name}', data, '{db_alias}')")
        
        create_result = bridge.create(table_name, json.dumps(test_data), db_alias)
        create_response = json.loads(create_result)
        
        print(f"  📤 插入结果: {create_response}")
        
        if not create_response.get("success"):
            print(f"  ❌ 插入数据失败: {create_response.get('error')}")
            return False
        
        print(f"  ✅ 数据插入成功")
        
        # 查询数据并验证Object字段
        print(f"\n🔍 查询数据并验证 Object 字段...")
        
        # 构建查询条件
        query_conditions = json.dumps([
            {"field": "id", "operator": "Eq", "value": test_data["id"]}
        ])
        
        print(f"  🔍 查询条件: {query_conditions}")
        print(f"  🔄 调用 bridge.find('{table_name}', conditions, '{db_alias}')")
        
        find_result = bridge.find(table_name, query_conditions, db_alias)
        find_response = json.loads(find_result)
        
        print(f"  📥 查询结果: {find_response}")
        
        if not find_response.get("success"):
            print(f"  ❌ 查询数据失败: {find_response.get('error')}")
            return False
        
        # 验证查询结果
        data = find_response.get("data", [])
        if not data:
            print(f"  ❌ 未找到查询结果")
            return False
        
        record = data[0]
        print(f"\n📊 查询到记录详情:")
        print(f"  - 记录类型: {type(record)}")
        print(f"  - 记录字段数: {len(record) if isinstance(record, dict) else 'N/A'}")
        
        if isinstance(record, dict):
            for key, value in record.items():
                print(f"  - {key}: {type(value)} = {value if not isinstance(value, (dict, list)) or len(str(value)) < 100 else str(value)[:100] + '...'}")
        
        # 详细验证Object字段
        success = True
        
        print(f"\n🔬 详细验证 Object 字段类型...")
        
        # 检查metadata字段
        if "metadata" in record:
            metadata = record["metadata"]
            print(f"\n  📋 metadata 字段分析:")
            print(f"    - 类型: {type(metadata)}")
            print(f"    - 值: {metadata}")
            print(f"    - 是否为dict: {isinstance(metadata, dict)}")
            
            if isinstance(metadata, dict):
                print(f"    ✅ metadata 字段正确解析为 dict")
                
                # 检查嵌套的profile字段
                if "profile" in metadata:
                    profile = metadata["profile"]
                    print(f"\n    📋 metadata.profile 字段分析:")
                    print(f"      - 类型: {type(profile)}")
                    print(f"      - 值: {profile}")
                    print(f"      - 是否为dict: {isinstance(profile, dict)}")
                    
                    if isinstance(profile, dict):
                        print(f"      ✅ metadata.profile 字段正确解析为 dict")
                        
                        # 检查深层嵌套的preferences字段
                        if "preferences" in profile:
                            preferences = profile["preferences"]
                            print(f"\n      📋 metadata.profile.preferences 字段分析:")
                            print(f"        - 类型: {type(preferences)}")
                            print(f"        - 值: {preferences}")
                            print(f"        - 是否为dict: {isinstance(preferences, dict)}")
                            
                            if isinstance(preferences, dict):
                                print(f"        ✅ metadata.profile.preferences 字段正确解析为 dict")
                                
                                # 检查最深层的notifications字段
                                if "notifications" in preferences:
                                    notifications = preferences["notifications"]
                                    print(f"\n        📋 metadata.profile.preferences.notifications 字段分析:")
                                    print(f"          - 类型: {type(notifications)}")
                                    print(f"          - 值: {notifications}")
                                    print(f"          - 是否为dict: {isinstance(notifications, dict)}")
                                    
                                    if isinstance(notifications, dict):
                                        print(f"          ✅ metadata.profile.preferences.notifications 字段正确解析为 dict")
                                    else:
                                        print(f"          ❌ metadata.profile.preferences.notifications 字段未正确解析")
                                        success = False
                                else:
                                    print(f"        ❌ 未找到 notifications 字段")
                                    success = False
                            else:
                                print(f"        ❌ metadata.profile.preferences 字段未正确解析")
                                success = False
                        else:
                            print(f"      ❌ 未找到 preferences 字段")
                            success = False
                    else:
                        print(f"      ❌ metadata.profile 字段未正确解析")
                        success = False
                else:
                    print(f"    ❌ 未找到 profile 字段")
                    success = False
                    
                # 检查settings字段
                if "settings" in metadata:
                    settings = metadata["settings"]
                    print(f"\n    📋 metadata.settings 字段分析:")
                    print(f"      - 类型: {type(settings)}")
                    print(f"      - 值: {settings}")
                    print(f"      - 是否为dict: {isinstance(settings, dict)}")
                    
                    if isinstance(settings, dict):
                        print(f"      ✅ metadata.settings 字段正确解析为 dict")
                        
                        if "limits" in settings:
                            limits = settings["limits"]
                            print(f"\n      📋 metadata.settings.limits 字段分析:")
                            print(f"        - 类型: {type(limits)}")
                            print(f"        - 值: {limits}")
                            print(f"        - 是否为dict: {isinstance(limits, dict)}")
                            
                            if isinstance(limits, dict):
                                print(f"        ✅ metadata.settings.limits 字段正确解析为 dict")
                            else:
                                print(f"        ❌ metadata.settings.limits 字段未正确解析")
                                success = False
                        else:
                            print(f"      ❌ 未找到 limits 字段")
                            success = False
                    else:
                        print(f"      ❌ metadata.settings 字段未正确解析")
                        success = False
                else:
                    print(f"    ❌ 未找到 settings 字段")
                    success = False
            else:
                print(f"    ❌ metadata 字段未正确解析为 dict")
                success = False
        else:
            print(f"  ❌ 未找到 metadata 字段")
            success = False
        
        # 检查config字段
        if "config" in record:
            config = record["config"]
            print(f"\n  📋 config 字段分析:")
            print(f"    - 类型: {type(config)}")
            print(f"    - 值: {config}")
            print(f"    - 是否为dict: {isinstance(config, dict)}")
            
            if isinstance(config, dict):
                print(f"    ✅ config 字段正确解析为 dict")
                
                # 检查数组中的对象
                if "nested_arrays" in config:
                    nested_arrays = config["nested_arrays"]
                    print(f"\n    📋 config.nested_arrays 字段分析:")
                    print(f"      - 类型: {type(nested_arrays)}")
                    print(f"      - 值: {nested_arrays}")
                    print(f"      - 是否为list: {isinstance(nested_arrays, list)}")
                    
                    if isinstance(nested_arrays, list) and nested_arrays:
                        first_item = nested_arrays[0]
                        print(f"\n      📋 config.nested_arrays[0] 字段分析:")
                        print(f"        - 类型: {type(first_item)}")
                        print(f"        - 值: {first_item}")
                        print(f"        - 是否为dict: {isinstance(first_item, dict)}")
                        
                        if isinstance(first_item, dict):
                            print(f"        ✅ config.nested_arrays 中的对象正确解析为 dict")
                        else:
                            print(f"        ❌ config.nested_arrays 中的对象未正确解析")
                            success = False
                    else:
                        print(f"      ❌ config.nested_arrays 不是有效的数组")
                        success = False
                else:
                    print(f"    ❌ 未找到 nested_arrays 字段")
                    success = False
            else:
                print(f"    ❌ config 字段未正确解析为 dict")
                success = False
        else:
            print(f"  ❌ 未找到 config 字段")
            success = False
        
        # 输出最终结果
        print(f"\n" + "=" * 60)
        if success:
            print(f"🎉 MySQL 数据库 Object 字段解析测试通过！")
            print(f"✅ 所有 Object 字段都正确解析为 Python 字典类型")
        else:
            print(f"💥 MySQL 数据库 Object 字段解析测试失败！")
            print(f"❌ 部分 Object 字段未正确解析为 Python 字典类型")
            print(f"🔧 建议检查 MySQL 适配器层的 JSON 字段处理逻辑")
        print(f"=" * 60)
        
        return success
        
    except Exception as e:
        print(f"  ❌ MySQL 测试过程中出现异常: {e}")
        import traceback
        print(f"  📋 异常详情: {traceback.format_exc()}")
        return False


def main():
    """
    主函数 - 专门测试MySQL数据库的Object字段解析问题
    """
    print("🚀 开始调试 MySQL Object 字段解析问题")
    print("=" * 60)
    
    # 创建数据库桥接器
    bridge = create_db_queue_bridge()
    
    # 使用时间戳作为表名后缀，避免冲突
    timestamp = int(time.time() * 1000)
    table_name = f"mysql_object_debug_{timestamp}"
    
    print(f"📝 使用表名: {table_name}")
    
    try:
        # MySQL 配置（使用示例文件中的正确配置）
        print("\n🔧 配置 MySQL 数据库...")
        mysql_result = bridge.add_mysql_database(
            alias="debug_mysql",
            host="172.16.0.21",
            port=3306,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            max_connections=10,
            min_connections=2,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )
        print(f"MySQL 配置结果: {mysql_result}")
        
        # 执行调试测试
        success = test_mysql_object_field_debug(bridge, table_name, "debug_mysql")
        
        # 清理测试数据
        print(f"\n🧹 清理测试数据...")
        try:
            bridge.drop_table(table_name, "debug_mysql")
            print(f"  ✅ 已清理表 {table_name}")
        except Exception as e:
            print(f"  ⚠️ 清理表 {table_name} 时出错: {e}")
        
        return success
        
    except Exception as e:
        print(f"❌ 测试过程中出现异常: {e}")
        import traceback
        print(f"📋 异常详情: {traceback.format_exc()}")
        return False


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)