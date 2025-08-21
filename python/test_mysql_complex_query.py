#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MySQL 复杂查询验证脚本
基于 test_mongodb_complex_query.py 修改，验证 MySQL 数据库的复杂查询功能
包含 AND、OR、范围查询、字符串匹配等多种查询条件
"""

import sys
import os
import json
import time
from typing import Dict, Any, List

# 添加项目根目录到 Python 路径
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../'))

try:
    import rat_quickdb_py
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
        FieldDefinition,
        ModelMeta
    )
except ImportError as e:
    print(f"导入 rat_quickdb_py 失败: {e}")
    print("请确保已正确安装 rat_quickdb_py 模块")
    sys.exit(1)


class MySQLComplexQueryTest:
    """MySQL 复杂查询测试类"""
    
    def __init__(self):
        self.bridge = None
        self.table_name = "test_users"
        self.db_alias = "mysql_test"
    
    def setup_database(self):
        """设置 MySQL 数据库连接"""
        print("🔧 设置 MySQL 数据库连接...")
        
        try:
            # 创建数据库桥接器
            self.bridge = create_db_queue_bridge()
            print("✅ 数据库桥接器创建成功")
            
            # 添加 MySQL 数据库连接（使用 mysql_array_field_example.py 中的配置）
            result = self.bridge.add_mysql_database(
                alias=self.db_alias,
                host="172.16.0.21",
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
            self.bridge.set_default_alias(self.db_alias)
            print(f"✅ 设置默认数据库别名: {self.db_alias}")
            
        except Exception as e:
            print(f"❌ 数据库设置失败: {e}")
            raise
    
    def cleanup_existing_data(self):
        """清理现有测试数据"""
        print(f"🧹 清理现有测试数据...")
        try:
            # 删除表（如果存在）
            drop_result = self.bridge.drop_table(self.table_name, self.db_alias)
            print(f"删除表结果: {drop_result}")
        except Exception as e:
            print(f"⚠️ 删除表时出错（可能表不存在）: {e}")
    
    def create_table_schema(self):
        """创建测试表结构"""
        print(f"📋 创建表结构: {self.table_name}")
        
        # 定义字段 - 参考 mysql_cache_performance_comparison.py 的成功模式
        # 不显式定义 id 字段，让 MySQL 自动生成自增主键
        fields = {
            'name': string_field(required=True, description="用户姓名"),
            'age': integer_field(required=True, description="年龄"),
            'email': string_field(required=True, description="邮箱地址"),
            'score': float_field(required=True, description="分数"),
            'is_active': boolean_field(required=True, description="是否激活"),
            'department': string_field(required=True, description="部门"),
            'tags': array_field(
                item_type=FieldType.string(max_length=None, min_length=None),
                required=False,
                description="用户标签数组"
            ),
            'metadata': dict_field(
                fields={
                    "level": string_field(required=True, description="用户等级"),
                    "join_date": string_field(required=False, description="加入日期"),
                    "last_login": string_field(required=False, description="最后登录时间")
                },
                required=False,
                description="用户元数据"
            )
        }
        
        # 转换为可序列化的字典 - 参考 mysql_array_field_example.py 的成功模式
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
        
        # 创建表
        create_result = self.bridge.create_table(
            table=self.table_name,
            fields_json=json.dumps(serializable_fields),
            alias=self.db_alias
        )
        print(f"创建表结果: {create_result}")
    
    def insert_test_data(self):
        """插入测试数据"""
        print("📝 插入测试数据...")
        
        test_users = [
            {
                "name": "张三",
                "age": 25,
                "email": "zhangsan@example.com",
                "department": "技术部",
                "score": 85.5,
                "is_active": True,
                "tags": ["developer", "python"],
                "metadata": {"level": "junior", "join_date": "2024-01-15", "last_login": "2024-01-20"}
            },
            {
                "name": "李四",
                "age": 30,
                "email": "lisi@example.com",
                "department": "产品部",
                "score": 92.0,
                "is_active": True,
                "tags": ["product", "design"],
                "metadata": {"level": "senior", "join_date": "2024-01-10", "last_login": "2024-01-19"}
            },
            {
                "name": "王五",
                "age": 28,
                "email": "wangwu@example.com",
                "department": "技术部",
                "score": 78.5,
                "is_active": False,
                "tags": ["backend", "api"],
                "metadata": {"level": "middle", "join_date": "2024-01-05"}
            },
            {
                "name": "赵六",
                "age": 35,
                "email": "zhaoliu@example.com",
                "department": "管理部",
                "score": 95.0,
                "is_active": True,
                "tags": ["management", "strategy"],
                "metadata": {"level": "manager", "join_date": "2024-01-01", "last_login": "2024-01-18"}
            },
            {
                "name": "钱七",
                "age": 26,
                "email": "qianqi@company.net",
                "department": "技术部",
                "score": 88.0,
                "is_active": True,
                "tags": ["ai", "research"],
                "metadata": {"level": "senior", "join_date": "2023-12-20", "last_login": "2024-01-17"}
            }
        ]
        
        for user in test_users:
            try:
                result = self.bridge.create(
                    table=self.table_name,
                    data_json=json.dumps(user),
                    alias=self.db_alias
                )
                print(f"✅ 插入用户 {user['name']} 成功: {result}")
            except Exception as e:
                print(f"❌ 插入用户 {user['name']} 失败: {e}")
    
    def test_and_logic_query(self):
        """测试 AND 逻辑查询"""
        print("\n🔍 测试 AND 逻辑查询...")
        
        # 查询条件：技术部 AND 年龄大于25 AND 激活状态
        query = json.dumps({
            "operator": "and",
            "conditions": [
                {
                    "field": "department",
                    "operator": "Eq",
                    "value": "技术部"
                },
                {
                    "field": "age",
                    "operator": "Gt",
                    "value": 25
                },
                {
                    "field": "is_active",
                    "operator": "Eq",
                    "value": True
                }
            ]
        })
        
        try:
            result = self.bridge.find(
                table=self.table_name,
                query_json=query,
                alias=self.db_alias
            )
            print(f"AND 查询结果: {result}")
            
            # 解析结果 - find方法返回字典而不是JSON字符串
            if isinstance(result, str):
                result_data = json.loads(result)
            else:
                result_data = result
            if result_data.get("success"):
                records = result_data.get("data", [])
                print(f"✅ 找到 {len(records)} 条符合条件的记录")
                for record in records:
                    print(f"  - {record.get('name')} (年龄: {record.get('age')}, 部门: {record.get('department')})")
            else:
                print(f"❌ 查询失败: {result_data.get('error')}")
                
        except Exception as e:
            print(f"❌ AND 查询执行失败: {e}")
    
    def test_or_logic_query(self):
        """测试 OR 逻辑查询"""
        print("\n🔍 测试 OR 逻辑查询...")
        
        # 查询条件：分数大于90 OR 部门是产品部
        query = json.dumps({
            "operator": "or",
            "conditions": [
                {
                    "field": "score",
                    "operator": "Gt",
                    "value": 90.0
                },
                {
                    "field": "department",
                    "operator": "Eq",
                    "value": "产品部"
                }
            ]
        })
        
        try:
            result = self.bridge.find(
                table=self.table_name,
                query_json=query,
                alias=self.db_alias
            )
            print(f"OR 查询结果: {result}")
            
            # 解析结果 - find方法返回字典而不是JSON字符串
            if isinstance(result, str):
                result_data = json.loads(result)
            else:
                result_data = result
            if result_data.get("success"):
                records = result_data.get("data", [])
                print(f"✅ 找到 {len(records)} 条符合条件的记录")
                for record in records:
                    print(f"  - {record.get('name')} (分数: {record.get('score')}, 部门: {record.get('department')})")
            else:
                print(f"❌ 查询失败: {result_data.get('error')}")
                
        except Exception as e:
            print(f"❌ OR 查询执行失败: {e}")
    
    def test_range_query(self):
        """测试范围查询"""
        print("\n🔍 测试范围查询...")
        
        # 查询条件：年龄在26-30之间
        query = json.dumps({
            "operator": "and",
            "conditions": [
                {
                    "field": "age",
                    "operator": "Gte",
                    "value": 26
                },
                {
                    "field": "age",
                    "operator": "Lte",
                    "value": 30
                }
            ]
        })
        
        try:
            result = self.bridge.find(
                table=self.table_name,
                query_json=query,
                alias=self.db_alias
            )
            print(f"范围查询结果: {result}")
            
            # 解析结果 - find方法返回字典而不是JSON字符串
            if isinstance(result, str):
                result_data = json.loads(result)
            else:
                result_data = result
            if result_data.get("success"):
                records = result_data.get("data", [])
                print(f"✅ 找到 {len(records)} 条符合条件的记录")
                for record in records:
                    print(f"  - {record.get('name')} (年龄: {record.get('age')})")
            else:
                print(f"❌ 查询失败: {result_data.get('error')}")
                
        except Exception as e:
            print(f"❌ 范围查询执行失败: {e}")
    
    def test_string_pattern_query(self):
        """测试字符串模式查询"""
        print("\n🔍 测试字符串模式查询...")
        
        # 查询条件：邮箱包含 "example.com"
        query = json.dumps({
            "operator": "and",
            "conditions": [
                {
                    "field": "email",
                    "operator": "Contains",
                    "value": "example.com"
                }
            ]
        })
        
        try:
            result = self.bridge.find(
                table=self.table_name,
                query_json=query,
                alias=self.db_alias
            )
            print(f"字符串模式查询结果: {result}")
            
            # 解析结果 - find方法返回字典而不是JSON字符串
            if isinstance(result, str):
                result_data = json.loads(result)
            else:
                result_data = result
            if result_data.get("success"):
                records = result_data.get("data", [])
                print(f"✅ 找到 {len(records)} 条符合条件的记录")
                for record in records:
                    print(f"  - {record.get('name')} (邮箱: {record.get('email')})")
            else:
                print(f"❌ 查询失败: {result_data.get('error')}")
                
        except Exception as e:
            print(f"❌ 字符串模式查询执行失败: {e}")
    
    def test_array_query(self):
        """测试数组查询"""
        print("\n🔍 测试数组查询...")
        
        # 查询条件：标签包含 "Python"
        query = json.dumps({
            "operator": "and",
            "conditions": [
                {
                    "field": "tags",
                    "operator": "Contains",
                    "value": "Python"
                }
            ]
        })
        
        try:
            result = self.bridge.find(
                table=self.table_name,
                query_json=query,
                alias=self.db_alias
            )
            print(f"数组查询结果: {result}")
            
            # 解析结果 - find方法返回字典而不是JSON字符串
            if isinstance(result, str):
                result_data = json.loads(result)
            else:
                result_data = result
            if result_data.get("success"):
                records = result_data.get("data", [])
                print(f"✅ 找到 {len(records)} 条符合条件的记录")
                for record in records:
                    print(f"  - {record.get('name')} (标签: {record.get('tags')})")
            else:
                print(f"❌ 查询失败: {result_data.get('error')}")
                
        except Exception as e:
            print(f"❌ 数组查询执行失败: {e}")
    
    def test_mixed_and_or_query(self):
        """测试混合 AND/OR 查询"""
        print("\n🔍 测试混合 AND/OR 查询...")
        
        # 查询条件：(技术部 AND 激活状态) OR (分数大于90)
        query = json.dumps({
            "operator": "or",
            "conditions": [
                {
                    "operator": "and",
                    "conditions": [
                        {
                            "field": "department",
                            "operator": "Eq",
                            "value": "技术部"
                        },
                        {
                            "field": "is_active",
                            "operator": "Eq",
                            "value": True
                        }
                    ]
                },
                {
                    "field": "score",
                    "operator": "Gt",
                    "value": 90.0
                }
            ]
        })
        
        try:
            result = self.bridge.find(
                table=self.table_name,
                query_json=query,
                alias=self.db_alias
            )
            print(f"混合 AND/OR 查询结果: {result}")
            
            # 解析结果 - find方法返回字典而不是JSON字符串
            if isinstance(result, str):
                result_data = json.loads(result)
            else:
                result_data = result
            if result_data.get("success"):
                records = result_data.get("data", [])
                print(f"✅ 找到 {len(records)} 条符合条件的记录")
                for record in records:
                    print(f"  - {record.get('name')} (部门: {record.get('department')}, 分数: {record.get('score')}, 激活: {record.get('is_active')})")
            else:
                print(f"❌ 查询失败: {result_data.get('error')}")
                
        except Exception as e:
            print(f"❌ 混合 AND/OR 查询执行失败: {e}")
    
    def run_all_tests(self):
        """运行所有测试"""
        print("🚀 开始 MySQL 复杂查询验证测试...")
        
        try:
            # 设置数据库
            self.setup_database()
            
            # 清理现有数据
            self.cleanup_existing_data()
            
            # 创建表结构
            self.create_table_schema()
            
            # 插入测试数据
            self.insert_test_data()
            
            # 执行各种查询测试
            self.test_and_logic_query()
            self.test_or_logic_query()
            self.test_range_query()
            self.test_string_pattern_query()
            self.test_array_query()
            self.test_mixed_and_or_query()
            
            print("\n✅ MySQL 复杂查询验证测试完成！")
            
        except Exception as e:
            print(f"\n❌ 测试执行失败: {e}")
            raise


def main():
    """主函数"""
    test = MySQLComplexQueryTest()
    test.run_all_tests()


if __name__ == "__main__":
    main()