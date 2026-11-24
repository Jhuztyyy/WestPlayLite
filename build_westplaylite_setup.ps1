$ErrorActionPreference = "Stop"
$RepoRoot = "C:\Users\jhuzt\WestPlayLite"
if (-not (Test-Path $RepoRoot)) {
    throw "Repository not found at $RepoRoot"
}

Set-Location $RepoRoot
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Write-Host "=== 1. Fixing Companion / Setup CSS Layout ===" -ForegroundColor Cyan
$marker = "/* === WESTPLAYLITE LOW-END SETUP & COMPANION CSS === */"
Get-ChildItem "ui\src\companion" -Recurse -Filter "*.css" -ErrorAction SilentlyContinue | ForEach-Object {
    $css = Get-Content $_.FullName -Raw
    $idx = $css.IndexOf($marker)
    $baseCss = if ($idx -ge 0) { $css.Substring(0, $idx).TrimEnd() } else { $css.TrimEnd() }

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
    Set-Content -Path $_.FullName -Value ($baseCss + $cleanOverride) -NoNewline -Encoding UTF8
    Write-Host "  [CSS OK] $($_.Name)" -ForegroundColor Green
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

export function installUpdate(): Promise<void> {
  return Promise.resolve()
}

export function onUpdateProgress(_handler: (progress: Progress) => void): () => void {
  return () => {}
}

export function noteLines(_notes: string): string[] {
  return []
}
