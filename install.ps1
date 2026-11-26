$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$repo = "Selfnify/alogian-code"
$dir = Join-Path $env:LOCALAPPDATA "Alogian\bin"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$base = "https://github.com/$repo/releases/latest/download"
Invoke-WebRequest -UseBasicParsing -Uri "$base/alogian-win32-x64.exe" -OutFile (Join-Path $dir "alogian.exe")
Invoke-WebRequest -UseBasicParsing -Uri "$base/ast-grep-win32-x64.exe" -OutFile (Join-Path $dir "ast-grep.exe")
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not (($userPath -split ";") -contains $dir)) {
  [Environment]::SetEnvironmentVariable("Path", ((($userPath.TrimEnd(";")) + ";" + $dir).TrimStart(";")), "User")
}
Write-Host "Alogian Code installed to $dir"
Write-Host "Open a new terminal and run: alogian"
