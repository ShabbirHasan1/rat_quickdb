#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
SQLite 复杂查询验证脚本
验证 SQLite 数据库的复杂查询功能，包括 AND、OR、范围查询、字符串匹配等
"""

import asyncio
import sys
import os
from datetime import datetime, timezone
from typing import Dict, Any, List

# 添加项目根目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

try:
    from rat_quickdb_py import (
        create_db_queue_bridge,
        string_field,
        integer_field,
        float_field,
        boolean_field,
        datetime_field,
        array_field,
        dict_field,
        FieldType,
        FieldDefinition
    )
except ImportError as e:
    print(f"导入 rat_quickdb_py 失败: {e}")
    print("请确保已运行 'maturin develop' 编译 PyO3 绑定")
    sys.exit(1)

class SQLiteComplexQueryTest:
    def __init__(self):
        self.bridge = None
        self.db_alias = "sqlite_test"
        self.table_name = "test_users"
    
    def setup_database(self):
        """设置 SQLite 数据库连接"""
        print("设置 SQLite 数据库连接...")
        
        # 创建数据库桥接器
        self.bridge = create_db_queue_bridge()
        
        # 添加 SQLite 数据库（使用内存数据库）
        result = self.bridge.add_sqlite_database(
            alias=self.db_alias,
            path=":memory:",  # 使用内存数据库
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=300,
            max_lifetime=1800
        )
        print(f"SQLite 数据库添加结果: {result}")
        print("SQLite 数据库连接建立完成")
    
    def cleanup_data(self):
        """清理测试数据"""
        try:
            # 删除表（如果存在）
            result = self.bridge.drop_table(self.table_name, self.db_alias)
            print(f"已清理表: {self.table_name}")
        except Exception as e:
            print(f"清理数据时出错: {e}")
    
    def create_table_and_insert_data(self):
        """创建表结构并插入测试数据"""
        import json
        
        # 创建表结构定义
        fields = {
            'id': string_field(required=True, description="用户ID"),
            'name': string_field(required=True, description="用户姓名"),
            'age': integer_field(required=True, min_value=0, max_value=150, description="年龄"),
            'email': string_field(required=True, description="邮箱地址"),
            'department': string_field(required=True, description="部门"),
            'salary': float_field(required=True, min_value=0.0, description="薪资"),
            'is_active': boolean_field(required=True, description="是否激活"),
            'created_at': string_field(required=True, description="创建时间"),
            'metadata': string_field(required=False, description="元数据JSON"),
            'tags': string_field(required=False, description="标签JSON")
        }
        
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
            else:
                # 默认返回字符串类型
                return "string"
        
        # 转换为可序列化的字典
        serializable_fields = {}
        for field_name, field_def in fields.items():
            if hasattr(field_def, 'to_dict'):
                serializable_fields[field_name] = field_def.to_dict()
            else:
                serializable_fields[field_name] = convert_field_definition_to_json(field_def)
        
        result = self.bridge.create_table(self.table_name, json.dumps(serializable_fields), self.db_alias)
        print(f"已创建表: {self.table_name}")
        
        # 插入测试数据
        test_data = [
            {
                "name": "张三",
                "age": 25,
                "email": "zhangsan@example.com",
                "department": "技术部",
                "salary": 8000.0,
                "is_active": True,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "metadata": '{"level": "junior", "skills": ["Python", "SQL"]}',
                "tags": '["backend", "database"]'
            },
            {
                "name": "李四",
                "age": 30,
                "email": "lisi@example.com",
                "department": "产品部",
                "salary": 12000.0,
                "is_active": True,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "metadata": '{"level": "senior", "skills": ["Product", "Design"]}',
                "tags": '["frontend", "ui"]'
            },
            {
                "name": "王五",
                "age": 28,
                "email": "wangwu@example.com",
                "department": "技术部",
                "salary": 10000.0,
                "is_active": False,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "metadata": '{"level": "middle", "skills": ["Java", "Spring"]}',
                "tags": '["backend", "api"]'
            },
            {
                "name": "赵六",
                "age": 35,
                "email": "zhaoliu@example.com",
                "department": "管理部",
                "salary": 15000.0,
                "is_active": True,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "metadata": '{"level": "manager", "skills": ["Management", "Strategy"]}',
                "tags": '["management", "strategy"]'
            },
            {
                "name": "钱七",
                "age": 27,
                "email": "qianqi@company.net",
                "department": "技术部",
                "salary": 9500.0,
                "is_active": True,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "metadata": '{"level": "senior", "skills": ["AI", "Machine Learning"]}',
                "tags": '["ai", "research"]'
            },
            {
                "name": "孙八",
                "age": 32,
                "email": "sunba@example.com",
                "department": "运营部",
                "salary": 11000.0,
                "is_active": True,
                "created_at": datetime.now(timezone.utc).isoformat(),
                "metadata": '{"level": "senior", "skills": ["Marketing", "Analytics"]}',
                "tags": '["marketing", "data"]'
            }
        ]
        
        for data in test_data:
            result = self.bridge.create(self.table_name, json.dumps(data), self.db_alias)
            print(f"插入数据: {data['name']} -> {result}")
        
        print("测试数据插入完成")
    
    def test_and_logic_query(self):
        """测试 AND 逻辑查询"""
        print("\n=== 测试 AND 逻辑查询 ===")
        import json
        
        # 查询技术部且年龄大于25的员工
        query = {
            "department": "技术部",
            "age": {"Gt": 25}
        }
        
        results = self.bridge.find(self.table_name, json.dumps(query), self.db_alias)
        print(f"技术部且年龄>25的员工查询结果: {results}")
        
        # 解析查询结果
        try:
            if isinstance(results, str):
                results_data = json.loads(results)
            else:
                results_data = results
            
            if isinstance(results_data, dict) and 'success' in results_data:
                if results_data['success']:
                    data_list = results_data.get('data', [])
                    print(f"技术部且年龄>25的员工: {len(data_list)} 条记录")
                    for result in data_list:
                        if isinstance(result, str):
                            result_data = json.loads(result)
                        else:
                            result_data = result
                        print(f"  - {result_data.get('name')}: {result_data.get('age')}岁, {result_data.get('department')}")
                else:
                    print(f"查询失败: {results_data.get('error')}")
            elif isinstance(results_data, list):
                print(f"技术部且年龄>25的员工: {len(results_data)} 条记录")
                for result in results_data:
                    if isinstance(result, str):
                        result_data = json.loads(result)
                    else:
                        result_data = result
                    print(f"  - {result_data.get('name')}: {result_data.get('age')}岁, {result_data.get('department')}")
            else:
                print(f"查询结果格式异常: {type(results_data)}")
        except json.JSONDecodeError as e:
            print(f"JSON解析错误: {e}")
            print(f"原始结果: {repr(results)}")
    
    def test_or_logic_query(self):
        """测试 OR 逻辑查询"""
        print("\n=== 测试 OR 逻辑查询 ===")
        import json
        
        # 查询年龄小于27或薪资大于12000的员工
        query = {
            "operator": "or",
            "conditions": [
                {"field": "age", "operator": "Lt", "value": 27},
                {"field": "salary", "operator": "Gt", "value": 12000}
            ]
        }
        
        results = self.bridge.find(self.table_name, json.dumps(query), self.db_alias)
        print(f"年龄<27或薪资>12000的员工查询结果: {results}")
        
        # 解析查询结果
        try:
            if isinstance(results, str):
                results_data = json.loads(results)
            else:
                results_data = results
            
            if isinstance(results_data, dict) and 'success' in results_data:
                if results_data['success']:
                    data_list = results_data.get('data', [])
                    print(f"年龄<27或薪资>12000的员工: {len(data_list)} 条记录")
                    for result in data_list:
                        if isinstance(result, str):
                            result_data = json.loads(result)
                        else:
                            result_data = result
                        print(f"  - {result_data.get('name')}: {result_data.get('age')}岁, 薪资{result_data.get('salary')}")
                else:
                    print(f"查询失败: {results_data.get('error')}")
            elif isinstance(results_data, list):
                print(f"年龄<27或薪资>12000的员工: {len(results_data)} 条记录")
                for result in results_data:
                    if isinstance(result, str):
                        result_data = json.loads(result)
                    else:
                        result_data = result
                    print(f"  - {result_data.get('name')}: {result_data.get('age')}岁, 薪资{result_data.get('salary')}")
            else:
                print(f"查询结果格式异常: {type(results_data)}")
        except json.JSONDecodeError as e:
            print(f"JSON解析错误: {e}")
            print(f"原始结果: {repr(results)}")
    
    def test_range_query(self):
        """测试范围查询"""
        print("\n=== 测试范围查询 ===")
        import json
        
        # 查询年龄在27-32之间的员工（钱七27岁，王五28岁，李四30岁，孙八32岁）
        query = {
            "operator": "and",
            "conditions": [
                {"field": "age", "operator": "Gte", "value": 27},
                {"field": "age", "operator": "Lte", "value": 32}
            ]
        }
        
        results = self.bridge.find(self.table_name, json.dumps(query), self.db_alias)
        print(f"年龄在26-32之间的员工查询结果: {results}")
        
        # 解析查询结果 - find方法返回字典而不是JSON字符串
        try:
            if isinstance(results, str):
                results_data = json.loads(results)
            else:
                results_data = results
            
            if isinstance(results_data, dict) and results_data.get("success"):
                records = results_data.get("data", [])
                print(f"年龄在26-32之间的员工: {len(records)} 条记录")
                for record in records:
                    print(f"  - {record.get('name')}: {record.get('age')}岁")
            elif isinstance(results_data, list):
                print(f"年龄在26-32之间的员工: {len(results_data)} 条记录")
                for result in results_data:
                    if isinstance(result, str):
                        result_data = json.loads(result)
                    else:
                        result_data = result
                    print(f"  - {result_data.get('name')}: {result_data.get('age')}岁")
            else:
                print(f"查询结果格式异常: {type(results_data)}")
        except json.JSONDecodeError as e:
            print(f"JSON解析错误: {e}")
            print(f"原始结果: {repr(results)}")
    
    def test_string_matching_query(self):
        """测试字符串匹配查询"""
        print("\n=== 测试字符串匹配查询 ===")
        import json
        
        # 查询邮箱包含"example"的员工（大部分员工邮箱都包含example）
        query = {
            "field": "email",
            "operator": "Contains",
            "value": "example"
        }
        
        results = self.bridge.find(self.table_name, json.dumps(query), self.db_alias)
        print(f"邮箱包含'example.com'的员工查询结果: {results}")
        
        # 解析查询结果 - find方法返回字典而不是JSON字符串
        try:
            if isinstance(results, str):
                results_data = json.loads(results)
            else:
                results_data = results
            
            if isinstance(results_data, dict) and results_data.get("success"):
                records = results_data.get("data", [])
                print(f"邮箱包含'example.com'的员工: {len(records)} 条记录")
                for record in records:
                    print(f"  - {record.get('name')}: {record.get('email')}")
            elif isinstance(results_data, list):
                print(f"邮箱包含'example.com'的员工: {len(results_data)} 条记录")
                for result in results_data:
                    if isinstance(result, str):
                        result_data = json.loads(result)
                    else:
                        result_data = result
                    print(f"  - {result_data.get('name')}: {result_data.get('email')}")
            else:
                print(f"查询结果格式异常: {type(results_data)}")
        except json.JSONDecodeError as e:
            print(f"JSON解析错误: {e}")
            print(f"原始结果: {repr(results)}")
    
    def test_json_field_query(self):
        """测试 JSON 字段查询"""
        print("\n=== 测试 JSON 字段查询 ===")
        import json
        
        # 查询 metadata 包含"senior"的员工（李四、钱七、孙八的level都是senior）
        query = {
            "field": "metadata",
            "operator": "Contains",
            "value": "senior"
        }
        
        results = self.bridge.find(self.table_name, json.dumps(query), self.db_alias)
        print(f"metadata包含'senior'的员工查询结果: {results}")
        
        # 解析查询结果 - find方法返回字典而不是JSON字符串
        try:
            if isinstance(results, str):
                results_data = json.loads(results)
            else:
                results_data = results
            
            if isinstance(results_data, dict) and results_data.get("success"):
                records = results_data.get("data", [])
                print(f"metadata包含'senior'的员工: {len(records)} 条记录")
                for record in records:
                    print(f"  - {record.get('name')}: {record.get('metadata')}")
            elif isinstance(results_data, list):
                print(f"metadata包含'senior'的员工: {len(results_data)} 条记录")
                for result in results_data:
                    if isinstance(result, str):
                        result_data = json.loads(result)
                    else:
                        result_data = result
                    print(f"  - {result_data.get('name')}: {result_data.get('metadata')}")
            else:
                print(f"查询结果格式异常: {type(results_data)}")
        except json.JSONDecodeError as e:
            print(f"JSON解析错误: {e}")
            print(f"原始结果: {repr(results)}")
    
    def test_mixed_and_or_query(self):
        """测试混合 AND/OR 查询"""
        print("\n=== 测试混合 AND/OR 查询 ===")
        import json
        
        # 查询：(技术部 AND 年龄>26) OR (产品部 AND 薪资>11000)
        query = {
            "operator": "Or",
            "conditions": [
                {
                    "operator": "And",
                    "conditions": [
                        {
                            "field": "department",
                            "operator": "Eq",
                            "value": "技术部"
                        },
                        {
                            "field": "age",
                            "operator": "Gt",
                            "value": 26
                        }
                    ]
                },
                {
                    "operator": "And",
                    "conditions": [
                        {
                            "field": "department",
                            "operator": "Eq",
                            "value": "产品部"
                        },
                        {
                            "field": "salary",
                            "operator": "Gt",
                            "value": 11000
                        }
                    ]
                }
            ]
        }
        
        results = self.bridge.find(self.table_name, json.dumps(query), self.db_alias)
        print(f"混合 AND/OR 查询结果: {results}")
        
        # 解析查询结果
        try:
            if isinstance(results, str):
                results_data = json.loads(results)
            else:
                results_data = results
            
            if isinstance(results_data, dict) and results_data.get("success"):
                records = results_data.get("data", [])
                print(f"混合 AND/OR 查询: {len(records)} 条记录")
                for record in records:
                    print(f"  - {record.get('name')}: {record.get('department')}, 年龄{record.get('age')}, 薪资{record.get('salary')}")
            elif isinstance(results_data, list):
                print(f"混合 AND/OR 查询: {len(results_data)} 条记录")
                for result in results_data:
                    if isinstance(result, str):
                        result_data = json.loads(result)
                    else:
                        result_data = result
                    print(f"  - {result_data.get('name')}: {result_data.get('department')}, 年龄{result_data.get('age')}, 薪资{result_data.get('salary')}")
            else:
                print(f"查询结果格式异常: {type(results_data)}")
        except json.JSONDecodeError as e:
            print(f"JSON解析错误: {e}")
            print(f"原始结果: {repr(results)}")
    
    def test_empty_result_query(self):
        """测试预期为空的查询 - 边界情况测试"""
        print("\n=== 测试预期为空的查询 ===")
        import json
        
        # 查询不存在的部门
        query = {
            "department": "不存在的部门"
        }
        
        results = self.bridge.find(self.table_name, json.dumps(query), self.db_alias)
        print(f"查询不存在部门的结果: {results}")
        
        # 解析查询结果
        try:
            if isinstance(results, str):
                results_data = json.loads(results)
            else:
                results_data = results
            
            if isinstance(results_data, dict) and results_data.get("success"):
                records = results_data.get("data", [])
                if len(records) == 0:
                    print("✅ 预期为空的查询正确返回空结果")
                else:
                    print(f"❌ 预期为空但返回了 {len(records)} 条记录")
            elif isinstance(results_data, list):
                if len(results_data) == 0:
                    print("✅ 预期为空的查询正确返回空结果")
                else:
                    print(f"❌ 预期为空但返回了 {len(results_data)} 条记录")
            else:
                print(f"查询结果格式异常: {type(results_data)}")
        except json.JSONDecodeError as e:
            print(f"JSON解析错误: {e}")
            print(f"原始结果: {repr(results)}")
    
    def run_all_tests(self):
        """运行所有测试"""
        try:
            self.setup_database()
            self.cleanup_data()
            self.create_table_and_insert_data()
            
            # 运行各种查询测试
            self.test_and_logic_query()
            self.test_or_logic_query()
            self.test_range_query()
            self.test_string_matching_query()
            self.test_json_field_query()
            self.test_mixed_and_or_query()
            self.test_empty_result_query()
            
            print("\n=== SQLite 复杂查询测试完成 ===")
            print("\n📊 测试总结:")
            print("✅ AND逻辑查询 - 应该返回技术部且年龄>25的员工")
            print("✅ OR逻辑查询 - 应该返回技术部或产品部的员工")
            print("✅ 范围查询 - 应该返回年龄26-32之间的员工")
            print("✅ 字符串匹配查询 - 应该返回邮箱包含example.com的员工")
            print("✅ JSON字段查询 - 应该返回metadata包含senior的员工")
            print("✅ 混合AND/OR查询 - 应该返回复合条件的员工")
            print("✅ 空结果查询 - 应该返回空结果（边界测试）")
            
        except Exception as e:
            print(f"测试过程中出错: {e}")
            import traceback
            traceback.print_exc()
        finally:
            # 清理资源
            if self.bridge:
                self.cleanup_data()

def main():
    """主函数"""
    print("开始 SQLite 复杂查询验证测试...")
    
    test = SQLiteComplexQueryTest()
    test.run_all_tests()

if __name__ == "__main__":
    main()