#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RAT QuickDB Python 绑定基本使用示例

本示例展示了如何使用基于 JSON 字符串和无锁队列的 rat_quickdb Python 绑定。
包括：
- 队列桥接器创建和配置
- JSON 格式的数据库操作
- 缓存性能对比
- 错误处理示例
"""

import json
import time
from typing import Dict, Any

try:
    from rat_quickdb import QueueBridge, create_queue_bridge
except ImportError:
    print("错误：无法导入 rat_quickdb 模块")
    print("请确保已正确安装 rat-quickdb-py 包")
    print("安装命令：maturin develop")
    exit(1)


def create_queue_bridge_and_configure():
    """创建队列桥接器并配置数据库"""
    # 创建队列桥接器
    bridge = create_queue_bridge()
    
    # 配置数据库连接（JSON 格式）
    config_data = {
        "database_type": "sqlite",
        "connection_string": "sqlite://./example.db",
        "max_connections": 10,
        "connection_timeout": 30,
        "cache_enabled": True,
        "cache_ttl": 300
    }
    
    try:
        response = bridge.send_request("configure_database", json.dumps(config_data))
        print(f"数据库配置成功: {response}")
        return bridge
    except Exception as e:
        print(f"数据库配置失败: {e}")
        return None


def basic_crud_operations(bridge):
    """基本 CRUD 操作示例"""
    print("\n=== 基本 CRUD 操作 ===")
    
    # 创建用户表数据
    user_data = {
        "table": "users",
        "data": {
            "name": "张三",
            "age": 25,
            "email": "zhangsan@example.com",
            "city": "北京"
        }
    }
    
    print(f"创建用户: {user_data['data']}")
    try:
        response = bridge.send_request("create_record", json.dumps(user_data))
        result = json.loads(response)
        user_id = result.get("id")
        print(f"用户创建成功，ID: {user_id}")
    except Exception as e:
        print(f"创建用户失败: {e}")
        return None
    
    # 查询用户
    query_data = {
        "table": "users",
        "id": user_id
    }
    
    print(f"\n查询用户 ID: {user_id}")
    try:
        response = bridge.send_request("find_by_id", json.dumps(query_data))
        user = json.loads(response)
        print(f"查询结果: {user}")
    except Exception as e:
        print(f"查询用户失败: {e}")
    
    # 更新用户
    update_data = {
        "table": "users",
        "id": user_id,
        "data": {"age": 26, "city": "上海"}
    }
    
    print(f"\n更新用户数据: {update_data['data']}")
    try:
        response = bridge.send_request("update_by_id", json.dumps(update_data))
        result = json.loads(response)
        success = result.get("success", False)
        print(f"更新结果: {'成功' if success else '失败'}")
    except Exception as e:
        print(f"更新用户失败: {e}")
    
    # 再次查询验证更新
    try:
        response = bridge.send_request("find_by_id", json.dumps(query_data))
        updated_user = json.loads(response)
        print(f"更新后的用户: {updated_user}")
    except Exception as e:
        print(f"验证更新失败: {e}")
    
    return user_id


def query_operations(bridge):
    """查询操作示例"""
    print("\n=== 查询操作示例 ===")
    
    # 批量创建测试数据
    test_users = [
        {"name": "李四", "age": 30, "city": "广州"},
        {"name": "王五", "age": 28, "city": "深圳"},
        {"name": "赵六", "age": 32, "city": "北京"},
    ]
    
    print("创建测试用户...")
    for user_info in test_users:
        user_data = {
            "table": "users",
            "data": user_info
        }
        try:
            bridge.send_request("create_record", json.dumps(user_data))
        except Exception as e:
            print(f"创建用户失败: {e}")
    
    # 条件查询
    query_data = {
        "table": "users",
        "filter": {"city": "北京"},
        "sort": {"field": "age", "direction": "desc"},
        "limit": 10
    }
    
    print("\n查询北京用户（按年龄降序）:")
    try:
        response = bridge.send_request("find_many", json.dumps(query_data))
        beijing_users = json.loads(response)
        if isinstance(beijing_users, list):
            for user in beijing_users:
                print(f"  - {user.get('name')}, 年龄: {user.get('age')}, 城市: {user.get('city')}")
        else:
            print(f"查询结果: {beijing_users}")
    except Exception as e:
        print(f"查询失败: {e}")
    
    # 统计查询
    print("\n统计查询:")
    try:
        # 总用户数
        count_data = {"table": "users", "filter": {}}
        response = bridge.send_request("count", json.dumps(count_data))
        total_result = json.loads(response)
        total_count = total_result.get("count", 0)
        
        # 北京用户数
        beijing_count_data = {"table": "users", "filter": {"city": "北京"}}
        response = bridge.send_request("count", json.dumps(beijing_count_data))
        beijing_result = json.loads(response)
        beijing_count = beijing_result.get("count", 0)
        
        print(f"总用户数: {total_count}")
        print(f"北京用户数: {beijing_count}")
    except Exception as e:
        print(f"统计查询失败: {e}")


def cache_performance_test(bridge, user_id):
    """缓存性能测试"""
    print("\n=== 缓存性能测试 ===")
    
    if user_id is None:
        print("用户ID为空，跳过缓存测试")
        return
    
    query_data = {
        "table": "users",
        "id": user_id
    }
    
    try:
        # 第一次查询（无缓存）
        start_time = time.time()
        response1 = bridge.send_request("find_by_id", json.dumps(query_data))
        user1 = json.loads(response1)
        first_query_time = time.time() - start_time
        
        # 第二次查询（有缓存）
        start_time = time.time()
        response2 = bridge.send_request("find_by_id", json.dumps(query_data))
        user2 = json.loads(response2)
        cached_query_time = time.time() - start_time
        
        print(f"首次查询时间: {first_query_time:.4f}s")
        print(f"缓存查询时间: {cached_query_time:.4f}s")
        
        if cached_query_time > 0:
            print(f"性能提升: {first_query_time / cached_query_time:.2f}x")
        else:
            print("缓存查询时间过短，无法计算性能提升")
        
        # 验证数据一致性
        if user1 == user2:
            print("缓存数据一致性验证通过")
        else:
            print("警告：缓存数据不一致")
            
    except Exception as e:
        print(f"缓存性能测试失败: {e}")


def cleanup_operations(bridge, user_id):
    """清理操作"""
    print("\n=== 清理操作 ===")
    
    if user_id is None:
        print("用户ID为空，跳过删除操作")
    else:
        # 删除指定用户
        delete_data = {
            "table": "users",
            "id": user_id
        }
        
        print(f"删除用户 ID: {user_id}")
        try:
            response = bridge.send_request("delete_by_id", json.dumps(delete_data))
            result = json.loads(response)
            success = result.get("success", False)
            print(f"删除结果: {'成功' if success else '失败'}")
        except Exception as e:
            print(f"删除用户失败: {e}")
    
    # 清空表（谨慎操作）
    print("清空用户表...")
    try:
        delete_many_data = {
            "table": "users",
            "filter": {}
        }
        response = bridge.send_request("delete_many", json.dumps(delete_many_data))
        result = json.loads(response)
        deleted_count = result.get("deleted_count", 0)
        print(f"删除了 {deleted_count} 条记录")
    except Exception as e:
        print(f"清空表失败: {e}")


def main():
    """主函数"""
    print("RAT QuickDB Python 绑定示例（基于 JSON 和无锁队列）")
    print("=" * 60)
    
    try:
        # 创建队列桥接器并配置数据库
        print("初始化队列桥接器和数据库配置...")
        bridge = create_queue_bridge_and_configure()
        
        if bridge is None:
            print("初始化失败，退出程序")
            return
        
        # 执行各种操作
        user_id = basic_crud_operations(bridge)
        query_operations(bridge)
        cache_performance_test(bridge, user_id)
        cleanup_operations(bridge, user_id)
        
        print("\n示例执行完成！")
        print("\n注意：当前示例使用模拟响应，实际功能需要完整的后端实现")
        
    except Exception as e:
        print(f"\n错误: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()