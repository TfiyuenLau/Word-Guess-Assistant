#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "  [Web] 构建 Word-Guess-Assistant Web 静态发布包"
echo "=========================================="

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "1. 安装/检查前端依赖..."
if [ ! -d "node_modules" ]; then
  pnpm install --frozen-lockfile
fi

echo "2. 执行静态资源与 TypeScript 类型校验与编译..."
pnpm build

OUTPUT_DIR="$ROOT_DIR/release-artifacts/web"
mkdir -p "$OUTPUT_DIR"

echo "3. 打包 Web 静态产物为 zip 归档..."
cd "$ROOT_DIR/dist"
tar -czf "$OUTPUT_DIR/word-guess-assistant-web.tar.gz" .
if command -v zip >/dev/null 2>&1; then
  zip -r -q "$OUTPUT_DIR/word-guess-assistant-web.zip" .
fi

echo "=========================================="
echo "✅ Web 平台构建完成！产物路径:"
echo "   静态文件夹: $ROOT_DIR/dist"
echo "   归档文件:   $OUTPUT_DIR/word-guess-assistant-web.tar.gz"
echo "   可直接配合 Nginx、Docker、Cloudflare Pages、Vercel 或 GitHub Pages 部署上线！"
echo "=========================================="
