#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
PostgreSQL 数组字段示例
演示在 PostgreSQL 中使用 array_field、dict_field、list_field 等复杂字段类型
PostgreSQL 支持原生数组类型，提供强大的数组操作功能
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


def create_student_model() -> Dict[str, Any]:
    """
    创建学生模型，演示 PostgreSQL 中的数组字段使用
    PostgreSQL 支持原生数组类型，性能优异
    """
    print("\n=== 创建学生模型 (PostgreSQL 原生数组) ===")
    
    # 基础字段
    id_field = string_field(
        required=True,
        unique=True,
        description="学生ID"
    )
    
    name_field = string_field(
        required=True,
        max_length=100,
        description="学生姓名"
    )
    
    age_field = integer_field(
        required=True,
        min_value=6,
        max_value=25,
        description="学生年龄"
    )
    
    # 数组字段 - PostgreSQL 原生数组支持
    print("\n--- 数组字段定义 (PostgreSQL 原生数组) ---")
    
    # 分数数组 - 存储多门课程分数
    scores_field = array_field(
        item_type=FieldType.float(min_value=None, max_value=None),
        required=False,
        description="课程分数数组 - PostgreSQL REAL[] 类型"
    )
    print(f"分数数组字段: {scores_field.description}")
    
    # 等级数组 - 存储各科等级
    grades_field = array_field(
        item_type=FieldType.string(max_length=None, min_length=None),
        required=False,
        description="课程等级数组 - PostgreSQL TEXT[] 类型"
    )
    print(f"等级数组字段: {grades_field.description}")
    
    # 活跃状态数组 - 存储每月活跃状态
    is_active_field = array_field(
        item_type=FieldType.boolean(),
        required=False,
        description="月度活跃状态数组 - PostgreSQL BOOLEAN[] 类型"
    )
    print(f"活跃状态数组字段: {is_active_field.description}")
    
    # 标签数组 - 存储学生标签
    tags_field = array_field(
        item_type=FieldType.string(max_length=None, min_length=None),
        required=False,
        description="学生标签数组 - PostgreSQL TEXT[] 类型"
    )
    print(f"标签数组字段: {tags_field.description}")
    
    # 整数数组 - 存储课程ID
    course_ids_field = array_field(
        item_type=FieldType.integer(min_value=None, max_value=None),
        required=False,
        description="课程ID数组 - PostgreSQL INTEGER[] 类型"
    )
    print(f"课程ID数组字段: {course_ids_field.description}")
    
    # 爱好列表 - 混合类型数据（使用 JSONB）
    hobbies_field = list_field(
        item_type=FieldType.string(max_length=None, min_length=None),
        required=False,
        description="爱好列表 - PostgreSQL JSONB 存储混合类型"
    )
    print(f"爱好列表字段: {hobbies_field.description}")
    
    # 元数据字典 - 嵌套结构（使用 JSONB）
    metadata_fields = {
        "class_name": string_field(required=True, description="班级名称"),
        "teacher_id": integer_field(required=True, description="教师ID"),
        "semester_gpa": float_field(required=False, min_value=0.0, max_value=4.0, description="学期GPA"),
        "is_scholarship": boolean_field(required=False, description="是否获得奖学金")
    }
    metadata_field = dict_field(
        fields=metadata_fields,
        required=False,
        description="学生元数据 - PostgreSQL JSONB 存储嵌套对象"
    )
    print(f"元数据字典字段: {metadata_field.description}")
    
    # 自由格式 JSONB 字段
    extra_info_field = json_field(
        required=False,
        description="额外信息 - PostgreSQL JSONB 存储自由格式数据"
    )
    print(f"额外信息字段: {extra_info_field.description}")
    
    print("\n--- PostgreSQL 数组字段存储特点 ---")
    print("1. 原生数组类型支持：INTEGER[], TEXT[], REAL[], BOOLEAN[]")
    print("2. 高效的数组操作符：@>, <@, &&, ||")
    print("3. 数组函数：array_length(), array_append(), array_remove()")
    print("4. GIN 索引支持，快速数组元素查询")
    print("5. JSONB 类型支持复杂嵌套结构")
    print("6. 支持数组切片和多维数组")
    
    return {
        'id': id_field,
        'name': name_field,
        'age': age_field,
        'scores': scores_field,
        'grades': grades_field,
        'is_active': is_active_field,
        'tags': tags_field,
        'course_ids': course_ids_field,
        'hobbies': hobbies_field,
        'metadata': metadata_field,
        'extra_info': extra_info_field
    }


