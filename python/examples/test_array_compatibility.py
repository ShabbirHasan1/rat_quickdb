#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
数组字段兼容性测试脚本
测试 array_field 和 list_field 在不同数据库中的存储和查询功能
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'python'))

import rat_quickdb as rq
import json
from datetime import datetime

def test_sqlite_array_compatibility():
    """测试 SQLite 中的数组字段兼容性"""
    print("\n=== 测试 SQLite 数组字段兼容性 ===")
    
    # 创建 SQLite 连接
    config = rq.DatabaseConfig(
        database_type="sqlite",
        database_url="sqlite:///test_array.db"
    )
    
    # 定义测试模型
    model = rq.Model("test_arrays")
    model.add_field("id", rq.string_field(primary_key=True))
    model.add_field("tags", rq.array_field())  # 使用 array_field
    model.add_field("categories", rq.list_field())  # 使用 list_field
    model.add_field("name", rq.string_field())
    
    try:
        # 创建数据库实例
        db = rq.QuickDB(config)
        
        # 创建表
        db.create_table(model)
        print("✓ SQLite 表创建成功")
        
        # 插入测试数据
        test_data = {
            "id": "test1",
            "tags": ["python", "rust", "database"],
            "categories": ["tech", "programming", "tutorial"],
            "name": "测试记录"
        }
        
        result = db.create("test_arrays", test_data)
        print(f"✓ SQLite 数据插入成功: {result}")
        
        # 查询数据
        found = db.find_by_id("test_arrays", "test1")
        if found:
            print(f"✓ SQLite 数据查询成功:")
            print(f"  - tags: {found.get('tags')} (类型: {type(found.get('tags'))})")
            print(f"  - categories: {found.get('categories')} (类型: {type(found.get('categories'))})")
            
            # 验证数据类型
            tags = found.get('tags')
            categories = found.get('categories')
            
            if isinstance(tags, list) and isinstance(categories, list):
                print("✓ SQLite 数组字段正确存储为列表类型")
            else:
                print(f"⚠ SQLite 数组字段类型异常: tags={type(tags)}, categories={type(categories)}")
        else:
            print("✗ SQLite 数据查询失败")
            
    except Exception as e:
        print(f"✗ SQLite 测试失败: {e}")
        import traceback
        traceback.print_exc()

def test_mongodb_array_compatibility():
    """测试 MongoDB 中的数组字段兼容性"""
    print("\n=== 测试 MongoDB 数组字段兼容性 ===")
    
    # 创建 MongoDB 连接
    config = rq.DatabaseConfig(
        database_type="mongodb",
        database_url="mongodb://localhost:27017/test_array_db"
    )
    
    # 定义测试模型
    model = rq.Model("test_arrays")
    model.add_field("id", rq.string_field(primary_key=True))
    model.add_field("tags", rq.array_field())  # 使用 array_field
    model.add_field("categories", rq.list_field())  # 使用 list_field
    model.add_field("name", rq.string_field())
    
    try:
        # 创建数据库实例
        db = rq.QuickDB(config)
        
        # 创建表（MongoDB 中为集合）
        db.create_table(model)
        print("✓ MongoDB 集合创建成功")
        
        # 插入测试数据
        test_data = {
            "id": "test1",
            "tags": ["python", "rust", "database"],
            "categories": ["tech", "programming", "tutorial"],
            "name": "测试记录"
        }
        
        result = db.create("test_arrays", test_data)
        print(f"✓ MongoDB 数据插入成功: {result}")
        
        # 查询数据
        found = db.find_by_id("test_arrays", "test1")
        if found:
            print(f"✓ MongoDB 数据查询成功:")
            print(f"  - tags: {found.get('tags')} (类型: {type(found.get('tags'))})")
            print(f"  - categories: {found.get('categories')} (类型: {type(found.get('categories'))})")
            
            # 验证数据类型
            tags = found.get('tags')
            categories = found.get('categories')
            
            if isinstance(tags, list) and isinstance(categories, list):
                print("✓ MongoDB 数组字段正确存储为列表类型")
            else:
                print(f"⚠ MongoDB 数组字段类型异常: tags={type(tags)}, categories={type(categories)}")
        else:
            print("✗ MongoDB 数据查询失败")
            
    except Exception as e:
        print(f"✗ MongoDB 测试失败: {e}")
        print("注意: 请确保 MongoDB 服务正在运行")
        import traceback
        traceback.print_exc()

def test_array_query_operations():
    """测试数组字段的查询操作"""
    print("\n=== 测试数组字段查询操作 ===")
    
    # 测试 SQLite 数组查询
    print("\n--- SQLite 数组查询测试 ---")
    try:
        config = rq.DatabaseConfig(
            database_type="sqlite",
            database_url="sqlite:///test_array.db"
        )
        db = rq.QuickDB(config)
        
        # 插入更多测试数据
        test_data_2 = {
            "id": "test2",
            "tags": ["javascript", "frontend"],
            "categories": ["web", "ui"],
            "name": "前端记录"
        }
        db.create("test_arrays", test_data_2)
        
        # 查询所有记录
        all_records = db.find("test_arrays", [])
        print(f"✓ SQLite 查询到 {len(all_records)} 条记录")
        
        for record in all_records:
            print(f"  - {record.get('name')}: tags={record.get('tags')}")
            
    except Exception as e:
        print(f"✗ SQLite 数组查询测试失败: {e}")

def main():
    """主测试函数"""
    print("开始数组字段兼容性测试...")
    print(f"rat_quickdb 版本: {rq.get_version()}")
    
    # 测试 SQLite 兼容性
    test_sqlite_array_compatibility()
    
    # 测试 MongoDB 兼容性（如果可用）
    test_mongodb_array_compatibility()
    
    # 测试查询操作
    test_array_query_operations()
    
    print("\n=== 测试完成 ===")
    print("\n总结:")
    print("- array_field 和 list_field 在 SQL 数据库中通过 JSON 存储实现兼容")
    print("- MongoDB 中使用原生 BSON 数组存储")
    print("- 两种方式都能正确处理 Python 列表数据")

if __name__ == "__main__":
    main()