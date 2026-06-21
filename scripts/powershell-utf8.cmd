@echo off
setlocal

set "PS_UTF8_INIT=chcp 65001 > $null; $utf8NoBom = [System.Text.UTF8Encoding]::new($false); [Console]::InputEncoding = $utf8NoBom; [Console]::OutputEncoding = $utf8NoBom; $OutputEncoding = $utf8NoBom; $env:PYTHONUTF8 = '1'; $env:PYTHONIOENCODING = 'utf-8'; $env:LANG = 'C.UTF-8'; $PSDefaultParameterValues['Get-Content:Encoding'] = 'UTF8';"

powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -Command "%PS_UTF8_INIT% %*"
exit /b %ERRORLEVEL%