def create_student_indexes() -> List[IndexDefinition]:
    """
    创建学生模型的索引，包括 PostgreSQL 数组和 JSONB 索引
    """
    print("\n=== 创建 PostgreSQL 索引 (数组和 JSONB 索引) ===")
    
    indexes = []
    
    # 基础字段索引
    id_index = IndexDefinition(
        name="idx_student_id",
        columns=["id"],
        index_type=None,
        unique=True
    )
    indexes.append(id_index)
    print("创建唯一ID索引")
    
    # 复合索引
    name_age_index = IndexDefinition(
        name="idx_student_name_age",
        columns=["name", "age"],
        index_type=None,
        unique=False
    )
    indexes.append(name_age_index)
    print("创建姓名-年龄复合索引")
    
    # PostgreSQL 数组和 JSONB 索引说明
    print("\n--- PostgreSQL 数组和 JSONB 索引说明 ---")
    print("1. GIN 索引用于数组字段:")
    print("   CREATE INDEX idx_tags_gin ON students USING GIN (tags);")
    print("   CREATE INDEX idx_scores_gin ON students USING GIN (scores);")
    
    print("\n2. JSONB 字段索引:")
    print("   CREATE INDEX idx_metadata_gin ON students USING GIN (metadata);")
    print("   CREATE INDEX idx_metadata_class ON students ((metadata->>'class_name'));")
    
    print("\n3. 数组元素索引:")
    print("   CREATE INDEX idx_course_ids_gin ON students USING GIN (course_ids);")
    
    print("\n4. 表达式索引:")
    print("   CREATE INDEX idx_scores_avg ON students ((array_avg(scores)));")
    print("   CREATE INDEX idx_tags_count ON students ((array_length(tags, 1)));")
    
    return indexes


def demonstrate_postgresql_array_operations():
    """
    演示 PostgreSQL 数组字段的操作
    """
    print("\n=== PostgreSQL 数组字段操作演示 ===")
    
    # 创建数据库桥接器
    try:
        bridge = create_db_queue_bridge()
        print("数据库桥接器创建成功")
        
        # 添加 PostgreSQL 数据库连接
        # 使用远程 PostgreSQL 服务器配置 (来自 pgsql_cache_performance_comparison.py)
        result = bridge.add_postgresql_database(
            alias="pgsql_array_test",
            host="172.16.0.23",  # 远程 PostgreSQL 服务器
            port=5432,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            ssl_mode="prefer",
            max_connections=10,
            min_connections=2,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=1800
        )
        print(f"PostgreSQL 数据库连接结果: {result}")
        
        # 设置默认数据库
        bridge.set_default_alias("pgsql_array_test")
        
        # 示例数据 - 展示 PostgreSQL 原生数组存储
        sample_data = {
            "id": "student_001",
            "name": "李四",
            "age": 21,
            "scores": [88.5, 95.0, 82.5, 91.0],  # REAL[] 数组
            "grades": ["A-", "A+", "B+", "A"],    # TEXT[] 数组
            "is_active": [True, True, False, True, True],  # BOOLEAN[] 数组
            "tags": ["学习委员", "编程高手", "算法竞赛"],  # TEXT[] 数组
            "course_ids": [101, 102, 103, 104],  # INTEGER[] 数组
            "hobbies": ["游泳", "编程", 2, False],  # JSONB 混合类型
            "metadata": {  # JSONB 嵌套对象
                "class_name": "软件工程2021级2班",
                "teacher_id": 1002,
                "semester_gpa": 3.85,
                "is_scholarship": True
            },
            "extra_info": {  # JSONB 自由格式
                "emergency_contact": "139****5678",
                "dietary_restrictions": ["no_spicy"],
                "achievements": [
                    {"name": "ACM竞赛银奖", "year": 2023, "level": "regional"},
                    {"name": "优秀学生干部", "year": 2023, "level": "university"}
                ],
                "skills": {
                    "programming": ["Python", "Rust", "JavaScript"],
                    "languages": ["Chinese", "English", "Japanese"],
                    "certifications": ["CET-6", "TOEFL-100"]
                }
            }
        }
        
        print("\n--- 示例数据结构 ---")
        print(json.dumps(sample_data, ensure_ascii=False, indent=2))
        
        # 创建记录
        print("\n--- 创建学生记录 ---")
        create_result = bridge.create(
            table="students",
            data_json=json.dumps(sample_data),
            alias="pgsql_array_test"
        )
        print(f"创建结果: {create_result}")
        
        # 查询记录
        print("\n--- 查询学生记录 ---")
        query_conditions = [
            {
                "field": "id",
                "operator": "eq",
                "value": "student_001"
            }
        ]
        find_result = bridge.find(
            table="students",
            query_json=json.dumps(query_conditions),
            alias="pgsql_array_test"
        )
        print(f"查询结果: {find_result}")
        
        print("\n--- PostgreSQL 数组查询示例 ---")
        print("1. 数组包含查询: SELECT * FROM students WHERE tags @> ARRAY['编程高手'];")
        print("2. 数组元素查询: SELECT * FROM students WHERE 'A+' = ANY(grades);")
        print("3. 数组长度查询: SELECT name, array_length(scores, 1) FROM students;")
        print("4. 数组切片查询: SELECT name, scores[1:2] FROM students;")
        print("5. 数组聚合查询: SELECT name, array_avg(scores) FROM students;")
        
        print("\n--- PostgreSQL JSONB 查询示例 ---")
        print("6. JSONB 路径查询: SELECT metadata->>'class_name' FROM students;")
        print("7. JSONB 包含查询: SELECT * FROM students WHERE metadata @> '{\"is_scholarship\": true}';")
        print("8. JSONB 数组查询: SELECT * FROM students WHERE extra_info->'skills'->'programming' ? 'Python';")
        print("9. JSONB 深度查询: SELECT * FROM students WHERE extra_info #> '{achievements,0,level}' = '\"regional\"';")
        print("10. 组合查询: SELECT * FROM students WHERE tags @> ARRAY['编程高手'] AND metadata->>'is_scholarship' = 'true';")
        
        print("\n--- PostgreSQL 数组操作示例 ---")
        print("11. 数组追加: UPDATE students SET tags = array_append(tags, '新标签');")
        print("12. 数组删除: UPDATE students SET tags = array_remove(tags, '旧标签');")
        print("13. 数组连接: UPDATE students SET scores = scores || ARRAY[95.5];")
        print("14. JSONB 更新: UPDATE students SET metadata = metadata || '{\"new_field\": \"value\"}';")
        print("15. JSONB 删除: UPDATE students SET metadata = metadata - 'field_to_remove';")
        
    except Exception as e:
        print(f"PostgreSQL 操作演示失败: {e}")
        print("注意：需要确保 PostgreSQL 服务器可访问且配置正确")


