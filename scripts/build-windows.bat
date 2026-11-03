@echo off
REM ==========================================
REM  [Windows] Word-Guess-Assistant 构建批处理脚本
REM ==========================================
setlocal enabledelayedexpansion

echo ==========================================
echo   [Windows] 开始构建 Word-Guess-Assistant
echo ==========================================

cd /d "%~dp0.."

echo 1. 检查并安装前端依赖...
call pnpm install
if %ERRORLEVEL% NEQ 0 (
    echo [错误] pnpm install 执行失败
    exit /b %ERRORLEVEL%
)

echo 2. 编译前端资源...
call pnpm build
if %ERRORLEVEL% NEQ 0 (
    echo [错误] pnpm build 编译失败
    exit /b %ERRORLEVEL%
)

echo 3. 执行 Tauri Windows 打包...
call pnpm tauri build --bundles nsis,msi
if %ERRORLEVEL% NEQ 0 (
    echo [警告] Tauri 打包命令退出码异常，尝试检查生成目录...
)

if not exist "release-artifacts\windows" mkdir "release-artifacts\windows"
if exist "src-tauri\target\release\bundle\nsis\*.exe" copy /y "src-tauri\target\release\bundle\nsis\*.exe" "release-artifacts\windows\"
if exist "src-tauri\target\release\bundle\msi\*.msi" copy /y "src-tauri\target\release\bundle\msi\*.msi" "release-artifacts\windows\"
if exist "src-tauri\target\release\word-guess-assistant.exe" copy /y "src-tauri\target\release\word-guess-assistant.exe" "release-artifacts\windows\word-guess-assistant-portable.exe"

echo ==========================================
echo [完成] Windows 产物位于 release-artifacts\windows\
echo ==========================================
pause

