@echo off
:: Sokuten (速貼) Installer
:: Downloads and installs Sokuten to your local AppData folder.

echo.
echo  ========================================
echo    Sokuten Installer
echo    Windows text expander
echo  ========================================
echo.

set "INSTALL_DIR=%LOCALAPPDATA%\Sokuten"
set "EXE=%INSTALL_DIR%\sokuten.exe"
set "DOWNLOAD_URL=https://github.com/S23Web3/sokuten/releases/latest/download/sokuten.exe"

:: Create install directory
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
    echo  [+] Created %INSTALL_DIR%
) else (
    echo  [i] Install folder already exists
)

:: Download using curl.exe (built into Windows 10/11)
echo.
echo  Downloading sokuten.exe ...
echo.
curl.exe -L --progress-bar --output "%EXE%" "%DOWNLOAD_URL%"
if errorlevel 1 (
    echo.
    echo  [!] Download failed.
    echo.
    echo  Please download manually from:
    echo  https://github.com/S23Web3/sokuten/releases
    echo.
    pause
    exit /b 1
)

echo.
echo  [+] Download complete

:: Create Start Menu shortcut
echo  Creating Start Menu shortcut ...
powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $sc = $ws.CreateShortcut([System.IO.Path]::Combine([Environment]::GetFolderPath('StartMenu'), 'Programs', 'Sokuten.lnk')); $sc.TargetPath = '%EXE%'; $sc.Description = 'Sokuten text expander'; $sc.Save()"
echo  [+] Start Menu shortcut created

:: Ask about startup
echo.
set /p STARTUP="  Run Sokuten on Windows startup? (Y/N): "
if /i "%STARTUP%"=="Y" (
    powershell -NoProfile -Command "$ws = New-Object -ComObject WScript.Shell; $sc = $ws.CreateShortcut([System.IO.Path]::Combine([Environment]::GetFolderPath('Startup'), 'Sokuten.lnk')); $sc.TargetPath = '%EXE%'; $sc.Description = 'Sokuten text expander'; $sc.Save()"
    echo  [+] Added to startup
)

echo.
echo  ========================================
echo    Installation complete!
echo.
echo    Installed to: %INSTALL_DIR%
echo    Start Menu:   Sokuten
echo.
echo    Launching Sokuten now...
echo  ========================================
echo.
pause

start "" "%EXE%"
