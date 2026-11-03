#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "  [Android] 构建 Word-Guess-Assistant 安卓安装包 (APK/AAB)"
echo "=========================================="

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# 1. 环境变量自适应加载
if [ -z "${JAVA_HOME:-}" ]; then
  if [ -d "$HOME/.local/jdk-17" ]; then
    export JAVA_HOME="$HOME/.local/jdk-17"
    export PATH="$JAVA_HOME/bin:$PATH"
    echo "ℹ️  自动加载 OpenJDK 17: $JAVA_HOME"
  elif command -v javac >/dev/null 2>&1; then
    JAVA_BIN="$(readlink -f "$(command -v javac)")"
    export JAVA_HOME="$(dirname "$(dirname "$JAVA_BIN")")"
    echo "ℹ️  从系统路径探测到 JAVA_HOME: $JAVA_HOME"
  else
    echo "❌ 错误: 未找到 Java 17+。请设置 JAVA_HOME 或运行: sudo apt-get install openjdk-17-jdk"
    exit 1
  fi
fi

if [ -z "${ANDROID_HOME:-}" ]; then
  if [ -d "$HOME/Android/Sdk" ]; then
    export ANDROID_HOME="$HOME/Android/Sdk"
  elif [ -d "$HOME/.local/android-sdk" ]; then
    export ANDROID_HOME="$HOME/.local/android-sdk"
  elif [ -d "/mnt/c/Users/$USER/AppData/Local/Android/Sdk" ]; then
    export ANDROID_HOME="/mnt/c/Users/$USER/AppData/Local/Android/Sdk"
  fi
fi

if [ -z "${NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ]; then
  # 探测 SDK 内部的 ndk
  LATEST_NDK=$(ls -d "$ANDROID_HOME/ndk/"* 2>/dev/null | sort -V | tail -n 1 || true)
  if [ -n "$LATEST_NDK" ]; then
    export NDK_HOME="$LATEST_NDK"
  fi
fi

echo "📋 编译环境检查:"
echo "   JAVA_HOME:    ${JAVA_HOME:-'未检测到'}"
echo "   ANDROID_HOME: ${ANDROID_HOME:-'未检测到'}"
echo "   NDK_HOME:     ${NDK_HOME:-'未检测到'}"

# 2. 确保已生成 Android 工程
if [ ! -d "$ROOT_DIR/src-tauri/gen/android" ]; then
  echo "2. 初始化 Android Gradle 工程..."
  pnpm tauri android init --ci --skip-targets-install
fi

# 3. 编译前端资源
echo "3. 编译前端生产资源..."
pnpm build

# 4. 执行 Android 打包
OUTPUT_DIR="$ROOT_DIR/release-artifacts/android"
mkdir -p "$OUTPUT_DIR"

BUILD_MODE="${1:-release}"

if [ "$BUILD_MODE" = "debug" ]; then
  echo "4. 正在编译 Debug APK..."
  pnpm tauri android build --debug || {
    echo "⚠️ Gradle 直接编译或使用本地构建..."
  }
else
  echo "4. 正在编译 Release APK / AAB..."
  pnpm tauri android build --apk || {
    echo "⚠️ 提示: 若本地无完整 Android NDK 交叉编译工具链，可通过 GitHub Actions 云端流水线全自动完成 APK/AAB 签名打包。"
  }
fi

# 搜集生成的 APK 文件
APK_SRC_DIR="$ROOT_DIR/src-tauri/gen/android/app/build/outputs/apk"
if [ -d "$APK_SRC_DIR" ]; then
  find "$APK_SRC_DIR" -name "*.apk" -exec cp -f {} "$OUTPUT_DIR/" \;
fi

AAB_SRC_DIR="$ROOT_DIR/src-tauri/gen/android/app/build/outputs/bundle"
if [ -d "$AAB_SRC_DIR" ]; then
  find "$AAB_SRC_DIR" -name "*.aab" -exec cp -f {} "$OUTPUT_DIR/" \;
fi

echo "=========================================="
echo "✅ Android 脚本流程执行完毕！输出目录:"
echo "   $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR" 2>/dev/null || true
echo "=========================================="

