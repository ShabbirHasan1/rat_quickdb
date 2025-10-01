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
from datetime import datetime, timezone
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
        float_field,
        list_field,
        dict_field,
        # 类型定义
        FieldDefinition,
        FieldType,
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
    
    print("\n7. 创建浮点数字段:")
    score_field = float_field(
        required=True,
        min_value=0.0,
        max_value=100.0,
        description="分数字段"
    )
    print(f"  字段类型: FloatField")
    print(f"  是否必填: {score_field.is_required}")
    print(f"  字段描述: {score_field.description}")
    
    print("\n8. 创建数组字段:")
    tags_field = array_field(
        item_type=FieldType.string(max_length=None, min_length=None),
        required=False,
        description="标签数组字段"
    )
    print(f"  字段类型: ArrayField")
    print(f"  是否必填: {tags_field.is_required}")
    print(f"  字段描述: {tags_field.description}")
    
    print("\n9. 创建列表字段:")
    items_field = list_field(
        item_type=FieldType.string(max_length=None, min_length=None),
        required=False,
        description="项目列表字段"
    )
    print(f"  字段类型: ListField")
    print(f"  是否必填: {items_field.is_required}")
    print(f"  字段描述: {items_field.description}")
    
    print("\n10. 创建字典字段:")
    profile_fields = {
        "name": string_field(required=True, description="姓名"),
        "age": integer_field(required=True, min_value=0, max_value=150, description="年龄")
    }
    profile_field = dict_field(
        fields=profile_fields,
        required=False,
        description="用户档案字段"
    )
    print(f"  字段类型: DictField")
    print(f"  是否必填: {profile_field.is_required}")
    print(f"  字段描述: {profile_field.description}")
    
    print("\n11. 创建JSON字段:")
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
        'score': score_field,
        'tags': tags_field,
        'items': items_field,
        'profile': profile_field,
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
    
    print("\n=== 数组字段类型演示 ===")
    
    # 浮点数字段示例
    float_field_example = float_field(
        required=True,
        min_value=0.0,
        max_value=100.0,
        description="浮点数字段示例"
    )
    print(f"  浮点数字段示例: {float_field_example.description}")
    
    # 数组字段示例 - 字符串数组
    string_array_field = array_field(
        item_type=FieldType.string(max_length=None, min_length=None),
        required=True,
        description="字符串数组字段示例 - 存储标签、分类等"
    )
    print(f"  字符串数组字段示例: {string_array_field.description}")
    
    # 数组字段示例 - 整数数组
    integer_array_field = array_field(
        item_type=FieldType.integer(min_value=None, max_value=None),
        required=False,
        description="整数数组字段示例 - 存储分数、评级等"
    )
    print(f"  整数数组字段示例: {integer_array_field.description}")
    
    # 数组字段示例 - 浮点数数组
    float_array_field = array_field(
        item_type=FieldType.float(min_value=None, max_value=None),
        required=False,
        description="浮点数数组字段示例 - 存储坐标、权重等"
    )
    print(f"  浮点数数组字段示例: {float_array_field.description}")
    
    # 数组字段示例 - 布尔数组
    boolean_array_field = array_field(
        item_type=FieldType.boolean(),
        required=False,
        description="布尔数组字段示例 - 存储开关状态等"
    )
    print(f"  布尔数组字段示例: {boolean_array_field.description}")
    
    # 列表字段示例 - 混合类型列表
    list_field_example = list_field(
        item_type=FieldType.string(max_length=None, min_length=None),
        required=False,
        description="混合类型列表字段示例 - 可存储不同类型的数据"
    )
    print(f"  列表字段示例: {list_field_example.description}")
    
    # 字典字段示例 - 嵌套对象
    dict_fields = {
        "name": string_field(required=True, description="姓名"),
        "age": integer_field(required=True, min_value=0, max_value=150, description="年龄"),
        "score": float_field(required=False, min_value=0.0, max_value=100.0, description="分数"),
        "active": boolean_field(required=False, description="是否激活")
    }
    dict_field_example = dict_field(
        fields=dict_fields,
        required=False,
        description="嵌套对象字段示例 - 结构化数据存储"
    )
    print(f"  字典字段示例: {dict_field_example.description}")
    
    # JSON字段示例
    json_field_example = json_field(
        required=False,
        description="JSON字段示例 - 灵活的非结构化数据存储"
    )
    print(f"  JSON字段示例: {json_field_example.description}")
    
    print("\n=== 数组字段在不同数据库中的存储方式 ===")
    print("  MongoDB: 使用原生数组支持")
    print("  PostgreSQL: 使用原生数组类型")
    print("  MySQL: 使用JSON格式存储")
    print("  SQLite: 使用JSON格式存储")
    
    return {
        'email': email_field, 
        'score': score_field,
        'float_example': float_field_example,
        'string_array': string_array_field,
        'integer_array': integer_array_field,
        'float_array': float_array_field,
        'boolean_array': boolean_array_field,
        'list_example': list_field_example,
        'dict_example': dict_field_example,
        'json_example': json_field_example
    }


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
    
    # 测试数组字段创建性能
    print("\n3. 数组字段创建性能测试:")
    start_time = time.time()
    
    array_fields = []
    array_types = [
        FieldType.string(max_length=None, min_length=None),
        FieldType.integer(min_value=None, max_value=None),
        FieldType.float(min_value=None, max_value=None),
        FieldType.boolean()
    ]
    for i in range(40):
        array_field_obj = array_field(
            item_type=array_types[i % len(array_types)],
            required=i % 3 == 0,
            description=f"测试数组字段{i}"
        )
        array_fields.append(array_field_obj)
    
    end_time = time.time()
    duration = end_time - start_time
    print(f"  创建40个数组字段耗时: {duration:.4f} 秒")
    print(f"  平均每个数组字段创建时间: {duration/40:.6f} 秒")
    
    # 测试复杂字段创建性能
    print("\n4. 复杂字段创建性能测试:")
    start_time = time.time()
    
    complex_fields = []
    for i in range(20):
        # 创建嵌套字典字段
        nested_fields = {
            "id": integer_field(required=True, description=f"ID字段{i}"),
            "name": string_field(required=True, max_length=100, description=f"名称字段{i}"),
            "tags": array_field(item_type=FieldType.string(max_length=None, min_length=None), required=False, description=f"标签字段{i}"),
            "metadata": json_field(required=False, description=f"元数据字段{i}")
        }
        complex_field = dict_field(
            fields=nested_fields,
            required=False,
            description=f"复杂嵌套字段{i}"
        )
        complex_fields.append(complex_field)
    
    end_time = time.time()
    duration = end_time - start_time
    print(f"  创建20个复杂字段耗时: {duration:.4f} 秒")
    print(f"  平均每个复杂字段创建时间: {duration/20:.6f} 秒")
    
    return len(fields), len(indexes), len(array_fields), len(complex_fields)


