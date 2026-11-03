#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${1:-all}"

echo "=========================================================="
echo "  🎯 Word-Guess-Assistant 全平台编译构建管理器"
echo "  目标模式: $TARGET (可选: web | linux | android | all)"
echo "=========================================================="

case "$TARGET" in
  web)
    "$SCRIPT_DIR/build-web.sh"
    ;;
  linux)
    "$SCRIPT_DIR/build-linux.sh"
    ;;
  android)
    "$SCRIPT_DIR/build-android.sh" release
    ;;
  all)
    echo "▶️ [1/3] 开始构建 Web 平台版本..."
    "$SCRIPT_DIR/build-web.sh"

    echo "▶️ [2/3] 开始构建 Linux 原生版本..."
    "$SCRIPT_DIR/build-linux.sh"

    echo "▶️ [3/3] 检查 Android 平台打包..."
    "$SCRIPT_DIR/build-android.sh" release || {
      echo "ℹ️  Android 本地打包需要完整 Android SDK/NDK 环境；云端可直接触发 GitHub Actions 编译。"
    }
    ;;
  *)
    echo "❌ 未知参数: $TARGET"
    echo "用法: $0 [web|linux|android|all]"
    exit 1
    ;;
esac

echo "=========================================================="
echo "🎉 流程完成！所有产物可在 release-artifacts/ 下查看。"
echo "=========================================================="

