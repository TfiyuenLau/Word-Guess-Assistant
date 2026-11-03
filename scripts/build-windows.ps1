# ==========================================
#  [Windows] Word-Guess-Assistant 构建脚本 (PowerShell)
# ==========================================

$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  [Windows] 开始构建 Word-Guess-Assistant 安装包" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 切换到项目根目录
$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

# 1. 检查 Node.js / pnpm / Rust 编译环境
Write-Host "1. 检查编译环境..." -ForegroundColor Yellow
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Error "未检测到 Node.js，请先安装 Node.js LTS (v20+)"
}
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    Write-Host "未检测到 pnpm，正在通过 corepack 安装..." -ForegroundColor Yellow
    corepack enable
    corepack prepare pnpm@latest --activate
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "未检测到 Rust/Cargo，请先安装 rustup 及 MSVC C++ 构建工具"
}

# 2. 安装前端依赖
Write-Host "2. 安装前端依赖..." -ForegroundColor Yellow
pnpm install

# 3. 编译前端生产资源
Write-Host "3. 编译前端生产资源..." -ForegroundColor Yellow
pnpm build

# 4. 执行 Tauri 构建 (生成 NSIS .exe 安装包与 WiX .msi 安装包)
Write-Host "4. 执行 Tauri Windows 打包 (NSIS & MSI)..." -ForegroundColor Yellow
pnpm tauri build --bundles nsis,msi

# 5. 归档产物
$OutputDir = Join-Path $RootDir "release-artifacts\windows"
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$BundleDir = Join-Path $RootDir "src-tauri\target\release\bundle"
if (Test-Path (Join-Path $BundleDir "nsis")) {
    Copy-Item (Join-Path $BundleDir "nsis\*.exe") -Destination $OutputDir -Force
}
if (Test-Path (Join-Path $BundleDir "msi")) {
    Copy-Item (Join-Path $BundleDir "msi\*.msi") -Destination $OutputDir -Force
}
$RawExe = Join-Path $RootDir "src-tauri\target\release\word-guess-assistant.exe"
if (Test-Path $RawExe) {
    Copy-Item $RawExe -Destination (Join-Path $OutputDir "word-guess-assistant-portable.exe") -Force
}

Write-Host "==========================================" -ForegroundColor Green
Write-Host "✅ Windows 平台打包完成！产物输出目录:" -ForegroundColor Green
Write-Host "   $OutputDir" -ForegroundColor White
Get-ChildItem $OutputDir | Select-Object Name, Length, LastWriteTime
Write-Host "==========================================" -ForegroundColor Green

