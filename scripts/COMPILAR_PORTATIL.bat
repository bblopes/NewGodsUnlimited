@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0COMPILAR_PORTATIL.ps1"
exit /b %errorlevel%
