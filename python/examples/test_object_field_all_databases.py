#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Object字段修复功能测试脚本 - 所有数据库类型

本脚本测试Object字段修复功能在所有支持的数据库类型上的表现：
- SQLite
- PostgreSQL  
- MySQL
- MongoDB

验证Object字段被正确解析为Python字典而非包装类型
"""

import json
import os
import sys
import time
from typing import Dict, Any, List

try:
    import rat_quickdb_py
    from rat_quickdb_py import create_db_queue_bridge
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    sys.exit(1)


def test_object_field_for_database(bridge, table_name: str, db_alias: str, db_type: str) -> bool:
    """
    测试指定数据库的Object字段修复功能
    
    Args:
        bridge: 数据库桥接器
        table_name: 表名
        db_alias: 数据库别名
        db_type: 数据库类型
    
    Returns:
        bool: 测试是否通过
    """
    print(f"\n🔍 测试 {db_type} 数据库的 Object 字段修复功能...")
    
    try:
        # 测试数据 - 包含复杂嵌套的Object字段
        # 为MySQL使用数字ID，其他数据库使用字符串ID
        if db_type.lower() == "mysql":
            test_id = 1001  # MySQL需要数字类型的ID用于AUTO_INCREMENT
        else:
            test_id = f"test_{db_type.lower()}_001"
        
        test_data = {
            "id": test_id,
            "name": f"{db_type}测试用户",
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
            "tags": ["user", "test", db_type.lower()],
            "config": {
                "database_type": db_type,
                "test_timestamp": time.time(),
                "nested_arrays": [
                    {"item": "array_item_1", "value": 100},
                    {"item": "array_item_2", "value": 200}
                ]
            }
        }
        
        # 插入测试数据
        print(f"  📝 插入测试数据到 {db_type}...")
        create_result = bridge.create(table_name, json.dumps(test_data), db_alias)
        create_response = json.loads(create_result)
        
        if not create_response.get("success"):
            print(f"  ❌ 插入数据失败: {create_response.get('error')}")
            return False
        
        print(f"  ✅ 数据插入成功")
        
        # 查询数据并验证Object字段
        print(f"  🔍 查询数据并验证 Object 字段...")
        
        # 构建查询条件
        if db_type == "MongoDB":
            query_conditions = json.dumps({"id": test_data["id"]})
        else:
            query_conditions = json.dumps([
                {"field": "id", "operator": "Eq", "value": test_data["id"]}
            ])
        
        find_result = bridge.find(table_name, query_conditions, db_alias)
        find_response = json.loads(find_result)
        
        if not find_response.get("success"):
            print(f"  ❌ 查询数据失败: {find_response.get('error')}")
            return False
        
        # 验证查询结果
        data = find_response.get("data", [])
        if not data:
            print(f"  ❌ 未找到查询结果")
            return False
        
        record = data[0]
        print(f"  📊 查询到记录: {type(record)}")
        
        # 验证Object字段是否为Python字典
        success = True
        
        # 检查metadata字段
        if "metadata" in record:
            metadata = record["metadata"]
            if isinstance(metadata, dict):
                print(f"  ✅ metadata 字段正确解析为 dict: {type(metadata)}")
                
                # 检查嵌套的profile字段
                if "profile" in metadata and isinstance(metadata["profile"], dict):
                    print(f"  ✅ metadata.profile 字段正确解析为 dict")
                    
                    # 检查深层嵌套的preferences字段
                    profile = metadata["profile"]
                    if "preferences" in profile and isinstance(profile["preferences"], dict):
                        print(f"  ✅ metadata.profile.preferences 字段正确解析为 dict")
                        
                        # 检查最深层的notifications字段
                        preferences = profile["preferences"]
                        if "notifications" in preferences and isinstance(preferences["notifications"], dict):
                            print(f"  ✅ metadata.profile.preferences.notifications 字段正确解析为 dict")
                        else:
                            print(f"  ❌ metadata.profile.preferences.notifications 字段未正确解析: {type(preferences.get('notifications'))}")
                            success = False
                    else:
                        print(f"  ❌ metadata.profile.preferences 字段未正确解析: {type(profile.get('preferences'))}")
                        success = False
                else:
                    print(f"  ❌ metadata.profile 字段未正确解析: {type(metadata.get('profile'))}")
                    success = False
                    
                # 检查settings字段
                if "settings" in metadata and isinstance(metadata["settings"], dict):
                    print(f"  ✅ metadata.settings 字段正确解析为 dict")
                    
                    settings = metadata["settings"]
                    if "limits" in settings and isinstance(settings["limits"], dict):
                        print(f"  ✅ metadata.settings.limits 字段正确解析为 dict")
                    else:
                        print(f"  ❌ metadata.settings.limits 字段未正确解析: {type(settings.get('limits'))}")
                        success = False
                else:
                    print(f"  ❌ metadata.settings 字段未正确解析: {type(metadata.get('settings'))}")
                    success = False
            else:
                print(f"  ❌ metadata 字段未正确解析为 dict: {type(metadata)}")
                # 添加调试信息
                if db_type.lower() == "mysql":
                    print(f"  🔍 MySQL metadata 字段调试信息:")
                    print(f"    原始值: {metadata}")
                    print(f"    类型: {type(metadata)}")
                    print(f"    是否字符串: {isinstance(metadata, str)}")
                    if isinstance(metadata, str):
                        print(f"    字符串长度: {len(metadata)}")
                        print(f"    前100字符: {metadata[:100]}")
                success = False
        else:
            print(f"  ❌ 未找到 metadata 字段")
            success = False
        
        # 检查config字段
        if "config" in record:
            config = record["config"]
            if isinstance(config, dict):
                print(f"  ✅ config 字段正确解析为 dict: {type(config)}")
                
                # 检查数组中的对象
                if "nested_arrays" in config and isinstance(config["nested_arrays"], list):
                    nested_arrays = config["nested_arrays"]
                    if nested_arrays and isinstance(nested_arrays[0], dict):
                        print(f"  ✅ config.nested_arrays 中的对象正确解析为 dict")
                    else:
                        print(f"  ❌ config.nested_arrays 中的对象未正确解析: {type(nested_arrays[0]) if nested_arrays else 'empty'}")
                        success = False
                else:
                    print(f"  ❌ config.nested_arrays 字段未正确解析: {type(config.get('nested_arrays'))}")
                    success = False
            else:
                print(f"  ❌ config 字段未正确解析为 dict: {type(config)}")
                success = False
        else:
            print(f"  ❌ 未找到 config 字段")
            success = False
        
        if success:
            print(f"  🎉 {db_type} 数据库 Object 字段修复功能测试通过！")
        else:
            print(f"  💥 {db_type} 数据库 Object 字段修复功能测试失败！")
        
        return success
        
    except Exception as e:
        print(f"  ❌ {db_type} 数据库测试过程中出现异常: {e}")
        return False


def cleanup_test_data(bridge, table_name: str, db_aliases: List[str]):
    """
    清理测试数据
    
    Args:
        bridge: 数据库桥接器
        table_name: 表名
        db_aliases: 数据库别名列表
    """
    print(f"\n🧹 清理测试数据...")
    
    for alias in db_aliases:
        try:
            bridge.drop_table(table_name, alias)
            print(f"  ✅ 已清理 {alias} 中的表 {table_name}")
        except Exception as e:
            print(f"  ⚠️ 清理 {alias} 中的表 {table_name} 时出错: {e}")


def main():
    """
    主函数 - 测试所有数据库类型的Object字段修复功能
    """
    print("🚀 开始测试 Object 字段修复功能 - 所有数据库类型")
    print("=" * 60)
    
    # 创建数据库桥接器
    bridge = create_db_queue_bridge()
    
    # 使用时间戳作为表名后缀，避免冲突
    timestamp = int(time.time() * 1000)
    table_name = f"object_field_test_{timestamp}"
    
    print(f"📝 使用表名: {table_name}")
    
    try:
        # SQLite 配置（本地文件）
        print("\n🔧 配置 SQLite 数据库...")
        sqlite_result = bridge.add_sqlite_database(
            alias="test_sqlite",
            path=f"./test_object_fields_{timestamp}.db",
            max_connections=10,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )
        print(f"SQLite 配置结果: {sqlite_result}")
        
        # PostgreSQL 配置（使用示例文件中的正确配置）
        print("\n🔧 配置 PostgreSQL 数据库...")
        postgres_result = bridge.add_postgresql_database(
            alias="test_postgres",
            host="172.16.0.23",
            port=5432,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            max_connections=10,
            min_connections=2,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600,
            ssl_mode="prefer"
        )
        print(f"PostgreSQL 配置结果: {postgres_result}")
        
        # MySQL 配置（使用示例文件中的正确配置）
        print("\n🔧 配置 MySQL 数据库...")
        mysql_result = bridge.add_mysql_database(
            alias="test_mysql",
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
        
        # MongoDB 配置（使用示例文件中的正确配置，包含TLS和ZSTD）
        print("\n🔧 配置 MongoDB 数据库...")
        
        # 创建TLS配置
        try:
            from rat_quickdb_py import PyTlsConfig
            tls_config = PyTlsConfig()
            tls_config.enable()
            tls_config.ca_cert_path = "/etc/ssl/certs/ca-certificates.crt"
            tls_config.client_cert_path = ""
            tls_config.client_key_path = ""
            print("  🔒 TLS配置创建成功")
        except Exception as e:
            print(f"  ⚠️ TLS配置创建失败: {e}")
            tls_config = None
        
        # 创建ZSTD配置
        try:
            from rat_quickdb_py import PyZstdConfig
            zstd_config = PyZstdConfig()
            zstd_config.enable()
            zstd_config.compression_level = 3
            zstd_config.compression_threshold = 1024
            print("  🗜️ ZSTD压缩配置创建成功")
        except Exception as e:
            print(f"  ⚠️ ZSTD配置创建失败: {e}")
            zstd_config = None
        
        mongodb_result = bridge.add_mongodb_database(
            alias="test_mongodb",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=8,
            min_connections=2,
            connection_timeout=5,
            idle_timeout=60,
            max_lifetime=300,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        print(f"MongoDB 配置结果: {mongodb_result}")
        
        # 测试配置
        test_configs = [
            ("test_sqlite", "SQLite"),
            ("test_postgres", "PostgreSQL"),
            ("test_mysql", "MySQL"),
            ("test_mongodb", "MongoDB")
        ]
        
        # 执行测试
        results = {}
        for db_alias, db_type in test_configs:
            try:
                success = test_object_field_for_database(bridge, table_name, db_alias, db_type)
                results[db_type] = success
            except Exception as e:
                print(f"❌ {db_type} 测试失败: {e}")
                results[db_type] = False
        
        # 输出测试结果汇总
        print("\n" + "=" * 60)
        print("📊 测试结果汇总")
        print("=" * 60)
        
        all_passed = True
        for db_type, success in results.items():
            status = "✅ 通过" if success else "❌ 失败"
            print(f"  {db_type:12} : {status}")
            if not success:
                all_passed = False
        
        print("\n" + "=" * 60)
        if all_passed:
            print("🎉 所有数据库的 Object 字段修复功能测试均通过！")
            print("✅ Object 字段现在能够正确解析为 Python 字典类型")
        else:
            print("💥 部分数据库的 Object 字段修复功能测试失败")
            print("⚠️ 请检查失败的数据库配置和实现")
        
        # 清理测试数据
        cleanup_test_data(bridge, table_name, [alias for alias, _ in test_configs])
        
        # 清理SQLite文件
        sqlite_file = f"./test_object_fields_{timestamp}.db"
        if os.path.exists(sqlite_file):
            os.remove(sqlite_file)
            print(f"  ✅ 已清理 SQLite 文件: {sqlite_file}")
        
        return all_passed
        
    except Exception as e:
        print(f"❌ 测试过程中出现异常: {e}")
        return False


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)