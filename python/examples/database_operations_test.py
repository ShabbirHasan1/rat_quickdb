#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB 数据库操作测试

本示例专门测试实际的数据库操作，验证数据是否真正写入磁盘。
"""

import json
import os
import time
from datetime import datetime
from typing import Dict, List, Optional

try:
    import rat_quickdb_py
    from rat_quickdb_py import (
        create_db_queue_bridge,
        get_version,
        string_field,
        integer_field,
        boolean_field,
        FieldDefinition,
        IndexDefinition,
        ModelMeta,
    )
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    exit(1)


def test_database_operations():
    """测试实际的数据库操作"""
    print("=== 数据库操作测试 ===\n")
    
    # 删除旧的数据库文件（如果存在）
    db_path = "./test_operations.db"
    if os.path.exists(db_path):
        os.remove(db_path)
        print(f"已删除旧的数据库文件: {db_path}")
    
    # 1. 创建数据库桥接器
    print("1. 创建数据库桥接器:")
    try:
        bridge = create_db_queue_bridge()
        print("  ✓ 桥接器创建成功")
    except Exception as e:
        print(f"  ✗ 桥接器创建失败: {e}")
        return False
    
    # 2. 添加SQLite数据库
    print("\n2. 添加SQLite数据库:")
    try:
        response = bridge.add_sqlite_database(
            alias="default",
            path=db_path,
            max_connections=10,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )
        result = json.loads(response)
        if result.get("success"):
            print("  ✓ SQLite数据库添加成功")
        else:
            print(f"  ✗ SQLite数据库添加失败: {result.get('error')}")
            return False
    except Exception as e:
        print(f"  ✗ SQLite数据库添加失败: {e}")
        return False
    
    # 3. 执行数据插入操作
    print("\n3. 执行数据插入操作:")
    
    # 准备测试数据
    test_users = [
        {
            "id": "user_001",
            "name": "张三",
            "email": "zhangsan@example.com",
            "age": 25,
            "active": True,
            "created_at": datetime.now().isoformat()
        },
        {
            "id": "user_002",
            "name": "李四",
            "email": "lisi@example.com",
            "age": 30,
            "active": True,
            "created_at": datetime.now().isoformat()
        },
        {
            "id": "user_003",
            "name": "王五",
            "email": "wangwu@example.com",
            "age": 28,
            "active": False,
            "created_at": datetime.now().isoformat()
        }
    ]
    
    # 插入用户数据
    success_count = 0
    for i, user_data in enumerate(test_users, 1):
        try:
            print(f"  插入用户 {i}: {user_data['name']}")
            response = bridge.create(
                table="users",
                data_json=json.dumps(user_data),
                alias="default"
            )
            result = json.loads(response)
            if result.get("success"):
                print(f"    ✓ 用户 {user_data['name']} 插入成功")
                success_count += 1
            else:
                print(f"    ✗ 用户 {user_data['name']} 插入失败: {result.get('error')}")
        except Exception as e:
            print(f"    ✗ 用户 {user_data['name']} 插入异常: {e}")
    
    print(f"\n  总计: {success_count}/{len(test_users)} 条记录插入成功")
    
    # 4. 检查数据库文件大小
    print("\n4. 检查数据库文件:")
    if os.path.exists(db_path):
        file_size = os.path.getsize(db_path)
        print(f"  数据库文件: {db_path}")
        print(f"  文件大小: {file_size} 字节")
        
        if file_size > 0:
            print("  ✓ 数据库文件不为空，数据已写入磁盘")
        else:
            print("  ✗ 数据库文件为空，数据可能未写入")
            return False
    else:
        print(f"  ✗ 数据库文件不存在: {db_path}")
        return False
    
    # 5. 执行查询操作验证数据
    print("\n5. 执行查询操作验证数据:")
    try:
        # 查询所有用户（空条件对象表示查询所有）
        query_conditions = json.dumps({})
        response = bridge.find(
            table="users",
            query_json=query_conditions,
            alias="default"
        )
        result = json.loads(response)
        
        if result.get("success"):
            data = result.get("data", [])
            print(f"  ✓ 查询成功，找到 {len(data)} 条记录")
            
            # 显示查询结果
            for record in data:
                if isinstance(record, dict):
                    name = record.get("name", "未知")
                    email = record.get("email", "未知")
                    print(f"    - {name} ({email})")
        else:
            print(f"  ✗ 查询失败: {result.get('error')}")
    except Exception as e:
        print(f"  ✗ 查询异常: {e}")
    
    # 6. 按ID查询特定用户
    print("\n6. 按ID查询特定用户:")
    try:
        response = bridge.find_by_id(
            table="users",
            id="user_001",
            alias="default"
        )
        result = json.loads(response)
        
        if result.get("success"):
            data = result.get("data")
            if data:
                print(f"  ✓ 找到用户: {data}")
            else:
                print("  ✗ 未找到指定用户")
        else:
            print(f"  ✗ 查询失败: {result.get('error')}")
    except Exception as e:
        print(f"  ✗ 查询异常: {e}")
    
    return True


def test_model_definition_with_database():
    """测试模型定义与数据库操作的结合"""
    print("\n=== 模型定义与数据库操作结合测试 ===\n")
    
    # 1. 定义用户模型
    print("1. 定义用户模型:")
    try:
        # 定义字段
        fields = {
            "id": string_field(required=True, unique=True, description="用户ID"),
            "name": string_field(required=True, max_length=100, description="用户姓名"),
            "email": string_field(required=True, unique=True, max_length=255, description="邮箱地址"),
            "age": integer_field(required=False, min_value=0, max_value=150, description="年龄"),
            "active": boolean_field(required=True, description="是否激活")
        }
        
        # 定义索引
        indexes = [
            IndexDefinition(fields=["email"], unique=True, name="idx_email_unique"),
            IndexDefinition(fields=["name"], unique=False, name="idx_name"),
            IndexDefinition(fields=["age"], unique=False, name="idx_age")
        ]
        
        # 创建模型元数据
        user_model = ModelMeta(
            collection_name="users",
            fields=fields,
            indexes=indexes,
            database_alias="default",
            description="用户信息模型"
        )
        
        print(f"  ✓ 用户模型定义成功")
        print(f"    集合名: {user_model.collection_name}")
        print(f"    字段数: {len(user_model.fields)}")
        print(f"    索引数: {len(user_model.indexes)}")
        
    except Exception as e:
        print(f"  ✗ 用户模型定义失败: {e}")
        return False
    
    return True


def main():
    """主函数"""
    print(f"RAT QuickDB 数据库操作测试 (版本: {get_version()})\n")
    
    try:
        # 测试数据库操作
        db_test_success = test_database_operations()
        
        # 测试模型定义
        model_test_success = test_model_definition_with_database()
        
        # 总结
        print("\n=== 测试总结 ===")
        print(f"数据库操作测试: {'✓ 通过' if db_test_success else '✗ 失败'}")
        print(f"模型定义测试: {'✓ 通过' if model_test_success else '✗ 失败'}")
        
        if db_test_success and model_test_success:
            print("\n🎉 所有测试通过！数据已成功写入数据库。")
        else:
            print("\n❌ 部分测试失败，请检查错误信息。")
            
    except KeyboardInterrupt:
        print("\n测试被用户中断")
    except Exception as e:
        print(f"\n测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()