def demonstrate_postgresql_performance():
    """
    演示 PostgreSQL 数组字段的性能特点
    """
    print("\n=== PostgreSQL 数组字段性能特点 ===")
    
    print("\n--- 性能优势 ---")
    print("1. 原生数组类型，无序列化开销")
    print("2. GIN 索引支持，O(log n) 查询复杂度")
    print("3. 向量化操作，批量处理效率高")
    print("4. JSONB 二进制格式，解析速度快")
    print("5. 支持并行查询和聚合")
    
    print("\n--- 存储效率 ---")
    print("1. 数组元素紧密存储，空间利用率高")
    print("2. JSONB 压缩存储，节省磁盘空间")
    print("3. 支持 TOAST 大对象存储")
    print("4. 列式存储扩展支持")
    
    print("\n--- 查询优化 ---")
    print("1. 数组操作符优化：@>, <@, &&")
    print("2. JSONB 路径索引优化")
    print("3. 部分索引支持")
    print("4. 表达式索引支持")
    print("5. 统计信息收集优化")


def cleanup_existing_tables():
    """清理现有的测试表"""
    print("🧹 清理现有的测试表...")
    try:
        # 创建临时桥接器进行清理
        temp_bridge = create_db_queue_bridge()
        
        # 添加PostgreSQL数据库连接
        result = temp_bridge.add_postgresql_database(
            alias="pgsql_cleanup",
            host="172.16.0.23",
            port=5432,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            ssl_mode="prefer",
            max_connections=5,
            min_connections=1,
            connection_timeout=10,
            idle_timeout=300,
            max_lifetime=600
        )
        
        result_data = json.loads(result)
        if result_data.get("success"):
            # 删除测试表中的数据
            tables_to_clean = ["students", "test_students", "student_array_test"]
            for table in tables_to_clean:
                try:
                    temp_bridge.drop_table(table, "pgsql_cleanup")
                    print(f"✅ 已清理表: {table}")
                except Exception as e:
                    print(f"⚠️ 清理表 {table} 时出错: {e}")
        else:
            print(f"⚠️ 无法连接到PostgreSQL进行清理: {result_data.get('error')}")
            
    except Exception as e:
        print(f"⚠️ 清理过程中出错: {e}")


def main():
    """
    主函数 - PostgreSQL 数组字段完整演示
    """
    print("=== PostgreSQL 数组字段示例程序 ===")
    print("演示在 PostgreSQL 中使用原生数组和 JSONB 类型")
    
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
            description="学生模型 - PostgreSQL 原生数组演示"
        )
        
        print(f"\n模型元数据创建完成:")
        print(f"  表名: {model_meta.collection_name}")
        print(f"  描述: {model_meta.description}")
        print(f"  字段数量: {len(fields)}")
        print(f"  索引数量: {len(indexes)}")
        
        # 演示数据库操作
        demonstrate_postgresql_array_operations()
        
        # 演示性能特点
        demonstrate_postgresql_performance()
        
        print("\n=== PostgreSQL 数组字段总结 ===")
        print("✓ 成功演示了 PostgreSQL 原生数组字段使用")
        print("✓ 展示了强大的数组操作符和函数")
        print("✓ 说明了 JSONB 类型的灵活性")
        print("✓ 提供了索引优化和性能调优建议")
        print("✓ PostgreSQL 是处理数组数据的最佳选择之一")
        
    except KeyboardInterrupt:
        print("\n程序被用户中断")
    except Exception as e:
        print(f"\n程序执行出错: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()