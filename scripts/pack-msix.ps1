param(
  [switch]$GenerateDevCert,
  [switch]$InstallDevCert
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$outputRoot = Join-Path $repoRoot "dist-msix"
$stageRoot = Join-Path $outputRoot "stage\x64"
$exeName = "regex-sweep.exe"
$exePath = Join-Path $repoRoot "src-tauri\target\release\$exeName"
$manifestPath = Join-Path $repoRoot "Package.appxmanifest"
$logoPath = Join-Path $repoRoot "logo.png"
$msixName = "RegexSweepDesktop_0.1.0.0_x64.msix"
$msixPath = Join-Path $outputRoot $msixName

Set-Location $repoRoot

if (-not (Get-Command "winapp" -ErrorAction SilentlyContinue)) {
  throw "winapp CLI was not found. Install it with winget install Microsoft.winappcli --source winget, or run this in GitHub Actions with microsoft/setup-WinAppCli."
}

npm run tauri -- build --no-bundle

Remove-Item -Recurse -Force $stageRoot -ErrorAction SilentlyContinue
Remove-Item -Force $msixPath -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $stageRoot | Out-Null
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

if (-not (Test-Path -LiteralPath $exePath)) {
  throw "Expected Windows executable was not produced: $exePath"
}

Copy-Item -LiteralPath $exePath -Destination (Join-Path $stageRoot $exeName)

winapp manifest update-assets $logoPath --manifest $manifestPath

$packArgs = @(
  $stageRoot,
  "--manifest",
  $manifestPath,
  "--executable",
  $exeName,
  "--output",
  $msixName
)

if ($GenerateDevCert) {
  $packArgs += "--generate-cert"
}

if ($InstallDevCert) {
  $packArgs += "--install-cert"
}

Set-Location $outputRoot
winapp pack @packArgs

if (-not (Test-Path -LiteralPath $msixPath)) {
  throw "MSIX packaging completed without creating $msixPath"
}

Write-Host "Created $msixPath"
