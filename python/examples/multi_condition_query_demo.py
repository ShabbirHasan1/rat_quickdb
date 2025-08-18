#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
多条件查询演示

本示例展示了 rat_quickdb 支持的三种查询条件格式：
1. 单个查询条件对象格式
2. 多个查询条件数组格式 
3. 简化的键值对格式
"""

import json
import os
import tempfile
import time
from rat_quickdb_py import create_db_queue_bridge, PyCacheConfig, PyL1CacheConfig


class MultiConditionQueryDemo:
    def __init__(self):
        self.bridge = create_db_queue_bridge()
        self.temp_dir = tempfile.mkdtemp()
        self.db_path = os.path.join(self.temp_dir, "multi_query_demo.db")
        
    def setup_database(self):
        """设置数据库和测试数据"""
        print("🔧 设置数据库...")
        
        # 创建缓存配置（仅启用L1缓存）
        cache_config = PyCacheConfig()
        cache_config.enable()
        cache_config.l1_config = PyL1CacheConfig(max_capacity=1000)
        
        # 添加SQLite数据库
        result = self.bridge.add_sqlite_database(
            alias="demo_db",
            path=self.db_path,
            max_connections=10,
            min_connections=2,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600,
            cache_config=cache_config
        )
        print(f"数据库添加结果: {result}")
        
        # 设置默认别名
        self.bridge.set_default_alias("demo_db")
        
        # 插入测试数据
        test_users = [
            {"name": "张三", "age": 25, "city": "北京", "department": "技术部", "salary": 8000},
            {"name": "李四", "age": 30, "city": "上海", "department": "销售部", "salary": 12000},
            {"name": "王五", "age": 28, "city": "广州", "department": "技术部", "salary": 9500},
            {"name": "赵六", "age": 35, "city": "深圳", "department": "市场部", "salary": 15000},
            {"name": "钱七", "age": 22, "city": "杭州", "department": "技术部", "salary": 7000},
            {"name": "孙八", "age": 40, "city": "成都", "department": "管理部", "salary": 20000},
            {"name": "周九", "age": 26, "city": "西安", "department": "销售部", "salary": 8500},
            {"name": "吴十", "age": 33, "city": "南京", "department": "技术部", "salary": 11000},
        ]
        
        print("📝 插入测试数据...")
        for user in test_users:
            user_json = json.dumps(user)
            result = self.bridge.create("users", user_json, "demo_db")
            print(f"插入用户 {user['name']}: {result}")
            
        print("✅ 数据库设置完成\n")
        
    def demo_single_condition_query(self):
        """演示单个查询条件格式"""
        print("🔍 演示单个查询条件格式")
        print("格式: {\"field\": \"字段名\", \"operator\": \"操作符\", \"value\": \"值\"}")
        
        # 示例1: 等值查询
        query1 = json.dumps({
            "field": "name", 
            "operator": "Eq", 
            "value": "张三"
        })
        print(f"\n查询条件: {query1}")
        result1 = self.bridge.find("users", query1, "demo_db")
        print(f"查询结果: {result1}")
        
        # 示例2: 大于查询
        query2 = json.dumps({
            "field": "age", 
            "operator": "Gt", 
            "value": 30
        })
        print(f"\n查询条件: {query2}")
        result2 = self.bridge.find("users", query2, "demo_db")
        print(f"查询结果: {result2}")
        
        # 示例3: 包含查询
        query3 = json.dumps({
            "field": "city", 
            "operator": "Contains", 
            "value": "京"
        })
        print(f"\n查询条件: {query3}")
        result3 = self.bridge.find("users", query3, "demo_db")
        print(f"查询结果: {result3}")
        
    def demo_multi_condition_array_query(self):
        """演示多个查询条件数组格式"""
        print("\n\n🔍 演示多个查询条件数组格式 (AND逻辑)")
        print("格式: [{\"field\": \"字段1\", \"operator\": \"操作符1\", \"value\": \"值1\"}, {\"field\": \"字段2\", \"operator\": \"操作符2\", \"value\": \"值2\"}]")
        
        # 示例1: 年龄大于25且部门为技术部
        query1 = json.dumps([
            {"field": "age", "operator": "Gt", "value": 25},
            {"field": "department", "operator": "Eq", "value": "技术部"}
        ])
        print(f"\n查询条件: {query1}")
        result1 = self.bridge.find("users", query1, "demo_db")
        print(f"查询结果: {result1}")
        
        # 示例2: 薪资在8000-12000之间且城市包含"海"或"京"
        query2 = json.dumps([
            {"field": "salary", "operator": "Gte", "value": 8000},
            {"field": "salary", "operator": "Lte", "value": 12000},
            {"field": "city", "operator": "Contains", "value": "海"}
        ])
        print(f"\n查询条件: {query2}")
        result2 = self.bridge.find("users", query2, "demo_db")
        print(f"查询结果: {result2}")
        
        # 示例3: 复杂多条件查询
        query3 = json.dumps([
            {"field": "age", "operator": "Gte", "value": 25},
            {"field": "age", "operator": "Lt", "value": 35},
            {"field": "department", "operator": "Ne", "value": "管理部"},
            {"field": "salary", "operator": "Gt", "value": 7500}
        ])
        print(f"\n查询条件: {query3}")
        result3 = self.bridge.find("users", query3, "demo_db")
        print(f"查询结果: {result3}")
        
    def demo_simplified_key_value_query(self):
        """演示简化的键值对格式"""
        print("\n\n🔍 演示简化的键值对格式 (默认使用Eq操作符)")
        print("格式: {\"字段1\": \"值1\", \"字段2\": \"值2\"}")
        
        # 示例1: 简单等值查询
        query1 = json.dumps({
            "name": "李四"
        })
        print(f"\n查询条件: {query1}")
        result1 = self.bridge.find("users", query1, "demo_db")
        print(f"查询结果: {result1}")
        
        # 示例2: 多字段等值查询
        query2 = json.dumps({
            "department": "技术部",
            "city": "广州"
        })
        print(f"\n查询条件: {query2}")
        result2 = self.bridge.find("users", query2, "demo_db")
        print(f"查询结果: {result2}")
        
        # 示例3: 混合数据类型查询
        query3 = json.dumps({
            "age": 30,
            "department": "销售部"
        })
        print(f"\n查询条件: {query3}")
        result3 = self.bridge.find("users", query3, "demo_db")
        print(f"查询结果: {result3}")
        
    def demo_or_logic_query(self):
        """演示OR逻辑查询"""
        print("\n\n🔍 演示OR逻辑查询")
        print("格式: {\"operator\": \"or\", \"conditions\": [{条件1}, {条件2}, ...]}")
        
        # 示例1: 简单OR查询 - 年龄大于35或薪资大于15000
        query1 = json.dumps({
            "operator": "or",
            "conditions": [
                {"field": "age", "operator": "Gt", "value": 35},
                {"field": "salary", "operator": "Gt", "value": 15000}
            ]
        })
        print(f"\n查询条件: {query1}")
        result1 = self.bridge.find_with_groups("users", query1, "demo_db")
        print(f"查询结果: {result1}")
        
        # 示例2: 复杂OR查询 - 技术部员工或城市在北京/上海的员工
        query2 = json.dumps({
            "operator": "or",
            "conditions": [
                {"field": "department", "operator": "Eq", "value": "技术部"},
                {
                    "operator": "or",
                    "conditions": [
                        {"field": "city", "operator": "Eq", "value": "北京"},
                        {"field": "city", "operator": "Eq", "value": "上海"}
                    ]
                }
            ]
        })
        print(f"\n查询条件: {query2}")
        result2 = self.bridge.find_with_groups("users", query2, "demo_db")
        print(f"查询结果: {result2}")
        
        # 示例3: 混合AND/OR查询 - (年龄25-30且技术部) 或 (薪资>12000且销售部)
        query3 = json.dumps({
            "operator": "or",
            "conditions": [
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "age", "operator": "Gte", "value": 25},
                        {"field": "age", "operator": "Lte", "value": 30},
                        {"field": "department", "operator": "Eq", "value": "技术部"}
                    ]
                },
                {
                    "operator": "and",
                    "conditions": [
                        {"field": "salary", "operator": "Gt", "value": 12000},
                        {"field": "department", "operator": "Eq", "value": "销售部"}
                    ]
                }
            ]
        })
        print(f"\n查询条件: {query3}")
        result3 = self.bridge.find_with_groups("users", query3, "demo_db")
        print(f"查询结果: {result3}")
        
        # 示例4: 单个条件组合格式
        query4 = json.dumps([
            {
                "operator": "or",
                "conditions": [
                    {"field": "name", "operator": "Contains", "value": "三"},
                    {"field": "name", "operator": "Contains", "value": "四"}
                ]
            }
        ])
        print(f"\n查询条件: {query4}")
        result4 = self.bridge.find_with_groups("users", query4, "demo_db")
        print(f"查询结果: {result4}")
        
    def demo_performance_comparison(self):
        """演示查询性能对比"""
        print("\n\n⚡ 查询性能对比")
        
        # 复杂查询条件
        complex_query = json.dumps([
            {"field": "age", "operator": "Gte", "value": 25},
            {"field": "salary", "operator": "Gt", "value": 8000},
            {"field": "department", "operator": "Eq", "value": "技术部"}
        ])
        
        # 第一次查询（冷启动）
        start_time = time.time()
        result1 = self.bridge.find("users", complex_query, "demo_db")
        first_query_time = (time.time() - start_time) * 1000
        
        # 第二次查询（缓存命中）
        start_time = time.time()
        result2 = self.bridge.find("users", complex_query, "demo_db")
        second_query_time = (time.time() - start_time) * 1000
        
        print(f"复杂查询条件: {complex_query}")
        print(f"第一次查询时间: {first_query_time:.2f}ms")
        print(f"第二次查询时间: {second_query_time:.2f}ms")
        print(f"性能提升: {(first_query_time / second_query_time):.2f}x")
        print(f"查询结果: {result1}")
        
    def cleanup(self):
        """清理资源"""
        print("\n🧹 清理资源...")
        try:
            if os.path.exists(self.db_path):
                os.remove(self.db_path)
            os.rmdir(self.temp_dir)
            print("✅ 资源清理完成")
        except Exception as e:
            print(f"❌ 清理失败: {e}")
            
    def run_demo(self):
        """运行完整演示"""
        print("🚀 多条件查询演示开始\n")
        
        try:
            self.setup_database()
            self.demo_single_condition_query()
            self.demo_multi_condition_array_query()
            self.demo_simplified_key_value_query()
            self.demo_or_logic_query()
            self.demo_performance_comparison()
            
            print("\n\n🎉 演示完成！")
            print("\n📋 总结:")
            print("1. 单个查询条件格式: 支持所有操作符，适合复杂单条件查询")
            print("2. 多条件数组格式: 支持复杂的AND逻辑组合查询")
            print("3. 简化键值对格式: 适合简单的等值查询，语法简洁")
            print("4. OR逻辑查询格式: 支持复杂的OR/AND混合逻辑查询")
            print("5. 所有格式都支持缓存，显著提升查询性能")
            
        except Exception as e:
            print(f"❌ 演示过程中出现错误: {e}")
        finally:
            self.cleanup()


if __name__ == "__main__":
    demo = MultiConditionQueryDemo()
    demo.run_demo()