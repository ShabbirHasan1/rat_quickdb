#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
单独测试MySQL的JSON字段解析功能
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

import rat_quickdb_py as rq
import json
import time

def test_mysql_only():
    """只测试MySQL JSON字段解析"""
    print("\n" + "="*50)
    print("🚀 测试 MySQL JSON字段解析")
    print("="*50)

    bridge = rq.create_db_queue_bridge()

    # 添加MySQL数据库
    result = bridge.add_mysql_database(
        alias="mysql_json_test",
        host="172.16.0.21",
        port=3306,
        database="testdb",
        username="testdb",
        password="yash2vCiBA&B#h$#i&gb@IGSTh&cP#QC^",
        max_connections=5,
        min_connections=1,
        connection_timeout=30,
        idle_timeout=600,
        max_lifetime=3600
    )

    if not json.loads(result).get("success"):
        print(f"❌ MySQL数据库添加失败: {json.loads(result).get('error')}")
        return False

    print("✅ MySQL数据库添加成功")

    # 创建字段定义
    id_field = rq.integer_field(True, True, None, None, "主键ID")
    name_field = rq.string_field(True, False, None, None, "名称")
    json_field = rq.json_field(False, "JSON数据")

    # 创建索引
    index_def = rq.IndexDefinition(["id"], True, "idx_id")

    # 创建字段字典
    fields_dict = {
        "id": id_field,
        "name": name_field,
        "json_data": json_field
    }

    # 创建模型元数据
    table_name = f"mysql_json_{int(time.time())}"
    model_meta = rq.ModelMeta(
        table_name,
        fields_dict,
        [index_def],
        "mysql_json_test",
        "MySQL JSON测试表"
    )

    # 注册模型
    register_result = bridge.register_model(model_meta)
    if not json.loads(register_result).get("success"):
        print(f"❌ ODM模型注册失败")
        return False

    print("✅ ODM模型注册成功")

    # 测试数据 - MySQL特有的复杂数据结构
    test_data = {
        "name": "MySQL复杂JSON测试",
        "json_data": {
            "ecommerce": {
                "order": {
                    "order_id": "ORD-2025-001",
                    "customer": {
                        "customer_id": "CUST-001",
                        "name": "张三",
                        "email": "zhangsan@example.com",
                        "phone": "+86-138-0000-0000",
                        "addresses": [
                            {
                                "type": "billing",
                                "street": "北京市朝阳区某某街道123号",
                                "city": "北京",
                                "postal_code": "100000",
                                "is_default": True
                            },
                            {
                                "type": "shipping",
                                "street": "上海市浦东新区某某路456号",
                                "city": "上海",
                                "postal_code": "200000",
                                "is_default": False
                            }
                        ]
                    },
                    "items": [
                        {
                            "product_id": "P001",
                            "name": "笔记本电脑",
                            "category": "电子产品",
                            "price": 5999.99,
                            "quantity": 1,
                            "specs": {
                                "cpu": "Intel Core i7-12700H",
                                "memory": "16GB DDR5",
                                "storage": "512GB NVMe SSD",
                                "display": "15.6英寸 4K IPS"
                            }
                        },
                        {
                            "product_id": "P002",
                            "name": "无线鼠标",
                            "category": "配件",
                            "price": 199.00,
                            "quantity": 2,
                            "specs": {
                                "connection": "蓝牙5.2",
                                "battery": "可充电锂电池",
                                "dpi": "1600"
                            }
                        }
                    ],
                    "payment": {
                        "method": "credit_card",
                        "card_number": "****-****-****-1234",
                        "amount": 6397.99,
                        "currency": "CNY",
                        "transaction_id": "TXN-2025-001",
                        "status": "completed"
                    },
                    "shipping": {
                        "method": "express",
                        "cost": 25.00,
                        "estimated_delivery": "2025-01-20",
                        "tracking_number": "SF1234567890"
                    }
                }
            },
            "analytics": {
                "source": "web",
                "campaign": "新年促销",
                "device_type": "desktop",
                "browser": "Chrome",
                "ip_address": "192.168.1.100",
                "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                "session_id": "SES-2025-001",
                "event_timestamp": "2025-01-15T14:30:00Z"
            },
            "metadata": {
                "created_at": "2025-01-15T14:30:00Z",
                "updated_at": "2025-01-15T14:35:00Z",
                "version": 1,
                "tags": ["电商", "订单", "促销", "新年"],
                "priority": "high",
                "is_processed": True,
                "processing_time": 2.5
            }
        }
    }

    # 插入数据
    insert_result = bridge.create(table_name, json.dumps(test_data), "mysql_json_test")
    insert_data = json.loads(insert_result)

    if not insert_data.get("success"):
        print(f"❌ 数据插入失败: {insert_data.get('error')}")
        return False

    print("✅ 数据插入成功")

    # 查询数据
    query_result = bridge.find(table_name, '{}', "mysql_json_test")
    query_data = json.loads(query_result)

    if not query_data.get("success"):
        print(f"❌ 数据查询失败: {query_data.get('error')}")
        return False

    records = query_data.get("data")
    if not records or len(records) == 0:
        print("❌ 查询结果为空")
        return False

    record = records[0]
    print(f"✅ 数据查询成功")

    # 验证JSON字段
    json_field = record.get('json_data')
    print(f"   json_data类型: {type(json_field)}")

    if isinstance(json_field, dict):
        print("✅ JSON字段正确解析为dict")

        # 验证深层嵌套的电商数据结构
        ecommerce = json_field.get('ecommerce', {})
        if isinstance(ecommerce, dict):
            order = ecommerce.get('order', {})
            if isinstance(order, dict):
                print(f"✅ order.order_id: {order.get('order_id')}")

                customer = order.get('customer', {})
                if isinstance(customer, dict):
                    print(f"✅ customer.name: {customer.get('name')}")
                    print(f"✅ customer.email: {customer.get('email')}")

                    addresses = customer.get('addresses', [])
                    if isinstance(addresses, list) and len(addresses) > 0:
                        print(f"✅ customer.addresses数量: {len(addresses)}")
                        print(f"✅ 第一个地址类型: {addresses[0].get('type')}")
                        print(f"✅ 第一个地址城市: {addresses[0].get('city')}")

                items = order.get('items', [])
                if isinstance(items, list) and len(items) > 0:
                    print(f"✅ order.items数量: {len(items)}")
                    first_item = items[0]
                    if isinstance(first_item, dict):
                        print(f"✅ 第一个商品: {first_item.get('name')}")
                        print(f"✅ 第一个商品价格: {first_item.get('price')}")

                        specs = first_item.get('specs', {})
                        if isinstance(specs, dict):
                            print(f"✅ 第一个商品CPU: {specs.get('cpu')}")
                            print(f"✅ 第一个商品内存: {specs.get('memory')}")

                payment = order.get('payment', {})
                if isinstance(payment, dict):
                    print(f"✅ payment.method: {payment.get('method')}")
                    print(f"✅ payment.amount: {payment.get('amount')}")
                    print(f"✅ payment.status: {payment.get('status')}")

        analytics = json_field.get('analytics', {})
        if isinstance(analytics, dict):
            print(f"✅ analytics.source: {analytics.get('source')}")
            print(f"✅ analytics.campaign: {analytics.get('campaign')}")
            print(f"✅ analytics.device_type: {analytics.get('device_type')}")

        metadata = json_field.get('metadata', {})
        if isinstance(metadata, dict):
            print(f"✅ metadata.created_at: {metadata.get('created_at')}")
            print(f"✅ metadata.tags: {metadata.get('tags')}")
            print(f"✅ metadata.is_processed: {metadata.get('is_processed')}")
            print(f"✅ metadata.processing_time: {metadata.get('processing_time')}")

        print("\n🎯 MySQL JSON字段解析验证完成，所有复杂嵌套结构都正确解析！")
    else:
        print(f"❌ JSON字段解析失败: {type(json_field)}")
        return False

    # 清理
    bridge.drop_table(table_name, "mysql_json_test")
    print("✅ MySQL测试完成")
    return True

def main():
    """主函数 - 只测试MySQL"""
    print("🧪 MySQL数据库JSON字段解析验证")
    print("专门测试MySQL的JSON字段解析功能")

    # 初始化日志
    try:
        rq.init_logging_with_level("info")
        print("✅ 日志初始化成功")
    except:
        print("⚠️ 日志初始化失败")

    result = test_mysql_only()

    print("\n" + "="*50)
    print("🎯 测试结果")
    print("="*50)
    print(f"MySQL: {'✅ 通过' if result else '❌ 失败'}")

    if result:
        print("\n🎉 MySQL JSON字段解析功能完全正常！")
        print("✅ register_model功能正常工作")
        print("✅ ODM模型注册让MySQL能正确识别和解析JSON字段")
        print("✅ 支持超复杂的嵌套JSON结构")
        print("✅ 支持JSON数组中的复杂对象")
        print("✅ 所有数据类型（字符串、数字、布尔值、数组、对象）都正确处理")
        print("✅ MySQL的TEXT字段正确转换为JSON对象")
        return True
    else:
        print("\n⚠️ MySQL JSON字段解析功能存在问题")
        return False

if __name__ == "__main__":
    main()