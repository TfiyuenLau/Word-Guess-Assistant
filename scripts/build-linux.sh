#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "  [Linux] 构建 Word-Guess-Assistant 原生发布包"
echo "=========================================="

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "1. 检查前端依赖与资源..."
if [ ! -d "node_modules" ]; then
  pnpm install --frozen-lockfile
fi

echo "2. 编译生产前端资产..."
pnpm build

echo "3. 编译 Linux 原生可执行二进制与安装包 (AppImage / DEB)..."
# 编译 release 二进制
cargo build --release --manifest-path src-tauri/Cargo.toml

# 使用 Tauri CLI 打包
pnpm tauri build --bundles deb,appimage || {
  echo "⚠️ 注意: 若系统缺失部分打包依赖(如 appstreamcli/linuxdeploy)，Tauri 二进制依然已成功就绪。"
}

OUTPUT_DIR="$ROOT_DIR/release-artifacts/linux"
mkdir -p "$OUTPUT_DIR"

# 收集二进制与 bundle
cp -f "$ROOT_DIR/src-tauri/target/release/word-guess-assistant" "$OUTPUT_DIR/word-guess-assistant-linux-x86_64" || true

if [ -d "$ROOT_DIR/src-tauri/target/release/bundle/deb" ]; then
  cp -rf "$ROOT_DIR/src-tauri/target/release/bundle/deb"/*.deb "$OUTPUT_DIR/" 2>/dev/null || true
fi

if [ -d "$ROOT_DIR/src-tauri/target/release/bundle/appimage" ]; then
  cp -rf "$ROOT_DIR/src-tauri/target/release/bundle/appimage"/*.AppImage "$OUTPUT_DIR/" 2>/dev/null || true
fi

echo "=========================================="
echo "✅ Linux 平台构建完成！产物输出至:"
echo "   $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR" 2>/dev/null || true
echo "=========================================="
