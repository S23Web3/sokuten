@echo off
:: Sokuten (速貼) Uninstaller

echo.
echo  ========================================
echo    Sokuten (速貼) Uninstaller
echo  ========================================
echo.

set "INSTALL_DIR=%LOCALAPPDATA%\Sokuten"

:: Kill running instance
taskkill /f /im sokuten.exe >nul 2>&1

:: Remove Start Menu shortcut
set "STARTMENU=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Sokuten.lnk"
if exist "%STARTMENU%" (
    del "%STARTMENU%"
    echo  [+] Removed Start Menu shortcut
)

:: Remove Startup shortcut
set "STARTUP=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\Sokuten.lnk"
if exist "%STARTUP%" (
    del "%STARTUP%"
    echo  [+] Removed startup shortcut
)

:: Ask about data
echo.
set /p DELDATA="  Also delete saved phrases and settings? (Y/N): "
if /i "%DELDATA%"=="Y" (
    if exist "%INSTALL_DIR%" (
        rmdir /s /q "%INSTALL_DIR%"
        echo  [+] Deleted %INSTALL_DIR% and all data
    )
) else (
    if exist "%INSTALL_DIR%\sokuten.exe" (
        del "%INSTALL_DIR%\sokuten.exe"
        echo  [+] Removed sokuten.exe (phrases and config kept)
    )
)

echo.
echo  Uninstall complete.
echo.
pause
