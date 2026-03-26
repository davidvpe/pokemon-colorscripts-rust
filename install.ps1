$ErrorActionPreference = 'Stop'

$repo      = "davidvpe/pokemon-colorscripts-rust"
$url       = "https://github.com/$repo/releases/latest/download/pokemon-colorscripts-windows-x86_64.exe"
$installDir = "$env:LOCALAPPDATA\Programs\pokemon-colorscripts"
$dest      = "$installDir\pokemon-colorscripts.exe"

Write-Host "Installing pokemon-colorscripts..."

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Invoke-WebRequest -Uri $url -OutFile $dest

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
    Write-Host "Added to PATH (restart your shell to take effect)"
}

Write-Host "Done! Run: pokemon-colorscripts --random"
