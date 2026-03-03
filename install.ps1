# Install ct from the latest GitHub release.
# Usage: irm https://raw.githubusercontent.com/apetcu/vibelens/main/install.ps1 | iex
# Or: .\install.ps1 -Dir $env:LOCALAPPDATA\vibelens

param(
    [string]$Dir = "$env:LOCALAPPDATA\vibelens\bin"
)

$ErrorActionPreference = "Stop"
$Repo = "apetcu/vibelens"
$Asset = "ct-windows-x86_64.exe"

# Only x86_64 is provided for Windows
if ($env:PROCESSOR_ARCHITECTURE -notmatch "AMD64") {
    Write-Host "Unsupported architecture. We only provide x86_64. Download from Releases or build from source."
    exit 1
}

$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{
    "Accept" = "application/vnd.github+json"
    "User-Agent" = "vibelens-install"
}
$tag = $release.tag_name
$url = "https://github.com/$Repo/releases/download/$tag/$Asset"

Write-Host "Installing ct $tag to $Dir ..."
New-Item -ItemType Directory -Force -Path $Dir | Out-Null
$dest = Join-Path $Dir "ct.exe"
Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing

Write-Host "Installed: $dest"
$pathDir = [Environment]::GetEnvironmentVariable("Path", "User")
if ($pathDir -notlike "*$Dir*") {
    [Environment]::SetEnvironmentVariable("Path", "$pathDir;$Dir", "User")
    $env:Path = "$env:Path;$Dir"
    Write-Host "Added $Dir to your user PATH. Restart the terminal and run \`ct\`."
} else {
    Write-Host "Run \`ct\` from a new terminal."
}
