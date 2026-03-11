$ErrorActionPreference = 'Stop'

$ApiUrl = 'https://api.github.com/repos/xidl/xidl/releases/latest'

$arch = $Env:PROCESSOR_ARCHITECTURE
switch ($arch) {
  'AMD64' { $ArchTag = 'x86_64' }
  'ARM64' { $ArchTag = 'aarch64' }
  default { throw "Unsupported architecture: $arch" }
}

$Artifact = "xidlc-$ArchTag-pc-windows-gnu.tar.gz"

$release = Invoke-RestMethod -Uri $ApiUrl -Headers @{ 'User-Agent' = 'xidl-installer' }
$asset = $release.assets | Where-Object { $_.name -eq $Artifact } | Select-Object -First 1
if (-not $asset) {
  throw "Artifact not found in latest release: $Artifact"
}

$sha = $null
$lines = ($release.body -split "`n")
for ($i = 0; $i -lt $lines.Length; $i++) {
  if ($lines[$i] -match [Regex]::Escape($Artifact)) {
    for ($j = $i + 1; $j -lt [Math]::Min($i + 6, $lines.Length); $j++) {
      if ($lines[$j] -match 'sha256[:\s]*([0-9a-fA-F]{64})') {
        $sha = $Matches[1].ToLower()
        break
      }
    }
  }
  if ($sha) { break }
}
if (-not $sha) {
  foreach ($line in $lines) {
    if ($line -match "${Artifact}.*sha256[:\s]*([0-9a-fA-F]{64})") {
      $sha = $Matches[1].ToLower()
      break
    }
  }
}
if (-not $sha) {
  throw "sha256 not found in latest release notes for: $Artifact"
}

$TempDir = New-Item -ItemType Directory -Force -Path ([System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString()))
$ArchivePath = Join-Path $TempDir $Artifact

Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $ArchivePath

$hash = (Get-FileHash -Algorithm SHA256 -Path $ArchivePath).Hash.ToLower()
if ($hash -ne $sha) {
  throw "sha256 mismatch: expected $sha, got $hash"
}

$InstallDir = Join-Path $Env:USERPROFILE '.local\bin'
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

tar -xzf $ArchivePath -C $InstallDir

$ExePath = Join-Path $InstallDir 'xidlc.exe'
if (-not (Test-Path $ExePath)) {
  throw "xidlc.exe not found after extraction"
}

Write-Host "Installed xidlc to $ExePath (release $($release.tag_name))"
if ($Env:PATH -notlike "*$InstallDir*") {
  Write-Host "Add $InstallDir to your PATH, e.g.:"
  Write-Host "  setx PATH \"$InstallDir;`$Env:PATH\""
}
