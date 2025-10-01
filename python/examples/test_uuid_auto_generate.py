#!/usr/bin/env python3
"""
Python绑定UUID自动生成测试示例

这个示例验证RAT QuickDB Python绑定的UUID自动生成功能
"""

import asyncio
import os
from rat_quickdb_py import *

async def main():
    print("=== Python绑定UUID自动生成测试 ===\n")

    # 清理可能存在的测试数据库文件
    db_path = "./test_python_uuid.db"
    if os.path.exists(db_path):
        os.remove(db_path)

    try:
        # 1. 创建数据库桥接器
        print("1. 创建数据库桥接器...")
        bridge = create_db_queue_bridge()

        # 2. 添加SQLite数据库，使用UUID策略
        print("2. 添加SQLite数据库（UUID策略）...")
        import json
        response = bridge.add_sqlite_database(
            alias="python_uuid_test",
            path=db_path,
            max_connections=5,
            min_connections=1,
            connection_timeout=30,
            idle_timeout=600,
            max_lifetime=3600,
            id_strategy="Uuid"  # 使用UUID策略
        )

        result = json.loads(response)
        if result.get("success"):
            print("   ✅ 数据库添加成功")
        else:
            print(f"   ❌ 数据库添加失败: {result.get('error')}")
            return

        # 等待连接建立
        await asyncio.sleep(1)

        # 3. 创建测试用户模型
        print("3. 定义测试用户模型...")

        # 字段定义
        fields = {
            "id": string_field(None, None, None).unique(),  # ID字段，允许自动生成
            "name": string_field(None, None, None).required(),
            "email": string_field(None, None, None).required().unique(),
            "age": integer_field(None, None).required(),
            "salary": float_field(None, None).required(),
            "active": boolean_field().required(),
            "department": string_field(None, None, None).required(),
            "created_at": datetime_field().required(),
            "profile": json_field(),
            "tags": array_field(FieldType.string(max_length=None, min_length=None), None, None)
        }

        # 索引定义
        indexes = [
            IndexDefinition(fields=["id"], unique=True, name="idx_id"),
            IndexDefinition(fields=["email"], unique=True, name="idx_email"),
            IndexDefinition(fields=["department"], unique=False, name="idx_department"),
            IndexDefinition(fields=["active"], unique=False, name="idx_active")
        ]

        # 创建模型元数据
        model_meta = ModelMeta(
            collection_name="python_test_users",
            fields=fields,
            indexes=indexes
        )

        # 4. 注册模型
        print("4. 注册模型...")
        register_response = bridge.register_model(model_meta)
        register_result = json.loads(register_response)

        if register_result.get("success"):
            print("   ✅ 模型注册成功")
        else:
            print(f"   ❌ 模型注册失败: {register_result.get('error')}")
            return

        # 5. 创建测试用户数据（不包含ID，让系统自动生成）
        print("\n5. 创建测试用户（ID自动生成）...")

        test_users = [
            {
                "name": "Python张三",
                "email": "python_zhangsan@test.com",
                "age": 28,
                "salary": 18000.50,
                "active": True,
                "department": "Python技术部",
                "created_at": "2025-10-01T10:00:00Z",
                "profile": {"position": "Python工程师", "skills": ["Python", "FastAPI", "Django"]},
                "tags": ["Python", "后端", "API"]
            },
            {
                "name": "Python李四",
                "email": "python_lisi@test.com",
                "age": 32,
                "salary": 25000.00,
                "active": False,
                "department": "Python产品部",
                "created_at": "2025-10-01T10:00:00Z",
                "profile": {"position": "产品经理", "skills": ["需求分析", "产品设计"]},
                "tags": ["产品", "设计"]
            },
            {
                "name": "Python王五",
                "email": "python_wangwu@test.com",
                "age": 26,
                "salary": 16000.75,
                "active": True,
                "department": "Python技术部",
                "created_at": "2025-10-01T10:00:00Z",
                "profile": {"position": "前端工程师", "skills": ["JavaScript", "React", "Vue"]},
                "tags": ["前端", "JavaScript", "React"]
            }
        ]

        created_user_ids = []

        for i, user_data in enumerate(test_users, 1):
            print(f"   创建用户 {i}: {user_data['name']}")

            # 创建用户（不包含ID，系统应该自动生成UUID）
            user_json = json.dumps(user_data)
            response = bridge.create("python_test_users", user_json, "python_uuid_test")
            result = json.loads(response)

            if result.get("success"):
                user_id = result.get("data")
                created_user_ids.append(user_id)
                print(f"   ✅ 创建成功，自动生成UUID: {user_id}")

                # 查看UUID的实际数据类型和内容
                print(f"   user_id类型: {type(user_id)}")
                print(f"   user_id内容: {repr(user_id)}")

                # 验证UUID格式
                import uuid as uuid_lib
                try:
                    # 去掉外层的引号
                    clean_id = user_id.strip('"')
                    uuid_obj = uuid_lib.UUID(clean_id)
                    print(f"   ✅ UUID格式验证通过: {uuid_obj}")
                except ValueError as e:
                    print(f"   ❌ UUID格式验证失败: {e}")
            else:
                print(f"   ❌ 创建失败: {result.get('error')}")
                return

        print(f"\n✅ 成功创建 {len(created_user_ids)} 个用户")

        # 6. 查询测试
        print("\n6. 查询测试...")

        # 查询所有用户
        empty_query = json.dumps({})
        response = bridge.find("python_test_users", empty_query, "python_uuid_test")
        result = json.loads(response)
        if result.get("success"):
            users = result.get("data", [])
            print(f"✅ 查询成功，找到 {len(users)} 个用户")

            for i, user in enumerate(users, 1):
                user_id = user.get("id", "未知ID")
                name = user.get("name", "未知姓名")
                age = user.get("age", 0)
                department = user.get("department", "未知部门")
                print(f"   {i}. {name} ({user_id}) - {age}岁 - {department}")
        else:
            print(f"❌ 查询失败: {result.error}")
            return

        # 7. 条件查询测试
        print("\n7. 条件查询测试...")

        # 查询激活用户
        active_query = json.dumps({"active": True})
        response = bridge.find("python_test_users", active_query, "python_uuid_test")
        result = json.loads(response)
        if result.get("success"):
            active_users = result.get("data", [])
            print(f"✅ 激活用户查询成功，找到 {len(active_users)} 个用户")
            for user in active_users:
                print(f"   - {user['name']} ({user['id']})")
        else:
            print(f"❌ 激活用户查询失败: {result.get('error')}")

        # 查询技术部用户
        dept_query = json.dumps({"department": "Python技术部"})
        response = bridge.find("python_test_users", dept_query, "python_uuid_test")
        result = json.loads(response)
        if result.get("success"):
            tech_users = result.get("data", [])
            print(f"✅ 技术部用户查询成功，找到 {len(tech_users)} 个用户")
            for user in tech_users:
                print(f"   - {user['name']} ({user['id']}) - 薪资: {user['salary']}")
        else:
            print(f"❌ 技术部用户查询失败: {result.get('error')}")

        # 8. 更新测试
        print("\n8. 更新测试...")

        if created_user_ids:
            first_user_id = created_user_ids[0]

            # 更新第一个用户
            update_data = {
                "salary": 20000.00,
                "department": "Python研发部",
                "active": True
            }

            # 使用条件查询更新，而不是按ID更新
            updates_json = json.dumps(update_data)
            condition_json = json.dumps({"id": first_user_id.strip('"')})
            response = bridge.update("python_test_users", condition_json, updates_json, "python_uuid_test")
            result = json.loads(response)
            if result.get("success"):
                print(f"✅ 用户更新成功: {first_user_id}")

                # 验证更新结果 - 使用find和ID条件查询
                verify_query = json.dumps({"id": first_user_id.strip('"')})
                response = bridge.find("python_test_users", verify_query, "python_uuid_test")
                verify_result = json.loads(response)
                if verify_result.get("success") and verify_result.get("data"):
                    users = verify_result.get("data", [])
                    if users:
                        updated_user = users[0]  # 取第一个结果
                        print(f"   更新后信息:")
                        print(f"   - 姓名: {updated_user['name']}")
                        print(f"   - 部门: {updated_user['department']}")
                        print(f"   - 薪资: {updated_user['salary']}")
                        print(f"   - 状态: {'激活' if updated_user['active'] else '未激活'}")
                    else:
                        print("❌ 更新验证查询失败：未找到用户")
                else:
                    print(f"❌ 更新验证查询失败: {verify_result.get('error')}")
            else:
                print(f"❌ 用户更新失败: {result.get('error')}")

        # 9. 删除测试
        print("\n9. 删除测试...")

        if len(created_user_ids) > 1:
            # 删除最后一个用户 - 使用条件查询
            last_user_id = created_user_ids[-1]
            delete_condition = json.dumps({"id": last_user_id.strip('"')})

            response = bridge.delete("python_test_users", delete_condition, "python_uuid_test")
            result = json.loads(response)
            if result.get("success"):
                print(f"✅ 用户删除成功: {last_user_id}")

                # 验证删除结果
                empty_query = json.dumps({})
                response = bridge.find("python_test_users", empty_query, "python_uuid_test")
                verify_result = json.loads(response)
                if verify_result.get("success"):
                    remaining_users = verify_result.get("data", [])
                    print(f"   删除后剩余用户数: {len(remaining_users)}")
                else:
                    print("❌ 删除验证查询失败")
            else:
                print(f"❌ 用户删除失败: {result.get('error')}")

        print("\n=== Python绑定UUID自动生成测试完成 ===")
        print("✅ 所有功能测试通过！")

    except Exception as e:
        print(f"❌ 测试过程中发生错误: {e}")
        import traceback
        traceback.print_exc()

    finally:
        # 清理测试数据库文件
        if os.path.exists(db_path):
            os.remove(db_path)
            print(f"清理测试数据库文件: {db_path}")

if __name__ == "__main__":
    asyncio.run(main())