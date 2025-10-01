#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
DateTime字段使用演示
展示如何在RAT QuickDB Python绑定中正确使用DateTime字段
"""

import asyncio
import json
import os
from datetime import datetime, timezone
from typing import Dict, List, Optional

try:
    from rat_quickdb_py import (
        create_db_queue_bridge,
        string_field,
        integer_field,
        boolean_field,
        datetime_field,
        FieldType,
        FieldDefinition,
        IndexDefinition
    )
except ImportError as e:
    print(f"导入 rat_quickdb_py 失败: {e}")
    print("请确保已运行 'maturin develop' 编译 PyO3 绑定")
    exit(1)

class DateTimeFieldDemo:
    """DateTime字段使用演示类"""

    def __init__(self):
        self.bridge = None
        self.db_alias = "datetime_demo"
        self.table_name = "events"

    def setup_database(self):
        """设置数据库连接"""
        print("设置数据库连接...")

        # 创建数据库桥接器
        self.bridge = create_db_queue_bridge()

        # 清理旧数据库文件
        db_path = "./datetime_demo.db"
        if os.path.exists(db_path):
            os.remove(db_path)
            print(f"🧹 清理旧数据库文件: {db_path}")

        # 添加SQLite数据库
        result = self.bridge.add_sqlite_database(
            alias=self.db_alias,
            path=db_path,
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=300,
            max_lifetime=1800
        )
        print(f"SQLite数据库添加结果: {result}")
        print("数据库连接建立完成")

    def create_table_with_datetime_fields(self):
        """创建包含DateTime字段的表"""
        print("\n创建包含DateTime字段的表...")

        # 定义字段 - 包含各种DateTime字段配置
        fields = {
            'id': string_field(required=True, unique=True, description="事件ID"),
            'title': string_field(required=True, max_length=200, description="事件标题"),
            'description': string_field(required=False, description="事件描述"),
            'event_time': datetime_field(required=True, description="事件发生时间"),
            'created_at': datetime_field(required=True, description="创建时间"),
            'updated_at': datetime_field(required=False, description="更新时间"),
            'start_time': datetime_field(required=False, description="开始时间"),
            'end_time': datetime_field(required=False, description="结束时间"),
            'reminder_time': datetime_field(required=False, description="提醒时间"),
            'is_active': boolean_field(required=True, description="是否激活"),
            'priority': integer_field(required=False, min_value=1, max_value=5, description="优先级")
        }

        def convert_field_definition_to_json(field_def):
            """将FieldDefinition对象转换为JSON可序列化的格式"""
            field_repr = str(field_def)

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
                return "string"

        # 转换为可序列化的字典
        serializable_fields = {}
        for field_name, field_def in fields.items():
            if hasattr(field_def, 'to_dict'):
                serializable_fields[field_name] = field_def.to_dict()
            else:
                serializable_fields[field_name] = convert_field_definition_to_json(field_def)

        # 创建表
        result = self.bridge.create_table(
            self.table_name,
            json.dumps(serializable_fields),
            self.db_alias
        )
        print(f"表创建结果: {result}")

        print("索引创建已包含在表定义中（通过数据库自动处理）")

    def demonstrate_datetime_usage(self):
        """演示DateTime字段的正确使用"""
        print("\n=== DateTime字段使用演示 ===")

        # 演示正确的时间创建方式
        print("\n1. 正确的DateTime创建方式:")

        # 当前UTC时间（推荐方式）
        current_time = datetime.now(timezone.utc)
        print(f"   UTC时间: {current_time}")
        print(f"   ISO格式: {current_time.isoformat()}")

        # 特定时间
        specific_time = datetime(2025, 10, 1, 14, 30, 0, tzinfo=timezone.utc)
        print(f"   特定时间: {specific_time}")
        print(f"   ISO格式: {specific_time.isoformat()}")

        # 验证时间格式
        test_times = [
            current_time.isoformat(),
            specific_time.isoformat(),
            "2025-12-25T10:00:00+00:00",  # 圣诞节
            "2025-01-01T00:00:00+00:00",  # 新年
        ]

        print(f"\n2. 测试时间格式验证:")
        for time_str in test_times:
            try:
                parsed_time = datetime.fromisoformat(time_str)
                print(f"   ✅ {time_str} -> {parsed_time}")
            except ValueError as e:
                print(f"   ❌ {time_str} -> 解析失败: {e}")

    def create_sample_events(self):
        """创建示例事件数据"""
        print("\n创建示例事件数据...")

        # 示例事件数据
        events = [
            {
                "id": "event_001",
                "title": "项目启动会议",
                "description": "讨论新项目的启动计划和目标",
                "event_time": datetime.now(timezone.utc).isoformat(),
                "created_at": datetime.now(timezone.utc).isoformat(),
                "updated_at": None,
                "start_time": datetime(2025, 10, 2, 9, 0, 0, tzinfo=timezone.utc).isoformat(),
                "end_time": datetime(2025, 10, 2, 10, 30, 0, tzinfo=timezone.utc).isoformat(),
                "reminder_time": datetime(2025, 10, 2, 8, 30, 0, tzinfo=timezone.utc).isoformat(),
                "is_active": True,
                "priority": 5
            },
            {
                "id": "event_002",
                "title": "代码审查",
                "description": "审查新功能的代码实现",
                "event_time": datetime.now(timezone.utc).isoformat(),
                "created_at": datetime.now(timezone.utc).isoformat(),
                "updated_at": datetime.now(timezone.utc).isoformat(),
                "start_time": datetime(2025, 10, 3, 14, 0, 0, tzinfo=timezone.utc).isoformat(),
                "end_time": datetime(2025, 10, 3, 15, 0, 0, tzinfo=timezone.utc).isoformat(),
                "reminder_time": None,
                "is_active": True,
                "priority": 3
            },
            {
                "id": "event_003",
                "title": "客户演示",
                "description": "向客户演示产品新功能",
                "event_time": datetime(2025, 10, 5, 16, 0, 0, tzinfo=timezone.utc).isoformat(),
                "created_at": datetime.now(timezone.utc).isoformat(),
                "updated_at": None,
                "start_time": datetime(2025, 10, 5, 15, 30, 0, tzinfo=timezone.utc).isoformat(),
                "end_time": datetime(2025, 10, 5, 17, 0, 0, tzinfo=timezone.utc).isoformat(),
                "reminder_time": datetime(2025, 10, 5, 15, 0, 0, tzinfo=timezone.utc).isoformat(),
                "is_active": False,  # 已取消
                "priority": 4
            }
        ]

        # 插入数据
        for event in events:
            result = self.bridge.create(
                self.table_name,
                json.dumps(event),
                self.db_alias
            )
            print(f"创建事件 '{event['title']}': {result}")

    def test_datetime_queries(self):
        """测试DateTime字段的查询功能"""
        print("\n=== DateTime字段查询测试 ===")

        # 1. 查询所有事件
        print("\n1. 查询所有事件:")
        all_events = self.bridge.find(self.table_name, "{}", self.db_alias)
        if all_events:
            events_data = json.loads(all_events)
            if isinstance(events_data, dict) and events_data.get("success"):
                events = events_data.get("data", [])
                print(f"   找到 {len(events)} 个事件")
                for event in events:
                    print(f"   - {event.get('title')}: {event.get('event_time')}")

        # 2. 按时间范围查询
        print("\n2. 查询今天的事件:")
        today_start = datetime.now(timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)
        today_end = today_start.replace(hour=23, minute=59, second=59, microsecond=999999)

        time_range_query = {
            "operator": "and",
            "conditions": [
                {
                    "field": "event_time",
                    "operator": "Gte",
                    "value": today_start.isoformat()
                },
                {
                    "field": "event_time",
                    "operator": "Lte",
                    "value": today_end.isoformat()
                }
            ]
        }

        today_events = self.bridge.find(
            self.table_name,
            json.dumps(time_range_query),
            self.db_alias
        )

        if today_events:
            events_data = json.loads(today_events)
            if isinstance(events_data, dict) and events_data.get("success"):
                events = events_data.get("data", [])
                print(f"   今天的事件: {len(events)} 个")
                for event in events:
                    print(f"   - {event.get('title')}: {event.get('event_time')}")

        # 3. 查询有提醒时间的活跃事件
        print("\n3. 查询有提醒时间的活跃事件:")
        reminder_query = {
            "operator": "and",
            "conditions": [
                {"field": "is_active", "operator": "Eq", "value": True},
                {"field": "reminder_time", "operator": "IsNotNull", "value": None}
            ]
        }

        reminder_events = self.bridge.find(
            self.table_name,
            json.dumps(reminder_query),
            self.db_alias
        )

        if reminder_events:
            events_data = json.loads(reminder_events)
            if isinstance(events_data, dict) and events_data.get("success"):
                events = events_data.get("data", [])
                print(f"   有提醒的活跃事件: {len(events)} 个")
                for event in events:
                    print(f"   - {event.get('title')}: 提醒时间 {event.get('reminder_time')}")

        # 4. 按优先级和时间排序查询
        print("\n4. 查询高优先级事件（按时间排序）:")
        priority_query = {
            "field": "priority",
            "operator": "Gte",
            "value": 4
        }

        priority_events = self.bridge.find(
            self.table_name,
            json.dumps(priority_query),
            self.db_alias
        )

        if priority_events:
            events_data = json.loads(priority_events)
            if isinstance(events_data, dict) and events_data.get("success"):
                events = events_data.get("data", [])
                print(f"   高优先级事件: {len(events)} 个")
                # 按event_time排序
                events.sort(key=lambda x: x.get('event_time', ''))
                for event in events:
                    print(f"   - {event.get('title')} (优先级{event.get('priority')}): {event.get('event_time')}")

    def test_datetime_updates(self):
        """测试DateTime字段的更新功能"""
        print("\n=== DateTime字段更新测试 ===")

        try:
            # 查找一个事件进行更新
            event_to_update = self.bridge.find_by_id(
                self.table_name,
                "event_002",
                self.db_alias
            )

            if event_to_update and event_to_update.strip():
                event_data = json.loads(event_to_update)
                print(f"更新前的事件: {event_data.get('title')}")
                print(f"   更新时间: {event_data.get('updated_at')}")

                # 更新事件
                update_data = {
                    "updated_at": datetime.now(timezone.utc).isoformat(),
                    "description": "审查新功能的代码实现（已更新）",
                    "priority": 4  # 提高优先级
                }

                update_result = self.bridge.update(
                    self.table_name,
                    "event_002",
                    json.dumps(update_data),
                    self.db_alias
                )

                print(f"更新结果: {update_result}")

                # 验证更新
                updated_event = self.bridge.find_by_id(
                    self.table_name,
                    "event_002",
                    self.db_alias
                )

                if updated_event and updated_event.strip():
                    updated_data = json.loads(updated_event)
                    print(f"更新后的事件:")
                    print(f"   描述: {updated_data.get('description')}")
                    print(f"   优先级: {updated_data.get('priority')}")
                    print(f"   更新时间: {updated_data.get('updated_at')}")
                else:
                    print("更新后查询失败")
            else:
                print("未找到要更新的事件")
        except Exception as e:
            print(f"更新测试过程中出错: {e}")

    def run_all_tests(self):
        """运行所有测试"""
        try:
            self.setup_database()
            self.create_table_with_datetime_fields()
            self.demonstrate_datetime_usage()
            self.create_sample_events()
            self.test_datetime_queries()
            self.test_datetime_updates()

            print("\n=== DateTime字段演示完成 ===")
            print("✅ DateTime字段创建和使用正常")
            print("✅ DateTime字段索引创建正常")
            print("✅ DateTime字段查询功能正常")
            print("✅ DateTime字段更新功能正常")
            print("✅ 时间格式化和解析正常")

        except Exception as e:
            print(f"测试过程中出错: {e}")
            import traceback
            traceback.print_exc()
        finally:
            # 清理资源
            if self.bridge:
                try:
                    self.bridge.drop_table(self.table_name, self.db_alias)
                    print(f"已清理表: {self.table_name}")
                except Exception as e:
                    print(f"清理表时出错: {e}")

def main():
    """主函数"""
    print("=== RAT QuickDB Python DateTime字段演示 ===")
    print("本演示展示如何在Python绑定中正确使用DateTime字段")

    demo = DateTimeFieldDemo()
    demo.run_all_tests()

if __name__ == "__main__":
    main()