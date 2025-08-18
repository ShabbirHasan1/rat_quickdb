# RAT QuickDB Python 绑定构建脚本

.PHONY: help dev build test clean install example

# 默认目标
help:
	@echo "RAT QuickDB Python 绑定构建命令:"
	@echo "  dev      - 开发模式构建并安装到当前 Python 环境"
	@echo "  build    - 构建 wheel 包"
	@echo "  test     - 运行测试"
	@echo "  example  - 运行基本使用示例"
	@echo "  clean    - 清理构建文件"
	@echo "  install  - 安装到当前 Python 环境"
	@echo "  check    - 检查 Rust 代码"

# 开发模式构建
dev:
	@echo "开发模式构建 rat_quickdb Python 绑定..."
	maturin develop --features pyo3/extension-module
	@echo "构建完成！现在可以在 Python 中导入 rat_quickdb"

# 构建 wheel 包
build:
	@echo "构建 wheel 包..."
	maturin build --release --features pyo3/extension-module
	@echo "构建完成！wheel 包位于 target/wheels/ 目录"

# 运行测试
test: dev
	@echo "运行 Python 测试..."
	python -m pytest tests/ -v

# 运行示例
example: dev
	@echo "运行基本使用示例..."
	cd examples && python basic_usage.py

# 清理构建文件
clean:
	@echo "清理构建文件..."
	cargo clean
	rm -rf target/
	rm -rf dist/
	rm -rf *.egg-info/
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
	find . -name "*.pyc" -delete 2>/dev/null || true

# 安装到当前环境
install: build
	@echo "安装到当前 Python 环境..."
	pip install target/wheels/*.whl --force-reinstall

# 检查 Rust 代码
check:
	@echo "检查 Rust 代码..."
	cargo check --features pyo3/extension-module
	cargo clippy --features pyo3/extension-module

# 发布到 PyPI（需要配置认证）
publish: build
	@echo "发布到 PyPI..."
	maturin publish --features pyo3/extension-module

# 本地安装开发版本
dev-install: dev
	@echo "开发版本已安装到当前 Python 环境"
	@echo "可以使用 'import rat_quickdb' 导入模块"