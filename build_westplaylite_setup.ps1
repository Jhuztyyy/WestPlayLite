$ErrorActionPreference = "Stop"
$RepoRoot = "C:\Users\jhuzt\WestPlayLite"
if (-not (Test-Path $RepoRoot)) {
    throw "Repository not found at $RepoRoot"
}
Set-Location $RepoRoot
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Write-Host "=== 1. Removing old Low-End CSS overrides (they broke the companion layout) ===" -ForegroundColor Cyan
$markers = @(
    "/* === WESTPLAYLITE LOW-END SETUP & COMPANION CSS === */",
    "/* === WESTPLAYLITE LOW-END GPU OVERRIDES === */"
)
Get-ChildItem "ui\src\companion" -Recurse -Filter "*.css" | ForEach-Object {
    $css = Get-Content $_.FullName -Raw
    $cuts = @()
    foreach ($m in $markers) {
        $i = $css.IndexOf($m)
        if ($i -ge 0) { $cuts += $i }
    }
    if ($cuts.Count -gt 0) {
        $idx = ($cuts | Measure-Object -Minimum).Minimum
        Set-Content -Path $_.FullName -Value $css.Substring(0, $idx).TrimEnd() -NoNewline -Encoding UTF8
        Write-Host "  [CSS cleaned] $($_.Name)" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "=== 2. Closing running WestPlay / WebView2 processes ===" -ForegroundColor Cyan
Get-Process -Name "westplay", "msedgewebview2" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

Write-Host ""
Write-Host "=== 3. Building release UI and the whole workspace ===" -ForegroundColor Cyan
npm --prefix ui run build:release
if ($LASTEXITCODE -ne 0) { throw "UI build failed." }
cargo build --release --workspace
if ($LASTEXITCODE -ne 0) { throw "Workspace cargo build failed." }

Write-Host ""
Write-Host "=== 4. Staging plugin DLLs and helper binaries ===" -ForegroundColor Cyan
$releaseDir = Join-Path $RepoRoot "target\release"
$stageDir = Join-Path $releaseDir "westplaylite_res"
if (Test-Path $stageDir) { Remove-Item $stageDir -Recurse -Force }
New-Item -ItemType Directory -Path $stageDir -Force | Out-Null
$staged = @()
Get-ChildItem $releaseDir -Filter "*.dll" -File -ErrorAction SilentlyContinue | ForEach-Object {
    Copy-Item $_.FullName -Destination (Join-Path $stageDir $_.Name) -Force
    $staged += $_.Name
}
Get-ChildItem $releaseDir -Filter "westplay-*.exe" -File -ErrorAction SilentlyContinue | ForEach-Object {
    Copy-Item $_.FullName -Destination (Join-Path $stageDir $_.Name) -Force
    $staged += $_.Name
}
if ($staged.Count -gt 0) {
    Write-Host "  [Staged] $($staged -join ', ')" -ForegroundColor Green
} else {
    Write-Warning "No .dll or westplay-*.exe files found in target\release."
}

Write-Host ""
Write-Host "=== 5. Building the NSIS installer (*-setup.exe) ===" -ForegroundColor Cyan
# Windows PowerShell drops the quotes of a JSON argument, so the override goes through a file (paths are
# relative to app\, where tauri.conf.json lives).
$overrideFile = Join-Path $RepoRoot "app\tauri.lite-override.json"
$override = '{"bundle":{"active":true,"resources":{"../ui/dist/":"ui/","../target/release/westplaylite_res/":"./"}}}'
[System.IO.File]::WriteAllText($overrideFile, $override, (New-Object System.Text.UTF8Encoding($false)))
npx @tauri-apps/cli@2 build --bundles nsis --config $overrideFile
$tauriExit = $LASTEXITCODE
Remove-Item $overrideFile -Force -ErrorAction SilentlyContinue
if ($tauriExit -ne 0) { throw "Tauri NSIS build failed." }

$nsisDir = Join-Path $RepoRoot "target\release\bundle\nsis"
$setupExe = Get-ChildItem $nsisDir -Filter "*-setup.exe" -File -ErrorAction SilentlyContinue | Select-Object -First 1
Write-Host ""
if ($setupExe) {
    Write-Host "[DONE] Installer: $($setupExe.FullName)" -ForegroundColor Green
    explorer.exe $nsisDir
} else {
    Write-Warning "Build finished, but no installer found in $nsisDir"
}
