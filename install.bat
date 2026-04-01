@echo off
:: Sokuten (速貼) — Installer
:: Double-click to install. No Rust, no dependencies.
setlocal

set "INSTALL_DIR=%LOCALAPPDATA%\Sokuten"
set "EXE=%INSTALL_DIR%\sokuten.exe"
set "URL=https://github.com/S23Web3/sokuten/releases/latest/download/sokuten.exe"
set "SHA256=46cc574bfdc87b10a20052e9e4c7348c5a2d50360852b36c21a68490cbafcd01"
set "TMPFILE=%TEMP%\sokuten_download.exe"

echo.
echo  ============================================
echo    Sokuten  --  Windows text expander
echo    Installing...
echo  ============================================
echo.

:: --- Download -----------------------------------------------------------
echo  Downloading sokuten.exe from GitHub...
echo.
curl.exe -L --progress-bar --fail --output "%TMPFILE%" "%URL%"
if errorlevel 1 (
    echo.
    echo  [ERROR] Download failed. Check your internet connection.
    echo  Manual download: https://github.com/S23Web3/sokuten/releases
    goto :fail
)

:: --- Verify size (must be > 4 MB = 4000000 bytes) ----------------------
for %%F in ("%TMPFILE%") do set "FSIZE=%%~zF"
if %FSIZE% LSS 4000000 (
    echo.
    echo  [ERROR] Downloaded file is too small (%FSIZE% bytes^).
    echo  The file may be corrupted. Please try again.
    del "%TMPFILE%" 2>nul
    goto :fail
)

:: --- Verify SHA256 checksum --------------------------------------------
echo.
echo  Verifying file integrity...
for /f "tokens=1" %%H in ('certutil -hashfile "%TMPFILE%" SHA256 ^| findstr /v "hash" ^| findstr /v "CertUtil"') do set "ACTUAL=%%H"
if /i not "%ACTUAL%"=="%SHA256%" (
    echo.
    echo  [ERROR] Checksum mismatch — file may be corrupted or tampered.
    echo  Expected: %SHA256%
    echo  Got:      %ACTUAL%
    del "%TMPFILE%" 2>nul
    goto :fail
)
echo  [OK] Checksum verified

:: --- Install -----------------------------------------------------------
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
move /y "%TMPFILE%" "%EXE%" >nul
echo  [OK] Installed to %INSTALL_DIR%

:: --- Start Menu shortcut -----------------------------------------------
powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $lnk = $ws.CreateShortcut([IO.Path]::Combine([Environment]::GetFolderPath('StartMenu'),'Programs','Sokuten.lnk')); $lnk.TargetPath = '%EXE%'; $lnk.Description = 'Sokuten text expander'; $lnk.Save()"
echo  [OK] Start Menu shortcut created

:: --- Startup (optional) ------------------------------------------------
echo.
choice /c YN /m "  Add Sokuten to Windows startup"
if errorlevel 2 goto :skip_startup
powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $lnk = $ws.CreateShortcut([IO.Path]::Combine([Environment]::GetFolderPath('Startup'),'Sokuten.lnk')); $lnk.TargetPath = '%EXE%'; $lnk.Description = 'Sokuten text expander'; $lnk.Save()"
echo  [OK] Added to startup
:skip_startup

echo.
echo  ============================================
echo    Done! Launching Sokuten now.
echo  ============================================
echo.
start "" "%EXE%"
endlocal
exit /b 0

:fail
echo.
pause
endlocal
exit /b 1
