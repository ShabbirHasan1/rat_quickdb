#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MongoDB 简化复杂查询验证脚本

验证 MongoDB 数据库的复杂查询功能，包括：
1. 多条件 AND 查询
2. 范围查询
3. 字符串模糊匹配
4. 列表查询
5. 组合查询条件
"""

import json
import os
import time
from rat_quickdb_py import (
    create_db_queue_bridge,
    PyCacheConfig,
    PyL1CacheConfig,
    PyL2CacheConfig,
    PyTtlConfig,
    PyCompressionConfig,
    PyTlsConfig,
    PyZstdConfig
)


class MongoDBComplexQueryTest:
    def __init__(self):
        self.bridge = create_db_queue_bridge()
        
        # 使用时间戳作为集合名后缀，避免重复
        timestamp = int(time.time() * 1000)
        self.collection_name = f"test_users_{timestamp}"
        
    def setup_database(self):
        """设置MongoDB数据库连接"""
        print("🔧 设置MongoDB数据库连接...")
        
        # 不使用缓存，直接连接MongoDB
        
        # 不使用缓存配置，直接连接MongoDB
        cache_config = None
        
        # TLS配置
        tls_config = PyTlsConfig()
        tls_config.enable()
        tls_config.ca_cert_path = "/etc/ssl/certs/ca-certificates.crt"
        tls_config.client_cert_path = ""
        tls_config.client_key_path = ""
        
        # ZSTD配置
        zstd_config = PyZstdConfig()
        zstd_config.enable()
        zstd_config.compression_level = 3
        zstd_config.compression_threshold = 1024
        
        # 添加MongoDB数据库（无缓存）
        result = self.bridge.add_mongodb_database(
            alias="mongodb_test",
            host="db0.0ldm0s.net",
            port=27017,
            database="testdb",
            username="testdb",
            password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
            auth_source="testdb",
            direct_connection=True,
            max_connections=8,
            min_connections=2,
            connection_timeout=5,
            idle_timeout=60,
            max_lifetime=300,
            cache_config=cache_config,
            tls_config=tls_config,
            zstd_config=zstd_config
        )
        print(f"MongoDB数据库添加结果: {result}")
        
        # 设置默认别名
        self.bridge.set_default_alias("mongodb_test")
        
    def insert_test_data(self):
        """插入测试数据"""
        print("📝 插入测试数据...")
        
        test_users = [
            {
                "id": "user_001",
                "name": "张三",
                "age": 25,
                "email": "zhangsan@example.com",
                "department": "技术部",
                "salary": 8000.0,
                "is_active": True,
                "skills": ["Python", "SQL"],
                "city": "北京",
                "metadata": '{"level": "junior", "skills": ["Python", "SQL"]}',
                "tags": '["backend", "database"]'
            },
            {
                "id": "user_002",
                "name": "李四",
                "age": 30,
                "email": "lisi@example.com",
                "department": "产品部",
                "salary": 12000.0,
                "is_active": True,
                "skills": ["Product", "Design"],
                "city": "上海",
                "metadata": '{"level": "senior", "skills": ["Product", "Design"]}',
                "tags": '["frontend", "ui"]'
            },
            {
                "id": "user_003",
                "name": "王五",
                "age": 28,
                "email": "wangwu@example.com",
                "department": "技术部",
                "salary": 10000.0,
                "is_active": False,
                "skills": ["Java", "Spring"],
                "city": "深圳",
                "metadata": '{"level": "middle", "skills": ["Java", "Spring"]}',
                "tags": '["backend", "api"]'
            },
            {
                "id": "user_004",
                "name": "赵六",
                "age": 35,
                "email": "zhaoliu@example.com",
                "department": "管理部",
                "salary": 15000.0,
                "is_active": True,
                "skills": ["Management", "Strategy"],
                "city": "广州",
                "metadata": '{"level": "manager", "skills": ["Management", "Strategy"]}',
                "tags": '["management", "strategy"]'
            },
            {
                "id": "user_005",
                "name": "钱七",
                "age": 27,
                "email": "qianqi@company.net",
                "department": "技术部",
                "salary": 9500.0,
                "is_active": True,
                "skills": ["AI", "Machine Learning"],
                "city": "杭州",
                "metadata": '{"level": "senior", "skills": ["AI", "Machine Learning"]}',
                "tags": '["ai", "research"]'
            },
            {
                "id": "user_006",
                "name": "孙八",
                "age": 32,
                "email": "sunba@example.com",
                "department": "运营部",
                "salary": 11000.0,
                "is_active": True,
                "skills": ["Marketing", "Analytics"],
                "city": "成都",
                "metadata": '{"level": "senior", "skills": ["Marketing", "Analytics"]}',
                "tags": '["marketing", "data"]'
            }
        ]
        
        for user in test_users:
            user_json = json.dumps(user)
            result = self.bridge.create(self.collection_name, user_json, "mongodb_test")
            print(f"  插入用户 {user['name']}: {result}")
            
    def test_and_logic_query(self):
        """测试 AND 逻辑查询"""
        print("\n🔍 测试 AND 逻辑查询...")
        
        # 查询技术部且年龄大于25的员工（王五28岁，钱七27岁）
        query = {
            "department": "技术部",
            "age": {"Gt": 25}
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        print(f"  原始查询结果: {json.dumps(results_data, ensure_ascii=False, indent=2)}")
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录")
            for result in results:
                print(f"    - {result.get('name')}: {result.get('age')}岁, {result.get('department')}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
            print(f"  查询结果: 找到 0 条记录")
            results = []
            
        return len(results) > 0
        
    def test_range_query(self):
        """测试范围查询"""
        print("\n🔍 测试范围查询...")
        
        # 查找年龄在25-30岁之间的员工
        query = {
            "age": {"Gte": 25, "Lte": 30}
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        print(f"  原始查询结果: {json.dumps(results_data, ensure_ascii=False, indent=2)}")
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录")
            
            for result in results:
                print(f"    - {result['name']}: 年龄 {result['age']}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
            results = []
            
        return len(results) > 0
        
    def test_string_contains_query(self):
        """测试字符串包含查询"""
        print("\n🔍 测试字符串包含查询...")
        
        # 查找邮箱包含"example.com"的员工
        query = {
            "email": {"Contains": "example.com"}
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        print(f"  原始查询结果: {json.dumps(results_data, ensure_ascii=False, indent=2)}")
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录")
            
            for result in results:
                print(f"    - {result['name']}: {result['email']}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
            results = []
            
        return len(results) > 0
        
    def test_array_in_query(self):
        """测试数组In查询"""
        print("\n🔍 测试数组In查询...")
        
        # 查找部门为技术部或产品部的员工（张三、王五、钱七、李四）
        query = {
            "department": {"In": ["技术部", "产品部"]}
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        print(f"  原始查询结果: {json.dumps(results_data, ensure_ascii=False, indent=2)}")
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录")
            for result in results:
                print(f"    - {result.get('name')}: {result.get('department')}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
            print(f"  查询结果: 找到 0 条记录")
            results = []
            
        return len(results) > 0
        
    def test_or_logic_query(self):
        """测试OR逻辑查询"""
        print("\n🔍 测试OR逻辑查询...")
        
        # 查找年龄大于30或薪资大于11000的员工（孙八32岁，李四薪资12000）
        query = {
            "operator": "or",
            "conditions": [
                {"field": "age", "operator": "Gt", "value": 30},
                {"field": "salary", "operator": "Gt", "value": 11000}
            ]
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        print(f"  原始查询结果: {json.dumps(results_data, ensure_ascii=False, indent=2)}")
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录")
            
            for result in results:
                print(f"    - {result['name']}: 年龄 {result['age']}, 薪资: {result['salary']}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
            results = []
            
        return len(results) > 0
        
    def test_mixed_and_or_query(self):
        """测试混合AND/OR查询"""
        print("\n🔍 测试混合AND/OR查询...")
        
        # 查找(技术部且年龄>25) 或 (管理部且薪资>14000)的员工（王五28岁技术部，钱七27岁技术部，赵六35岁管理部薪资15000）
        query = {
            "operator": "or",
            "conditions": [
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "department", "operator": "Eq", "value": "技术部"},
                        {"field": "age", "operator": "Gt", "value": 25}
                    ]
                },
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "department", "operator": "Eq", "value": "管理部"},
                        {"field": "salary", "operator": "Gt", "value": 14000}
                    ]
                }
            ]
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        print(f"  原始查询结果: {json.dumps(results_data, ensure_ascii=False, indent=2)}")
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录")
            
            for result in results:
                print(f"    - {result['name']}: {result['department']}, 年龄: {result['age']}, 薪资: {result['salary']}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
            results = []
            
        return len(results) > 0
        
    def test_complex_combined_query(self):
        """测试复杂组合查询"""
        print("\n🔍 测试复杂组合查询...")
        
        # 查找技术部且薪资大于8000且状态为活跃的员工（钱七技术部薪资9500）
        query = {
            "department": "技术部",
            "salary": {"Gt": 8000},
            "is_active": True
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        print(f"  原始查询结果: {json.dumps(results_data, ensure_ascii=False, indent=2)}")
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录")
            for result in results:
                print(f"    - {result.get('name')}: {result.get('department')}, 薪资: {result.get('salary')}, 状态: {result.get('is_active')}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
            print(f"  查询结果: 找到 0 条记录")
            results = []
            
        return len(results) > 0
        
    def test_empty_result_query(self):
        """测试预期为空的查询结果"""
        print("\n🔍 测试预期为空的查询结果...")
        
        # 查找不存在的部门（预期为空）
        query = {
            "department": "不存在的部门"
        }
        
        print(f"  查询条件: {json.dumps(query, ensure_ascii=False, indent=2)}")
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  查询结果: 找到 {len(results)} 条记录（预期为0）")
            if len(results) == 0:
                print("  ✅ 空查询结果测试通过")
                return True
            else:
                print("  ❌ 空查询结果测试失败，预期为空但找到了记录")
                return False
        else:
            print(f"  查询失败: {results_data.get('error')}")
            return False
    
    def view_all_data(self):
        """查看所有插入的数据"""
        print("\n🔍 查看所有插入的数据...")
        
        # 查询所有数据
        query = {}
        
        results_json = self.bridge.find(self.collection_name, json.dumps(query), "mongodb_test")
        results_data = json.loads(results_json)
        
        if results_data.get("success"):
            results = results_data.get("data", [])
            print(f"  总共找到 {len(results)} 条记录:")
            for i, result in enumerate(results, 1):
                print(f"    {i}. {result.get('name')}: 部门={result.get('department')}, 年龄={result.get('age')}, 薪资={result.get('salary')}, 状态={result.get('is_active')}")
        else:
            print(f"  查询失败: {results_data.get('error')}")
        
    def cleanup(self):
        """清理资源"""
        print("\n🧹 清理资源...")
        try:
            # 删除测试数据
            delete_conditions = json.dumps([
                {"field": "id", "operator": "Contains", "value": "user_"}
            ])
            result = self.bridge.delete(self.collection_name, delete_conditions, "mongodb_test")
            print(f"  删除测试数据: {result}")
            
            # 无缓存目录需要清理
                
            print("  清理完成")
            
        except Exception as e:
            print(f"  清理过程中出错: {e}")
            
    def run_test(self):
        """运行所有测试"""
        print("=== MongoDB 复杂查询验证测试 ===")
        
        try:
            # 设置数据库
            self.setup_database()
            
            # 插入测试数据
            self.insert_test_data()
            
            # 先查看所有数据
            self.view_all_data()
            
            # 运行各种查询测试
            test_results = []
            test_results.append(self.test_and_logic_query())
            test_results.append(self.test_range_query())
            test_results.append(self.test_string_contains_query())
            test_results.append(self.test_array_in_query())
            test_results.append(self.test_or_logic_query())
            test_results.append(self.test_mixed_and_or_query())
            test_results.append(self.test_complex_combined_query())
            test_results.append(self.test_empty_result_query())
            
            # 统计结果
            passed_tests = sum(test_results)
            total_tests = len(test_results)
            
            print(f"\n📊 测试结果统计:")
            print(f"  总测试数: {total_tests}")
            print(f"  通过测试: {passed_tests}")
            print(f"  失败测试: {total_tests - passed_tests}")
            
            if passed_tests == total_tests:
                print("\n✅ 所有 MongoDB 复杂查询测试通过!")
                return True
            else:
                print(f"\n❌ 有 {total_tests - passed_tests} 个测试失败!")
                return False
                
        except Exception as e:
            print(f"\n❌ 测试过程中出错: {e}")
            return False
        finally:
            self.cleanup()


def main():
    """主函数"""
    test = MongoDBComplexQueryTest()
    success = test.run_test()
    
    if success:
        print("\n🎉 MongoDB 复杂查询验证完成，所有测试通过!")
        exit(0)
    else:
        print("\n💥 MongoDB 复杂查询验证失败!")
        exit(1)


if __name__ == "__main__":
    main()