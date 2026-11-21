@echo off
rem Uso: COMPILAR_VERSAO.bat 1.2.0-teste.1
if "%~1"=="" (
  echo Informe a versao. Exemplo: COMPILAR_VERSAO.bat 1.2.0-teste.1
  pause
  exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0COMPILAR_VERSAO.ps1" -Versao %1
pause
