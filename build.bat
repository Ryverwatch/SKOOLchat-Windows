@echo off
setlocal enabledelayedexpansion
title Building SKOOLchat Windows
cd /d "%~dp0"

echo === SKOOLchat Windows builder ===

rem --- 1. Fix folder layout automatically ---
if not exist Cargo.toml (
    echo [ERROR] Cargo.toml is missing from this folder: %cd%
    echo Put build.bat, Cargo.toml and main.rs together and try again.
    pause
    exit /b 1
)
if not exist src\main.rs (
    if exist main.rs (
        echo Fixing layout: moving main.rs into src folder...
        if not exist src mkdir src
        move /Y main.rs src\main.rs >nul
    ) else (
        echo [ERROR] main.rs was not found. Put it next to build.bat.
        pause
        exit /b 1
    )
)

rem --- 2. Make sure Rust is available, install it if not ---
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"
where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust not found. Trying to install it with winget...
    where winget >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] winget is not available. Install Rust manually from https://rustup.rs
        pause
        exit /b 1
    )
    winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
    set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"
    where cargo >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Rust was installed but is not ready yet.
        echo Close this window, open a new one, and run build.bat again.
        pause
        exit /b 1
    )
)

rem --- 3. Build, with automatic retry after updating dependencies ---
rem Link the C runtime into the exe so it runs on any PC with no extra installs
set "RUSTFLAGS=-C target-feature=+crt-static"
echo Building release version (first build downloads dependencies, please wait)...
cargo build --release > build.log 2>&1
if errorlevel 1 (
    echo First attempt failed. Updating dependencies and retrying...
    cargo update >> build.log 2>&1
    cargo build --release > build.log 2>&1
)

if errorlevel 1 (
    type build.log
    echo.
    findstr /i /c:"link.exe" /c:"linker" build.log >nul
    if not errorlevel 1 (
        echo Missing C++ build tools detected. Trying to install them with winget...
        winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
        echo Retrying build...
        cargo build --release > build.log 2>&1
    )
)

if errorlevel 1 (
    type build.log
    echo.
    echo [ERROR] Build still failing. The full log is saved in build.log
    echo Send that log to Claude and it will fix the code.
    pause
    exit /b 1
)

copy /Y target\release\SKOOLchat-Windows.exe "%cd%\SKOOLchat-Windows.exe" >nul

echo.
echo Build complete!
echo Your single app file is: %cd%\SKOOLchat-Windows.exe
echo You can copy just that one file anywhere and run it.
pause
