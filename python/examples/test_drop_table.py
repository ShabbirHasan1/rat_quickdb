#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
简单的 drop_table 功能测试
"""

import json
import sys
import os

# 添加当前目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from rat_quickdb_py import create_db_queue_bridge

def test_drop_table():
    """测试 drop_table 功能"""
    print("=== 简单 drop_table 测试 ===")
    
    try:
        # 创建桥接器
        bridge = create_db_queue_bridge()
        print("✅ 桥接器创建成功")
        
        # 添加 SQLite 数据库
        db_result = bridge.add_sqlite_database(
            alias="test_db",
            path="./test_drop.db",
            max_connections=10,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=1800,
            cache_config=None
        )
        print(f"✅ SQLite 数据库添加成功: {db_result}")
        
        # 设置默认别名
        bridge.set_default_alias("test_db")
        print("✅ 默认别名设置成功")
        
        # 测试 drop_table
        print("\n🧹 测试 drop_table 功能...")
        drop_result = bridge.drop_table("test_table", "test_db")
        print(f"drop_table 返回结果: {drop_result}")
        print(f"返回结果类型: {type(drop_result)}")
        
        # 尝试解析 JSON 响应
        try:
            parsed_result = json.loads(drop_result)
            print(f"解析后的结果: {parsed_result}")
            print(f"成功状态: {parsed_result.get('success')}")
            print(f"数据: {parsed_result.get('data')}")
            print(f"错误: {parsed_result.get('error')}")
            
            if parsed_result.get('success'):
                print("✅ drop_table 执行成功")
            else:
                print(f"❌ drop_table 执行失败: {parsed_result.get('error')}")
                
        except json.JSONDecodeError as e:
            print(f"❌ JSON 解析失败: {e}")
            print(f"原始响应: '{drop_result}'")
            
    except Exception as e:
        print(f"❌ 测试失败: {e}")
        import traceback
        traceback.print_exc()
    
    finally:
        # 清理测试文件
        try:
            if os.path.exists("./test_drop.db"):
                os.remove("./test_drop.db")
                print("🧹 清理测试文件完成")
        except:
            pass

if __name__ == "__main__":
    test_drop_table()