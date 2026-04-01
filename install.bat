@echo off
:: Sokuten (速貼) — Installer
:: Downloads, verifies, and installs. Rolls back cleanly on any error.
setlocal enabledelayedexpansion

set "INSTALL_DIR=%LOCALAPPDATA%\Sokuten"
set "EXE=%INSTALL_DIR%\sokuten.exe"
set "URL=https://github.com/S23Web3/sokuten/releases/latest/download/sokuten.exe"
set "SHA256=c711d4e52ff41d75bd8f206f7b60db88b471d9ad9661d7b791da9f465f2785ca"
set "TMPFILE=%TEMP%\sokuten_download.exe"
set "STARTMENU=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Sokuten.lnk"
set "STARTUPLNK=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\Sokuten.lnk"

:: Track what we've done so we can roll back
set "DONE_MKDIR=0"
set "DONE_INSTALL=0"
set "DONE_STARTMENU=0"
set "DONE_STARTUP=0"

echo.
echo  ============================================
echo    Sokuten  --  Windows text expander
echo    Installing...
echo  ============================================
echo.

:: --- Download to temp ---------------------------------------------------
echo  Downloading sokuten.exe from GitHub...
echo.
curl.exe -L --progress-bar --fail --output "%TMPFILE%" "%URL%"
if errorlevel 1 (
    echo.
    echo  [ERROR] Download failed. Check your internet connection.
    echo  Manual download: https://github.com/S23Web3/sokuten/releases
    del "%TMPFILE%" 2>nul
    goto :rollback
)

:: --- Verify size (must be > 4 MB) ---------------------------------------
for %%F in ("%TMPFILE%") do set "FSIZE=%%~zF"
if !FSIZE! LSS 4000000 (
    echo.
    echo  [ERROR] Downloaded file is too small (!FSIZE! bytes^).
    echo  The release asset may be corrupted. Please try again.
    del "%TMPFILE%" 2>nul
    goto :rollback
)

:: --- Verify SHA256 ------------------------------------------------------
echo.
echo  Verifying file integrity...
for /f "skip=1 tokens=1" %%H in ('certutil -hashfile "%TMPFILE%" SHA256') do (
    if not defined ACTUAL set "ACTUAL=%%H"
)
if /i not "%ACTUAL%"=="%SHA256%" (
    echo.
    echo  [ERROR] Checksum mismatch - file may be corrupted or tampered.
    echo  Expected: %SHA256%
    echo  Got:      %ACTUAL%
    del "%TMPFILE%" 2>nul
    goto :rollback
)
echo  [OK] Checksum verified

:: --- Create install directory -------------------------------------------
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
    if errorlevel 1 (
        echo  [ERROR] Could not create %INSTALL_DIR%
        del "%TMPFILE%" 2>nul
        goto :rollback
    )
    set "DONE_MKDIR=1"
)

:: --- Install exe --------------------------------------------------------
move /y "%TMPFILE%" "%EXE%" >nul
if errorlevel 1 (
    echo  [ERROR] Could not install to %INSTALL_DIR%
    del "%TMPFILE%" 2>nul
    goto :rollback
)
set "DONE_INSTALL=1"
echo  [OK] Installed to %INSTALL_DIR%

:: --- Start Menu shortcut ------------------------------------------------
powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $lnk = $ws.CreateShortcut('%STARTMENU%'); $lnk.TargetPath = '%EXE%'; $lnk.Description = 'Sokuten text expander'; $lnk.Save()"
if errorlevel 1 (
    echo  [WARNING] Could not create Start Menu shortcut - continuing anyway
) else (
    set "DONE_STARTMENU=1"
    echo  [OK] Start Menu shortcut created
)

:: --- Startup (optional) -------------------------------------------------
echo.
choice /c YN /m "  Add Sokuten to Windows startup"
if errorlevel 2 goto :skip_startup
powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $lnk = $ws.CreateShortcut('%STARTUPLNK%'); $lnk.TargetPath = '%EXE%'; $lnk.Description = 'Sokuten text expander'; $lnk.Save()"
if errorlevel 1 (
    echo  [WARNING] Could not create startup shortcut - continuing anyway
) else (
    set "DONE_STARTUP=1"
    echo  [OK] Added to startup
)
:skip_startup

:: --- Done ---------------------------------------------------------------
echo.
echo  ============================================
echo    Installation complete!
echo    Launching Sokuten now.
echo  ============================================
echo.
start "" "%EXE%"
endlocal
exit /b 0

:: --- Rollback -----------------------------------------------------------
:rollback
echo.
echo  Rolling back...

if "%DONE_STARTUP%"=="1" (
    del "%STARTUPLNK%" 2>nul
    echo  [--] Removed startup shortcut
)
if "%DONE_STARTMENU%"=="1" (
    del "%STARTMENU%" 2>nul
    echo  [--] Removed Start Menu shortcut
)
if "%DONE_INSTALL%"=="1" (
    del "%EXE%" 2>nul
    echo  [--] Removed installed exe
)
if "%DONE_MKDIR%"=="1" (
    rmdir "%INSTALL_DIR%" 2>nul
    echo  [--] Removed install folder
)

echo.
echo  Rollback complete. No files left behind.
echo.
pause
endlocal
exit /b 1
