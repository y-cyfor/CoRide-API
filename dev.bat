@echo off
echo Deleting old database for fresh initialization...
del /q "%~dp0backend\data\coride.db" 2>nul
if exist "%~dp0backend\data\coride.db" (
    echo WARNING: Failed to delete database file, it may be in use.
) else (
    echo Database cleared.
)
echo.
echo [Backend] Starting Rust backend on port 8000...
start "CoRide-API Backend" cmd /k "cd /d %~dp0backend && cargo run"
echo.
echo [Frontend] Starting Vue frontend...
start "CoRide-API Frontend" cmd /k "cd /d %~dp0web && pnpm dev"
echo.
echo Both servers are starting in separate windows.