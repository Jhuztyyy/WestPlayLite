<#
.SYNOPSIS
Builds a complete WestPlayLite Windows NSIS installer (*-setup.exe) for current and future versions.

.DESCRIPTION

Repairs companion/setup CSS so .ts and .ts-frame keep their proper positioning/scaling while
stripping expensive blurs, 3D hover tilt, shadows, and animations.

Disables upstream enes-less/WestPlay auto-updates so WestPlayLite stays on your build.

Builds the full Cargo workspace (--release --workspace) so plugin DLLs and helper binaries are compiled.

Stages all compiled workspace DLLs and helper binaries into target\release\westplaylite_res.

Runs Tauri v2 NSIS build with bundle.active=true and bundles the staged workspace binaries into *-setup.exe.
#>

$ErrorActionPreference = "Stop"
$RepoRoot = "C:\Users\jhuzt\WestPlayLite"
if (-not (Test-Path $RepoRoot)) {
throw "Repository not found at $RepoRoot"
}

Set-Location $RepoRoot$env:PATH = "$env:USERPROFILE.cargo\bin;$env:PATH"

Write-Host "=== 1. Fixing Companion / Setup CSS Layout ===" -ForegroundColor Cyan
$marker = "/* === WESTPLAYLITE LOW-END SETUP & COMPANION CSS === /"
Get-ChildItem "ui\src\companion" -Recurse -Filter ".css" -ErrorAction SilentlyContinue | ForEach-Object {
$css = Get-Content$_.FullName -Raw
$idx =$css.IndexOf($marker)$baseCss = if ($idx -ge 0) {$css.Substring(0, $idx).TrimEnd() } else {$css.TrimEnd() }

$cleanOverride = @"


$marker
.ob, .ob *, .ts, .ts * {
backdrop-filter: none !important;
-webkit-backdrop-filter: none !important;
animation: none !important;
transition: none !important;
box-shadow: none !important;
text-shadow: none !important;
}
.ts-light, .ts-scan, .ts-cursor {
display: none !important;
}
.ts-tilt {
transform: none !important;
}
.ts, .ts-bezel, .ts-glass, .ts-frame {
perspective: none !important;
filter: none !important;
will-change: auto !important;
}
"@
Set-Content -Path $.FullName -Value ($baseCss +$cleanOverride) -NoNewline -Encoding UTF8
Write-Host "  [CSS OK] $($.Name)" -ForegroundColor Green
}

Write-Host "`n=== 2. Disabling Upstream Auto-Updater ===" -ForegroundColor Cyan
$updateTsPath = "ui\src\companion\update.ts"
if (Test-Path $updateTsPath) {
$disabledUpdateTs = @'
import { useEffect, useState } from 'react'

export interface Offer {
version: string
notes: string
date: string | null
}

export interface Progress {
done: number
total: number | null
}

export function useUpdateOffer(): Offer | null {
const [offer] = useState<Offer | null>(null)
useEffect(() => {}, [])
return offer
}

export function lookForUpdate(): Promise<Offer | null> {
return Promise.resolve(null)
}

export function installUpdate(): Promise {
return Promise.resolve()
}

export function onUpdateProgress(_handler: (progress: Progress) => void): () => void {
return () => {}
}

export function noteLines(_notes: string): string[] {
return []
}
'@
Set-Content -Path $updateTsPath -Value $disabledUpdateTs -NoNewline -Encoding UTF8
Write-Host "  [Updater Disabled] ui\src\companion\update.ts" -ForegroundColor Green
}

Write-Host "`n=== 3. Closing Running WestPlay / WebView2 Processes ===" -ForegroundColor Cyan
Get-Process -Name "westplay","msedgewebview2" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

Write-Host "`n=== 4. Building Full Workspace & Release UI ===" -ForegroundColor Cyan
npm --prefix ui run build:release
if ($LASTEXITCODE -ne 0) { throw "UI build failed." }

cargo build --release --workspace
if ($LASTEXITCODE -ne 0) { throw "Workspace Cargo build failed." }

Write-Host "`n=== 5. Staging Plugin DLLs & Helper Binaries for Setup.exe ===" -ForegroundColor Cyan
$releaseDir = Join-Path$RepoRoot "target\release"
$stageDir = Join-Path$releaseDir "westplaylite_res"
if (Test-Path $stageDir) {
Remove-Item $stageDir -Recurse -Force
}
New-Item -ItemType Directory -Path $stageDir -Force | Out-Null

Collect all compiled DLLs and non-main westplay helper binaries from target/release

$stagedFiles = @()
Get-ChildItem $releaseDir -Filter ".dll" -File -ErrorAction SilentlyContinue | ForEach-Object {
Copy-Item $.FullName -Destination (Join-Path $stageDir$.Name) -Force
$stagedFiles +=$_.Name
}
Get-ChildItem $releaseDir -Filter "westplay.exe" -File -ErrorAction SilentlyContinue | Where-Object {
$.Name -ne "westplay.exe"
} | ForEach-Object {
Copy-Item $.FullName -Destination (Join-Path $stageDir$.Name) -Force
$stagedFiles +=$.Name
}

if ($stagedFiles.Count -gt 0) {
Write-Host "  [Staged] $($stagedFiles -join ', ')" -ForegroundColor Green
} else {
Write-Warning "No extra .dll or helper westplay.exe files found in target\release."
}

Write-Host "`n=== 6. Building NSIS Installer (*-setup.exe) ===" -ForegroundColor Cyan

Enable bundle.active=true and include staged workspace DLLs/helpers alongside ui/dist

$tauriConfigOverride = '{"bundle":{"active":true,"resources":{"../ui/dist/":"ui/","../target/release/westplaylite_res/":"./"}}}'
npx @tauri-apps/cli@2 build --bundles nsis --config $tauriConfigOverride
if ($LASTEXITCODE -ne 0) { throw "Tauri NSIS setup.exe build failed." }

$nsisDir = Join-Path $RepoRoot "target\release\bundle\nsis"
$setupExe = Get-ChildItem $nsisDir -Filter "*-setup.exe" -File -ErrorAction SilentlyContinue | Select-Object -First 1

Write-Host "`n[DONE] WestPlayLite setup build complete!" -ForegroundColor Green
if ($setupExe) {
Write-Host "Installer: $($setupExe.FullName)" -ForegroundColor Yellow
explorer.exe $nsisDir
} else {
Write-Warning "Build finished, check $nsisDir for generated installer files."
}