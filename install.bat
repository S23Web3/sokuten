@echo off
:: Sokuten (速貼) Installer
:: Downloads and installs Sokuten to your local AppData folder.

echo.
echo  ========================================
echo    Sokuten (速貼) Installer
echo    Windows text expander
echo  ========================================
echo.

set "INSTALL_DIR=%LOCALAPPDATA%\Sokuten"
set "EXE=%INSTALL_DIR%\sokuten.exe"
set "REPO=S23Web3/sokuten"
set "DOWNLOAD_URL=https://github.com/%REPO%/releases/latest/download/sokuten.exe"

:: Create install directory
if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
    echo  [+] Created %INSTALL_DIR%
) else (
    echo  [i] Install directory already exists
)

:: Download the latest release
echo.
echo  Downloading sokuten.exe ...
echo.
powershell -Command "try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '%DOWNLOAD_URL%' -OutFile '%EXE%' -UseBasicParsing; Write-Host '  [+] Download complete' } catch { Write-Host '  [!] Download failed:' $_.Exception.Message; exit 1 }"
if errorlevel 1 (
    echo.
    echo  Download failed. Please check your internet connection
    echo  or download manually from:
    echo  https://github.com/%REPO%/releases
    echo.
    pause
    exit /b 1
)

:: Create Start Menu shortcut
echo.
echo  Creating Start Menu shortcut ...
powershell -Command "$ws = New-Object -ComObject WScript.Shell; $sc = $ws.CreateShortcut([System.IO.Path]::Combine([Environment]::GetFolderPath('StartMenu'), 'Programs', 'Sokuten.lnk')); $sc.TargetPath = '%EXE%'; $sc.Description = 'Sokuten text expander'; $sc.Save(); Write-Host '  [+] Shortcut created'"

:: Ask about startup
echo.
set /p STARTUP="  Run Sokuten on Windows startup? (Y/N): "
if /i "%STARTUP%"=="Y" (
    powershell -Command "$ws = New-Object -ComObject WScript.Shell; $sc = $ws.CreateShortcut([System.IO.Path]::Combine([Environment]::GetFolderPath('Startup'), 'Sokuten.lnk')); $sc.TargetPath = '%EXE%'; $sc.Description = 'Sokuten text expander'; $sc.Save(); Write-Host '  [+] Added to startup'"
)

echo.
echo  ========================================
echo    Installation complete!
echo.
echo    Installed to: %INSTALL_DIR%
echo    Start Menu:   Sokuten
echo.
echo    To run now, press any key.
echo  ========================================
echo.
pause

start "" "%EXE%"
