#!/bin/bash
# SoloSoul Plugin Market — Git Hooks 安装脚本
# 一键配置 core.hooksPath，启用预提交自动生成 registry.json

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

echo "📦 安装 SoloSoul Plugin Market Git Hooks..."

# 配置 Git 使用 .githooks 目录
git config core.hooksPath .githooks

# 确保 hook 文件可执行
if [ -f .githooks/pre-commit ]; then
    chmod +x .githooks/pre-commit
    echo "   ✅ pre-commit hook 已启用"
else
    echo "   ⚠️  .githooks/pre-commit 不存在，请检查仓库完整性"
    exit 1
fi

# 检查 python3 可用性
echo ""
if command -v python3 >/dev/null 2>&1; then
    PYTHON_VERSION=$(python3 --version 2>/dev/null || echo "unknown")
    echo "   ✅ Python 已安装: $PYTHON_VERSION"
else
    echo "   ⚠️  python3 未安装！pre-commit hook 将跳过自动生成。"
    echo "      请安装 Python 3 以获得最佳开发体验。"
    echo "      macOS: brew install python3"
    echo "      Ubuntu: sudo apt install python3"
fi

echo ""
echo "🎉 Git Hooks 安装完成！"
echo "   后续提交时，若修改了 plugins/ 目录，将自动重新生成 registry.json。"
echo "   如需跳过 hook: git commit --no-verify"
