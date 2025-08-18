#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
rat_quickdb Python 绑定综合示例

本示例展示了 rat_quickdb Python 绑定的完整功能，包括：
1. 库信息和版本查询
2. 配置管理（构建器模式）
3. 数据库操作（CRUD）
4. 队列桥接器
5. 模型系统（字段定义、索引、元数据）
"""

import json
import time
from typing import Dict, Any, List

# 导入 rat_quickdb Python 绑定
import rat_quickdb_py
from rat_quickdb_py import (
    # 基础信息
    get_version, get_name, get_info,
    # 配置管理
    PoolConfigBuilder, create_default_pool_config,
    # 数据库操作
    SimpleDbManager, create_simple_db_manager,
    DataValue, QueryOperator, QueryCondition,
    # 队列桥接
    SimpleQueueBridge, create_simple_queue_bridge,
    # 模型系统
    FieldType, FieldDefinition, IndexDefinition, ModelMeta, ModelManager, create_model_manager
)


def print_section(title: str):
    """打印章节标题"""
    print(f"\n{'='*60}")
    print(f" {title}")
    print(f"{'='*60}")


def demo_basic_info():
    """演示基础信息查询"""
    print_section("1. 基础信息查询")
    
    print(f"库名称: {get_name()}")
    print(f"版本号: {get_version()}")
    print(f"库信息: {get_info()}")
    
    # 显示可用功能
    available_features = [x for x in dir(rat_quickdb_py) if not x.startswith('_')]
    print(f"可用功能数量: {len(available_features)}")
    print(f"主要功能: {', '.join(available_features[:10])}...")


def demo_config_management():
    """演示配置管理"""
    print_section("2. 配置管理（构建器模式）")
    
    # 使用构建器创建配置
    print("\n2.1 使用构建器创建连接池配置:")
    builder = PoolConfigBuilder()
    config = (
        builder
        .max_connections(20)
        .min_connections(5)
        .connection_timeout(30)
        .idle_timeout(300)
        .max_lifetime(3600)
        .build()
    )
    print(f"连接池配置创建成功: {type(config)}")
    print(f"最大连接数: {config.max_connections}")
    print(f"最小连接数: {config.min_connections}")
    print(f"连接超时: {config.connection_timeout}s")
    print(f"空闲超时: {config.idle_timeout}s")
    print(f"最大生命周期: {config.max_lifetime}s")
    
    # 使用默认配置
    print("\n2.2 使用默认配置:")
    default_config = create_default_pool_config(min_connections=2, max_connections=10)
    print(f"默认配置创建成功: {type(default_config)}")
    print(f"默认最大连接数: {default_config.max_connections}")
    print(f"默认最小连接数: {default_config.min_connections}")
    print(f"默认连接超时: {default_config.connection_timeout}s")
    print(f"默认空闲超时: {default_config.idle_timeout}s")
    print(f"默认最大生命周期: {default_config.max_lifetime}s")


def demo_database_operations():
    """演示数据库操作"""
    print_section("3. 数据库操作（CRUD）")
    
    # 创建数据库管理器
    print("\n3.1 创建数据库管理器:")
    db_manager = create_simple_db_manager()
    print(f"数据库管理器创建成功: {type(db_manager)}")
    
    # 测试连接
    print("\n3.2 测试数据库连接:")
    is_connected = db_manager.test_connection()
    print(f"数据库连接状态: {'成功' if is_connected else '失败'}")
    
    # 创建记录
    print("\n3.3 创建数据记录:")
    user_data = {
        "name": "张三",
        "age": "25",
        "email": "zhangsan@example.com",
        "active": "true"
    }
    
    record_id = db_manager.create_record("users", user_data)
    print(f"用户记录创建成功，ID: {record_id}")
    
    # 查询记录
    print("\n3.4 查询数据记录:")
    condition = QueryCondition("name", QueryOperator.eq(), DataValue.string("张三"))
    found_records = db_manager.find_records("users", [condition])
    print(f"查询到 {len(found_records)} 条记录")
    
    # 更新记录
    print("\n3.5 更新数据记录:")
    update_data = {"age": "26"}
    updated_count = db_manager.update_records("users", [condition], update_data)
    print(f"更新了 {updated_count} 条记录")
    
    # 统计记录
    print("\n3.6 统计记录数量:")
    total_count = db_manager.count_records("users", [])
    print(f"users 集合总记录数: {total_count}")
    
    # 检查记录存在性
    print("\n3.7 检查记录存在性:")
    exists_condition = QueryCondition("name", QueryOperator.eq(), DataValue.string("张三"))
    # 注意：SimpleDbManager 没有 record_exists 方法，我们跳过这个测试
    print("记录存在性检查: 跳过（SimpleDbManager 不支持此方法）")


def demo_queue_bridge():
    """演示队列桥接器"""
    print_section("4. 队列桥接器")
    
    # 创建队列桥接器
    print("\n4.1 创建队列桥接器:")
    queue_bridge = create_simple_queue_bridge()
    print(f"队列桥接器创建成功: {type(queue_bridge)}")
    
    # 测试连接
    print("\n4.2 测试队列连接:")
    is_connected = queue_bridge.test_connection()
    print(f"队列连接状态: {'成功' if is_connected else '失败'}")
    
    # 获取队列统计
    print("\n4.3 获取队列统计:")
    stats = queue_bridge.get_queue_stats()
    print(f"队列统计信息: {stats}")
    
    # 创建队列记录
    print("\n4.4 创建队列记录:")
    queue_data = {
        "task_id": "task_001",
        "priority": "1",
        "payload": json.dumps({"action": "process_data", "data": [1, 2, 3]})
    }
    
    task_id = queue_bridge.create_record("task_queue", json.dumps(queue_data))
    print(f"队列任务创建成功，ID: {task_id}")
    
    # 查询队列记录
    print("\n4.5 查询队列记录:")
    query_conditions = json.dumps([{"field": "task_id", "operator": "eq", "value": "task_001"}])
    found_tasks = queue_bridge.find_records("task_queue", query_conditions)
    print(f"查询到 {len(found_tasks)} 个任务")
    
    # 统计队列记录
    print("\n4.6 统计队列记录:")
    empty_conditions = json.dumps([])
    queue_count = queue_bridge.count_records("task_queue", empty_conditions)
    print(f"task_queue 队列总任务数: {queue_count}")


def demo_model_system():
    """演示模型系统"""
    print_section("5. 模型系统")
    
    # 定义字段类型
    print("\n5.1 定义字段类型:")
    string_type = FieldType.string()
    integer_type = FieldType.integer()
    boolean_type = FieldType.boolean()
    datetime_type = FieldType.datetime()
    
    print(f"字符串类型: {string_type}")
    print(f"整数类型: {integer_type}")
    print(f"布尔类型: {boolean_type}")
    print(f"日期时间类型: {datetime_type}")
    
    # 定义字段
    print("\n5.2 定义模型字段:")
    name_field = FieldDefinition(FieldType.string())
    age_field = FieldDefinition(FieldType.integer())
    email_field = FieldDefinition(FieldType.string())
    active_field = FieldDefinition(FieldType.boolean())
    created_at_field = FieldDefinition(FieldType.datetime())
    
    print(f"姓名字段: required={name_field.required()}, unique={name_field.unique()}")
    print(f"年龄字段: required={age_field.required()}, indexed={age_field.indexed()}")
    print(f"邮箱字段: required={email_field.required()}, unique={email_field.unique()}")
    
    # 定义索引
    print("\n5.3 定义模型索引:")
    name_index = IndexDefinition(["name"], unique=True, name="name_unique_idx")
    email_index = IndexDefinition(["email"], unique=True, name="email_unique_idx")
    age_index = IndexDefinition(["age"], unique=False, name="age_idx")
    compound_index = IndexDefinition(["name", "age"], unique=False, name="name_age_idx")
    
    print(f"姓名索引: fields={name_index.get_fields()}, unique={name_index.get_unique()}")
    print(f"邮箱索引: fields={email_index.get_fields()}, unique={email_index.get_unique()}")
    print(f"年龄索引: fields={age_index.get_fields()}, unique={age_index.get_unique()}")
    print(f"复合索引: fields={compound_index.get_fields()}, unique={compound_index.get_unique()}")
    
    # 创建模型元数据
    print("\n5.4 创建模型元数据:")
    fields = {
        "name": name_field,
        "age": age_field,
        "email": email_field,
        "active": active_field,
        "created_at": created_at_field
    }
    
    indexes = [name_index, email_index, age_index, compound_index]
    
    user_meta = ModelMeta(
        "users",  # collection_name
        fields,   # fields
        indexes,  # indexes
        "default",  # database_alias
        "用户模型，包含基本用户信息"  # description
    )
    
    print(f"模型集合名: {user_meta.get_collection_name()}")
    print(f"数据库别名: {user_meta.get_database_alias()}")
    print(f"模型描述: {user_meta.get_description()}")
    print(f"字段数量: {len(user_meta.get_fields())}")
    print(f"索引数量: {len(user_meta.get_indexes())}")
    
    # 创建模型管理器
    print("\n5.5 创建模型管理器:")
    model_manager = create_model_manager("users")
    print(f"模型管理器创建成功: {type(model_manager)}")


def demo_performance_comparison():
    """演示性能对比"""
    print_section("6. 性能对比测试")
    
    # 创建数据库管理器
    db_manager = create_simple_db_manager()
    
    # 批量创建记录性能测试
    print("\n6.1 批量创建记录性能测试:")
    start_time = time.time()
    
    for i in range(100):
        user_data = {
            "name": f"用户_{i:03d}",
            "age": str(20 + (i % 50)),
            "email": f"user_{i:03d}@example.com",
            "active": str(i % 2 == 0).lower()
        }
        db_manager.create_record("performance_test", user_data)
    
    create_time = time.time() - start_time
    print(f"创建 100 条记录耗时: {create_time:.3f} 秒")
    print(f"平均每条记录: {create_time/100*1000:.2f} 毫秒")
    
    # 批量查询性能测试
    print("\n6.2 批量查询性能测试:")
    start_time = time.time()
    
    for i in range(50):
        condition = QueryCondition("name", QueryOperator.eq(), DataValue.string(f"用户_{i:03d}"))
        results = db_manager.find_records("performance_test", [condition])
    
    query_time = time.time() - start_time
    print(f"执行 50 次查询耗时: {query_time:.3f} 秒")
    print(f"平均每次查询: {query_time/50*1000:.2f} 毫秒")
    
    # 统计总记录数
    total_records = db_manager.count_records("performance_test", [])
    print(f"\n性能测试集合总记录数: {total_records}")


def main():
    """主函数"""
    print("rat_quickdb Python 绑定综合示例")
    print("本示例展示了 rat_quickdb 的完整功能")
    
    try:
        # 1. 基础信息查询
        demo_basic_info()
        
        # 2. 配置管理
        demo_config_management()
        
        # 3. 数据库操作
        demo_database_operations()
        
        # 4. 队列桥接器
        demo_queue_bridge()
        
        # 5. 模型系统
        demo_model_system()
        
        # 6. 性能对比
        demo_performance_comparison()
        
        print_section("示例执行完成")
        print("所有功能演示成功完成！")
        print("\n主要特性总结:")
        print("✓ 基础信息查询 - 获取库版本和信息")
        print("✓ 配置管理 - 构建器模式创建连接池配置")
        print("✓ 数据库操作 - 完整的 CRUD 操作")
        print("✓ 队列桥接 - 无锁队列通信")
        print("✓ 模型系统 - 字段定义、索引和元数据管理")
        print("✓ 性能测试 - 批量操作性能评估")
        
    except Exception as e:
        print(f"\n❌ 示例执行出错: {e}")
        print("请检查 rat_quickdb 配置和依赖")
        raise


if __name__ == "__main__":
    main()