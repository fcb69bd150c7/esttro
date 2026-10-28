@echo off
setlocal
echo Starting Esttro Notepad...
cd /d "%~dp0ide"

where node >nul 2>nul
if errorlevel 1 (
    echo.
    echo [ERROR] Node.js is not installed or not in PATH.
    echo Install Node.js from https://nodejs.org to run Esttro.
    goto error
)

where npm >nul 2>nul
if errorlevel 1 (
    echo.
    echo [ERROR] npm is not installed or not in PATH.
    echo Install Node.js from https://nodejs.org to run Esttro.
    goto error
)

if not exist "node_modules" (
    echo [1/3] Installing dependencies...
    call npm install
    if errorlevel 1 goto error
)

if not exist "node_modules\electron\dist\electron.exe" (
    echo [2/3] Downloading Electron binary...
    call node node_modules\electron\install.js
    if errorlevel 1 goto error
)

if not exist "dist\index.html" (
    echo [3/3] Building frontend bundle...
    call npm run build
    if errorlevel 1 goto error
)

set "BINARY_ARG="
if not "%~1"=="" (
    set "BINARY_ARG=--binary=%~1"
) else (
    if exist "%~dp0bin\esttro.exe" (
        set "BINARY_ARG=--binary=%~dp0bin\esttro.exe"
    )
)

echo Launching IDE...
call "node_modules\.bin\electron.cmd" . %BINARY_ARG%
if errorlevel 1 goto error

goto end

:error
echo.
echo [ERROR] Failed to start Esttro Notepad.
pause

:end
