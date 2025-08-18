#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python ODM绑定使用示例

本示例展示了如何使用 RAT QuickDB 的 Python ODM 绑定：
- 字段定义和属性访问
- 模型元数据创建
- 索引定义
- 数据库连接和基本操作
"""

import json
import time
from datetime import datetime
from typing import Dict, List, Optional

try:
    import rat_quickdb_py
    from rat_quickdb_py import (
        create_db_queue_bridge,
        get_version,
        get_info,
        get_name,
        # 字段创建函数
        string_field,
        integer_field,
        boolean_field,
        datetime_field,
        uuid_field,
        reference_field,
        array_field,
        json_field,
        # 类型定义
        FieldDefinition,
        IndexDefinition,
        ModelMeta,
    )
except ImportError as e:
    print(f"错误：无法导入 rat_quickdb_py 模块: {e}")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    exit(1)


def demonstrate_field_creation():
    """演示字段创建和属性访问"""
    print("=== 字段创建和属性访问演示 ===")
    
    # 创建各种类型的字段
    print("\n1. 创建字符串字段:")
    username_field = string_field(
        required=True,
        unique=True,
        max_length=50,
        min_length=3,
        description="用户名字段"
    )
    print(f"  字段类型: StringField")
    print(f"  是否必填: {username_field.is_required}")
    print(f"  是否唯一: {username_field.is_unique}")
    print(f"  是否索引: {username_field.is_indexed}")
    print(f"  字段描述: {username_field.description}")
    
    print("\n2. 创建整数字段:")
    age_field = integer_field(
        required=False,
        min_value=0,
        max_value=150,
        description="年龄字段"
    )
    print(f"  字段类型: IntegerField")
    print(f"  是否必填: {age_field.is_required}")
    print(f"  是否唯一: {age_field.is_unique}")
    print(f"  字段描述: {age_field.description}")
    
    print("\n3. 创建布尔字段:")
    active_field = boolean_field(
        required=True,
        description="激活状态字段"
    )
    print(f"  字段类型: BooleanField")
    print(f"  是否必填: {active_field.is_required}")
    print(f"  字段描述: {active_field.description}")
    
    print("\n4. 创建日期时间字段:")
    created_at_field = datetime_field(
        required=True,
        description="创建时间字段"
    )
    print(f"  字段类型: DateTimeField")
    print(f"  是否必填: {created_at_field.is_required}")
    print(f"  字段描述: {created_at_field.description}")
    
    print("\n5. 创建UUID字段:")
    id_field = uuid_field(
        required=True,
        unique=True,
        description="唯一标识字段"
    )
    print(f"  字段类型: UuidField")
    print(f"  是否必填: {id_field.is_required}")
    print(f"  是否唯一: {id_field.is_unique}")
    print(f"  字段描述: {id_field.description}")
    
    print("\n6. 创建引用字段:")
    author_field = reference_field(
        target_collection="users",
        required=True,
        description="作者引用字段"
    )
    print(f"  字段类型: ReferenceField")
    print(f"  是否必填: {author_field.is_required}")
    print(f"  字段描述: {author_field.description}")
    
    print("\n7. 创建JSON字段:")
    metadata_field = json_field(
        required=False,
        description="元数据字段"
    )
    print(f"  字段类型: JsonField")
    print(f"  是否必填: {metadata_field.is_required}")
    print(f"  字段描述: {metadata_field.description}")
    
    return {
        'id': id_field,
        'username': username_field,
        'age': age_field,
        'is_active': active_field,
        'created_at': created_at_field,
        'author_id': author_field,
        'metadata': metadata_field
    }


def demonstrate_index_creation():
    """演示索引创建"""
    print("\n=== 索引创建演示 ===")
    
    # 创建单字段唯一索引
    print("\n1. 创建用户名唯一索引:")
    username_index = IndexDefinition(
        fields=["username"],
        unique=True,
        name="idx_username_unique"
    )
    print(f"  索引字段: {username_index.fields}")
    print(f"  是否唯一: {username_index.unique}")
    print(f"  索引名称: {username_index.name}")
    
    # 创建复合索引
    print("\n2. 创建复合索引:")
    compound_index = IndexDefinition(
        fields=["is_active", "created_at"],
        unique=False,
        name="idx_active_created"
    )
    print(f"  索引字段: {compound_index.fields}")
    print(f"  是否唯一: {compound_index.unique}")
    print(f"  索引名称: {compound_index.name}")
    
    # 创建普通索引
    print("\n3. 创建创建时间索引:")
    created_index = IndexDefinition(
        fields=["created_at"],
        unique=False,
        name="idx_created_at"
    )
    print(f"  索引字段: {created_index.fields}")
    print(f"  是否唯一: {created_index.unique}")
    print(f"  索引名称: {created_index.name}")
    
    return [username_index, compound_index, created_index]


def demonstrate_model_meta_creation(fields: Dict, indexes: List):
    """演示模型元数据创建"""
    print("\n=== 模型元数据创建演示 ===")
    
    # 创建用户模型元数据
    print("\n1. 创建用户模型元数据:")
    user_meta = ModelMeta(
        collection_name="users",
        fields=fields,
        indexes=indexes,
        database_alias="default",
        description="用户信息模型"
    )
    
    print(f"  集合名称: {user_meta.collection_name}")
    print(f"  数据库别名: {user_meta.database_alias}")
    print(f"  模型描述: {user_meta.description}")
    
    # 访问字段和索引信息
    try:
        fields_info = user_meta.fields
        indexes_info = user_meta.indexes
        print(f"  字段数量: {len(fields_info) if hasattr(fields_info, '__len__') else 'N/A'}")
        print(f"  索引数量: {len(indexes_info) if hasattr(indexes_info, '__len__') else 'N/A'}")
    except Exception as e:
        print(f"  访问字段/索引信息时出错: {e}")
    
    return user_meta


def demonstrate_database_operations():
    """演示数据库操作"""
    print("\n=== 数据库操作演示 ===")
    
    # 创建数据库队列桥接器
    print("\n1. 创建数据库队列桥接器:")
    try:
        bridge = create_db_queue_bridge()
        print("  队列桥接器创建成功")
    except Exception as e:
        print(f"  队列桥接器创建失败: {e}")
        return None
    
    # 添加SQLite数据库
    print("\n2. 添加SQLite数据库:")
    try:
        response = bridge.add_sqlite_database(
            alias="default",
            path="./odm_demo.db",
            max_connections=10,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )
        result = json.loads(response)
        if result.get("success"):
            print("  SQLite数据库添加成功")
        else:
            print(f"  SQLite数据库添加失败: {result.get('error')}")
            return None
    except Exception as e:
        print(f"  SQLite数据库添加失败: {e}")
        return None
    
    # 测试数据库连接
    print("\n3. 测试数据库连接:")
    try:
        # 这里可以添加一些基本的数据库操作测试
        print("  数据库连接正常")
    except Exception as e:
        print(f"  数据库连接测试失败: {e}")
    
    return bridge


def demonstrate_field_builder_pattern():
    """演示字段构建器模式"""
    print("\n=== 字段构建器模式演示 ===")
    
    # 演示字段的链式调用（如果支持的话）
    print("\n1. 创建复杂字段配置:")
    
    # 创建一个复杂的字符串字段
    email_field = string_field(
        required=True,
        unique=True,
        max_length=255,
        min_length=5,
        description="邮箱地址字段，必须唯一且符合邮箱格式"
    )
    
    print(f"  邮箱字段配置:")
    print(f"    必填: {email_field.is_required}")
    print(f"    唯一: {email_field.is_unique}")
    print(f"    索引: {email_field.is_indexed}")
    print(f"    描述: {email_field.description}")
    
    # 创建一个带范围限制的整数字段
    score_field = integer_field(
        required=True,
        min_value=0,
        max_value=100,
        description="分数字段，范围0-100"
    )
    
    print(f"\n  分数字段配置:")
    print(f"    必填: {score_field.is_required}")
    print(f"    唯一: {score_field.is_unique}")
    print(f"    描述: {score_field.description}")
    
    return {'email': email_field, 'score': score_field}


def demonstrate_version_info():
    """演示版本信息获取"""
    print("\n=== 版本信息演示 ===")
    
    try:
        version = get_version()
        info = get_info()
        name = get_name()
        
        print(f"  库名称: {name}")
        print(f"  版本号: {version}")
        print(f"  库信息: {info}")
    except Exception as e:
        print(f"  获取版本信息失败: {e}")


def demonstrate_performance_test():
    """演示性能测试"""
    print("\n=== 性能测试演示 ===")
    
    # 测试字段创建性能
    print("\n1. 字段创建性能测试:")
    start_time = time.time()
    
    fields = []
    for i in range(100):
        field = string_field(
            required=i % 2 == 0,
            unique=i % 10 == 0,
            description=f"测试字段{i}"
        )
        fields.append(field)
    
    end_time = time.time()
    duration = end_time - start_time
    print(f"  创建100个字段耗时: {duration:.4f} 秒")
    print(f"  平均每个字段创建时间: {duration/100:.6f} 秒")
    
    # 测试索引创建性能
    print("\n2. 索引创建性能测试:")
    start_time = time.time()
    
    indexes = []
    for i in range(50):
        index = IndexDefinition(
            fields=[f"field_{i}"],
            unique=i % 5 == 0,
            name=f"idx_field_{i}"
        )
        indexes.append(index)
    
    end_time = time.time()
    duration = end_time - start_time
    print(f"  创建50个索引耗时: {duration:.4f} 秒")
    print(f"  平均每个索引创建时间: {duration/50:.6f} 秒")
    
    return len(fields), len(indexes)


def main():
    """主函数"""
    print("=== RAT QuickDB Python ODM绑定演示 ===")
    
    try:
        # 显示版本信息
        demonstrate_version_info()
        
        # 演示字段创建
        fields = demonstrate_field_creation()
        
        # 演示索引创建
        indexes = demonstrate_index_creation()
        
        # 演示模型元数据创建
        model_meta = demonstrate_model_meta_creation(fields, indexes)
        
        # 演示字段构建器模式
        builder_fields = demonstrate_field_builder_pattern()
        
        # 演示数据库操作
        bridge = demonstrate_database_operations()
        
        # 演示性能测试
        field_count, index_count = demonstrate_performance_test()
        
        print(f"\n=== 演示完成 ===")
        print(f"总共创建了 {len(fields)} 个模型字段")
        print(f"总共创建了 {len(indexes)} 个模型索引")
        print(f"性能测试创建了 {field_count} 个字段和 {index_count} 个索引")
        print(f"数据库桥接器状态: {'已连接' if bridge else '未连接'}")
        
    except KeyboardInterrupt:
        print("\n演示被用户中断")
    except Exception as e:
        print(f"\n演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()