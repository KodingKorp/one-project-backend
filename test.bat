:: Load .env to set environment variables ::
@echo off
setlocal enabledelayedexpansion

:: Load .env file
for /f "usebackq tokens=1,2 delims==" %%a in (`findstr /r "^[^#]" .env`) do (
    set "%%a=%%b"
)
:: Print the loaded environment variables
echo Loaded environment variables:
for /f "tokens=1 delims==" %%a in ('set') do (
    if not "%%a"=="" (
        echo %%a=!%%a!
    )
)

:: docker-compose up ::
docker compose -f compose.yml up --build --force-recreate --remove-orphans