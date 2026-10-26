@echo off
rem ------------------------------------------------------------
rem Script: run-windows.cmd
rem Purpose: Start the Rust server and the Vite client, each in
rem          its own window, so the table can be opened locally.
rem Usage:   scripts\run-windows.cmd
rem ------------------------------------------------------------
setlocal

where cargo >nul 2>&1
if errorlevel 1 (
  echo cargo was not found. Install Rust from https://rustup.rs and open a new terminal.
  exit /b 1
)
where npm >nul 2>&1
if errorlevel 1 (
  echo npm was not found. Install Node.js from https://nodejs.org and open a new terminal.
  exit /b 1
)

pushd "%~dp0.."
if errorlevel 1 (
  echo Could not find the project folder.
  exit /b 1
)
set "ROOT=%CD%"
popd

echo Starting the server and the client in new windows.
echo The server window prints the table passphrase and the GM key.
echo Open http://127.0.0.1:5173 when the client window is ready.
echo Close both windows to stop the table.

start "Longtable server" /D "%ROOT%" cmd /k cargo run --manifest-path server/Cargo.toml
start "Longtable client" /D "%ROOT%\client" cmd /k "npm install && npm run dev"

endlocal
