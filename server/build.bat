@echo off
echo ==========================================
echo Markdown Server Build Script
echo ==========================================

REM Check if Rust is installed
cargo --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Error: Cargo is not installed or not in PATH.
    exit /b 1
)

echo.
echo [1/4] Formatting code...
cargo fmt

echo.
echo [2/4] Analyzing code...
cargo clippy --no-deps

echo.
echo [3/4] Building project...
cargo build --release

echo.
echo [4/4] Done!
echo Output file: target\release\markdown-server.exe
echo ==========================================