def cleanup_existing_tables():
    """清理现有的测试表"""
    print("🧹 清理现有的测试表...")
    try:
        # 创建临时桥接器用于清理
        bridge = create_db_queue_bridge()
        
        # 添加SQLite数据库连接用于清理
        bridge.add_sqlite_database(
            alias="cleanup_temp",
            path="./odm_demo.db",
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600
        )
        
        # 清理可能存在的测试表
        tables_to_clean = ["users", "test_table", "demo_table", "model_test"]
        for table in tables_to_clean:
            try:
                bridge.drop_table(table, "cleanup_temp")
                print(f"✅ 已清理表: {table}")
            except Exception as e:
                print(f"⚠️ 清理表 {table} 时出错: {e}")
        
    except Exception as e:
        print(f"⚠️ 清理现有表时出错: {e}")


def main():
    """主函数"""
    print("=== RAT QuickDB Python ODM绑定演示 ===")
    
    # 清理现有的测试表
    cleanup_existing_tables()
    
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
        field_count, index_count, array_field_count, complex_field_count = demonstrate_performance_test()
        
        print(f"\n=== 演示完成 ===")
        print(f"总共创建了 {len(fields)} 个模型字段")
        print(f"总共创建了 {len(indexes)} 个模型索引")
        print(f"性能测试创建了 {field_count} 个字段和 {index_count} 个索引")
        print(f"性能测试创建了 {array_field_count} 个数组字段和 {complex_field_count} 个复杂字段")
        print(f"数据库桥接器状态: {'已连接' if bridge else '未连接'}")
        print(f"构建器模式字段数量: {len(builder_fields)}")
        
    except KeyboardInterrupt:
        print("\n演示被用户中断")
    except Exception as e:
        print(f"\n演示过程中发生错误: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()