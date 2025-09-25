#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MySQL 数组字段示例
演示在 MySQL 中使用 array_field、dict_field、list_field 等复杂字段类型
MySQL 使用 JSON 格式存储数组和复杂数据结构
"""

import sys
import os
import time
import json
from typing import Dict, Any, List

# 添加项目根目录到 Python 路径
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../'))

try:
    import rat_quickdb_py
    from rat_quickdb_py import (
        # 字段创建函数
        string_field,
        integer_field,
        float_field,
        boolean_field,
        datetime_field,
        uuid_field,
        array_field,
        list_field,
        dict_field,
        json_field,
        # 类型定义
        FieldDefinition,
        FieldType,
        IndexDefinition,
        ModelMeta,
        # 数据库桥接器
        create_db_queue_bridge,
    )
except ImportError as e:
    print(f"导入 rat_quickdb_py 失败: {e}")
    print("请确保已正确安装 rat_quickdb_py 模块")
    sys.exit(1)


def cleanup_existing_tables():
    """清理现有的测试表"""
    print("🧹 清理现有的MySQL测试表...")
    try:
        # 创建临时桥接器进行清理
        temp_bridge = create_db_queue_bridge()
        
        # 添加MySQL数据库连接
        result = temp_bridge.add_mysql_database(
            alias="mysql_cleanup",
            host="172.16.0.21",
            port=3306,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            max_connections=5,
            min_connections=1,
            connection_timeout=10,
            idle_timeout=300,
            max_lifetime=600
        )
        
        print(f"MySQL连接结果: {result}")
        print(f"结果类型: {type(result)}")
        
        try:
            result_data = json.loads(result)
            if result_data.get("success"):
                print("✅ MySQL连接成功，开始清理表...")
                # 删除测试表结构
                tables_to_clean = ["students", "test_students", "student_array_test"]
                for table in tables_to_clean:
                    try:
                        drop_result = temp_bridge.drop_table(table, "mysql_cleanup")
                        print(f"✅ 已删除表: {table}, 结果: {drop_result}")
                    except Exception as e:
                        print(f"⚠️ 删除表 {table} 时出错: {e}")
            else:
                print(f"⚠️ 无法连接到MySQL进行清理: {result_data.get('error')}")
        except json.JSONDecodeError as e:
            print(f"⚠️ 解析MySQL连接结果失败: {e}")
            print(f"原始结果: {repr(result)}")
            # 如果JSON解析失败，但结果包含成功信息，仍然尝试清理
            if "成功" in result or "success" in result.lower():
                print("尝试基于字符串匹配进行清理...")
                tables_to_clean = ["students", "test_students", "student_array_test"]
                for table in tables_to_clean:
                    try:
                        drop_result = temp_bridge.drop_table(table, "mysql_cleanup")
                        print(f"✅ 已删除表: {table}, 结果: {drop_result}")
                    except Exception as e:
                        print(f"⚠️ 删除表 {table} 时出错: {e}")
            
    except Exception as e:
        print(f"⚠️ 清理过程中出错: {e}")
    
    print("清理完成")


def create_student_model() -> Dict[str, Any]:
    """
    创建学生模型，演示 MySQL 中的数组字段使用
    MySQL 将数组字段存储为 JSON 格式
    """
    print("\n=== 创建学生模型 (MySQL JSON 存储) ===")
    
    # 基础字段
    id_field = integer_field(
        required=True,
        unique=True,
        min_value=None,
        max_value=None,
        description="学生ID（主键）"
    )
    
    name_field = string_field(
        required=True,
        unique=None,
        max_length=100,
        min_length=None,
        description="学生姓名"
    )

    age_field = integer_field(
        required=True,
        unique=None,
        min_value=6,
        max_value=25,
        description="学生年龄"
    )
    
    # 数组字段 - MySQL 使用 JSON 存储
    print("\n--- 数组字段定义 (MySQL JSON 存储) ---")
    
    # 分数数组 - 存储多门课程分数
    scores_field = array_field(
        item_type=FieldType.float(),
        required=False,
        description="课程分数数组 - MySQL JSON存储"
    )
    print(f"分数数组字段: {scores_field.description}")
    
    # 等级数组 - 存储各科等级
    grades_field = array_field(
        item_type=FieldType.string(),
        required=False,
        description="课程等级数组 - MySQL JSON存储"
    )
    print(f"等级数组字段: {grades_field.description}")
    
    # 活跃状态数组 - 存储每月活跃状态
    is_active_field = array_field(
        item_type=FieldType.boolean(),
        required=False,
        description="月度活跃状态数组 - MySQL JSON存储"
    )
    print(f"活跃状态数组字段: {is_active_field.description}")
    
    # 标签数组 - 存储学生标签
    tags_field = array_field(
        item_type=FieldType.string(),
        required=False,
        description="学生标签数组 - MySQL JSON存储"
    )
    print(f"标签数组字段: {tags_field.description}")
    
    # 爱好列表 - 混合类型数据
    hobbies_field = list_field(
        item_type=FieldType.string(),
        required=False,
        description="爱好列表 - MySQL JSON存储混合类型"
    )
    print(f"爱好列表字段: {hobbies_field.description}")
    
    # 元数据字典 - 嵌套结构
    metadata_fields = {
        "class_name": string_field(required=True, unique=None, max_length=None, min_length=None, description="班级名称"),
        "teacher_id": integer_field(required=True, unique=None, min_value=None, max_value=None, description="教师ID"),
        "semester_gpa": float_field(required=False, unique=None, min_value=0.0, max_value=4.0, description="学期GPA"),
        "is_scholarship": boolean_field(required=False, description="是否获得奖学金")
    }
    metadata_field = dict_field(
        fields=metadata_fields,
        required=False,
        description="学生元数据 - MySQL JSON存储嵌套对象"
    )
    print(f"元数据字典字段: {metadata_field.description}")
    
    # 自由格式 JSON 字段
    extra_info_field = json_field(
        required=False,
        description="额外信息 - MySQL JSON存储自由格式数据"
    )
    print(f"额外信息字段: {extra_info_field.description}")
    
    print("\n--- MySQL 数组字段存储特点 ---")
    print("1. 所有数组和复杂字段都存储为 JSON 格式")
    print("2. 支持 JSON 函数进行查询和操作")
    print("3. 可以使用 JSON_EXTRACT 等函数访问数组元素")
    print("4. 支持 JSON 索引提高查询性能")
    print("5. 兼容性好，适合复杂数据结构存储")
    
    return {
        'id': id_field,
        'name': name_field,
        'age': age_field,
        'scores': scores_field,
        'grades': grades_field,
        'is_active': is_active_field,
        'tags': tags_field,
        'hobbies': hobbies_field,
        'metadata': metadata_field,
        'extra_info': extra_info_field
    }


def create_student_indexes() -> List[IndexDefinition]:
    """
    创建学生模型的索引，包括 JSON 字段索引
    """
    print("\n=== 创建 MySQL 索引 (包括 JSON 字段索引) ===")
    
    indexes = []
    
    # 基础字段索引
    id_index = IndexDefinition(
        fields=["id"],
        unique=True,
        name="idx_student_id"
    )
    indexes.append(id_index)
    print("创建唯一ID索引")
    
    # 复合索引
    name_age_index = IndexDefinition(
        fields=["name", "age"],
        unique=False,
        name="idx_student_name_age"
    )
    indexes.append(name_age_index)
    print("创建姓名-年龄复合索引")
    
    # JSON 字段索引 (MySQL 5.7+ 支持)
    # 注意：实际的 JSON 索引创建需要在数据库层面进行
    print("\n--- MySQL JSON 索引说明 ---")
    print("1. 可以为 JSON 字段的特定路径创建索引")
    print("2. 例如: CREATE INDEX idx_scores ON students ((CAST(scores->'$[0]' AS DECIMAL(5,2))))")
    print("3. 例如: CREATE INDEX idx_metadata_class ON students ((metadata->>'$.class_name'))")
    print("4. JSON 索引可以显著提高复杂查询性能")
    
    return indexes


def demonstrate_mysql_array_operations():
    """
    演示 MySQL 数组字段的操作
    """
    print("\n=== MySQL 数组字段操作演示 ===")
    
    # 创建数据库桥接器
    try:
        bridge = create_db_queue_bridge()
        print("数据库桥接器创建成功")
        
        # 添加 MySQL 数据库连接
        # 使用远程 MySQL 服务器配置 (来自 mysql_cache_performance_comparison.py)
        result = bridge.add_mysql_database(
            alias="mysql_array_test",
            host="172.16.0.21",  # 远程 MySQL 服务器
            port=3306,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            max_connections=10,
            min_connections=2,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=1800
        )
        print(f"MySQL 数据库连接结果: {result}")
        
        # 设置默认数据库
        bridge.set_default_alias("mysql_array_test")
        
        # 创建表结构 - 使用之前定义的模型
        print("\n--- 创建表结构 ---")
        fields = create_student_model()
        
        # 将FieldDefinition对象转换为可序列化的字典
        def convert_field_definition_to_json(field_def):
            """将FieldDefinition对象转换为JSON可序列化的格式"""
            # 获取字段类型的字符串表示
            field_repr = str(field_def)
            
            # 解析field_type部分
            if "field_type: String" in field_repr:
                return "string"
            elif "field_type: Integer" in field_repr:
                return "integer"
            elif "field_type: Float" in field_repr:
                return "float"
            elif "field_type: Boolean" in field_repr:
                return "boolean"
            elif "field_type: DateTime" in field_repr:
                return "datetime"
            elif "field_type: Uuid" in field_repr:
                return "uuid"
            elif "field_type: Json" in field_repr:
                return "json"
            elif "field_type: Array" in field_repr:
                # 解析数组的item_type
                if "item_type: String" in field_repr:
                    item_type = "string"
                elif "item_type: Integer" in field_repr:
                    item_type = "integer"
                elif "item_type: Float" in field_repr:
                    item_type = "float"
                elif "item_type: Boolean" in field_repr:
                    item_type = "boolean"
                else:
                    item_type = "string"
                
                return {
                    "type": "array",
                    "item_type": item_type
                }
            elif "field_type: Object" in field_repr:
                return "json"  # Object类型在MySQL中存储为JSON
            else:
                # 默认返回字符串类型
                return "string"
        
        serializable_fields = {}
        for field_name, field_def in fields.items():
            serializable_fields[field_name] = convert_field_definition_to_json(field_def)
        
        fields_json = json.dumps(serializable_fields)
        create_table_result = bridge.create_table(
            table="students",
            fields_json=fields_json,
            alias="mysql_array_test"
        )
        print(f"创建表结果: {create_table_result}")
        
        # 示例数据 - 展示 MySQL JSON 存储的数组数据
        sample_data = {
            "name": "张三",
            "age": 20,
            "scores": [85.5, 92.0, 78.5, 88.0],  # 浮点数数组
            "grades": ["A", "A+", "B+", "A-"],    # 字符串数组
            "is_active": [True, True, False, True, True],  # 布尔数组
            "tags": ["优秀学生", "班长", "数学竞赛"],  # 标签数组
            "hobbies": ["篮球", "编程", 3, True],  # 混合类型列表
            "metadata": {  # 嵌套对象
                "class_name": "计算机科学2021级1班",
                "teacher_id": 1001,
                "semester_gpa": 3.75,
                "is_scholarship": True
            },
            "extra_info": {  # 自由格式 JSON
                "emergency_contact": "138****1234",
                "dietary_restrictions": ["vegetarian"],
                "awards": [
                    {"name": "优秀学生", "year": 2023},
                    {"name": "数学竞赛二等奖", "year": 2023}
                ]
            }
        }
        
        print("\n--- 示例数据结构 ---")
        print(json.dumps(sample_data, ensure_ascii=False, indent=2))
        
        # 创建记录
        print("\n--- 创建学生记录 ---")
        create_result = bridge.create(
            table="students",
            data_json=json.dumps(sample_data),
            alias="mysql_array_test"
        )
        print(f"创建结果: {create_result}")
        
        # 查询记录
        print("\n--- 查询学生记录 ---")
        find_result = bridge.find(
            table="students",
            query_json="{}",
            alias="mysql_array_test"
        )
        print(f"查询结果: {find_result}")
        
        print("\n--- MySQL JSON 查询示例 ---")
        print("1. 查询分数数组第一个元素: SELECT scores->'$[0]' FROM students")
        print("2. 查询班级名称: SELECT metadata->>'$.class_name' FROM students")
        print("3. 查询包含特定标签的学生: SELECT * FROM students WHERE JSON_CONTAINS(tags, '\"优秀学生\"')")
        print("4. 查询分数数组长度: SELECT JSON_LENGTH(scores) FROM students")
        print("5. 更新数组元素: UPDATE students SET scores = JSON_SET(scores, '$[0]', 90.0)")
        
    except Exception as e:
        print(f"MySQL 操作演示失败: {e}")
        print("注意：需要确保 MySQL 服务器可访问且配置正确")


def main():
    """
    主函数 - MySQL 数组字段完整演示
    """
    print("=== MySQL 数组字段示例程序 ===")
    print("演示在 MySQL 中使用复杂数据类型（JSON 存储）")
    
    # 清理现有的测试表
    cleanup_existing_tables()
    
    try:
        # 创建模型字段
        fields = create_student_model()
        print(f"\n创建了 {len(fields)} 个字段")
        
        # 创建模型索引
        indexes = create_student_indexes()
        print(f"创建了 {len(indexes)} 个索引")
        
        # 创建模型元数据
        model_meta = ModelMeta(
            collection_name="students",
            fields=fields,
            indexes=indexes,
            database_alias=None,
            description="学生模型 - MySQL JSON 存储演示"
        )
        
        print(f"\n模型元数据创建完成:")
        print(f"  表名: {model_meta.collection_name}")
        print(f"  描述: {model_meta.description}")
        print(f"  字段数量: {len(fields)}")
        print(f"  索引数量: {len(indexes)}")
        
        # 演示数据库操作
        demonstrate_mysql_array_operations()
        
        print("\n=== MySQL 数组字段总结 ===")
        print("✓ 成功演示了 MySQL 中的数组字段使用")
        print("✓ 展示了 JSON 格式存储复杂数据结构")
        print("✓ 说明了 MySQL JSON 函数的使用方法")
        print("✓ 提供了索引优化建议")
        
    except KeyboardInterrupt:
        print("\n程序被用户中断")
    except Exception as e:
        print(f"\n程序执行出错: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()