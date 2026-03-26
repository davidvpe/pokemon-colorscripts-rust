# Install script for pokemon-colorscripts (Windows)
$ErrorActionPreference = 'Stop'

# Detect repo from git remote
try {
    $remote = git remote get-url origin 2>$null
    $repo = $remote -replace 'https://github.com/', '' -replace 'git@github.com:', '' -replace '\.git$', ''
} catch {
    Write-Error "Could not detect GitHub repo. Run from inside the cloned repository."
    exit 1
}

$url     = "https://github.com/$repo/releases/latest/download/pokemon-colorscripts-windows-x86_64.exe"
$installDir = "$env:LOCALAPPDATA\Programs\pokemon-colorscripts"
$dest    = "$installDir\pokemon-colorscripts.exe"

New-Item -ItemType Directory -Force -Path $installDir | Out-Null

Write-Host "Downloading pokemon-colorscripts-windows-x86_64.exe from $repo..."
Invoke-WebRequest -Uri $url -OutFile $dest

# Add install dir to user PATH if not already present
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
    Write-Host "Added $installDir to PATH (restart your shell to take effect)"
}

Write-Host "Installed to $dest"
