$ErrorActionPreference = 'Stop'

$ApiUrl = 'https://api.github.com/repos/xidl/xidl/releases/latest'

$arch = $Env:PROCESSOR_ARCHITECTURE
switch ($arch) {
  'AMD64' { $ArchTag = 'x86_64' }
  'ARM64' { $ArchTag = 'aarch64' }
  default { throw "Unsupported architecture: $arch" }
}

$PreferredArtifacts = @("xidlc-$ArchTag-pc-windows-msvc.zip")
if ($ArchTag -eq 'x86_64') {
  $PreferredArtifacts += "xidlc-$ArchTag-pc-windows-gnu.tar.gz"
}

$release = Invoke-RestMethod -Uri $ApiUrl -Headers @{ 'Accept' = 'application/vnd.github+json'; 'User-Agent' = 'xidl-installer' }
$asset = $null
foreach ($candidate in $PreferredArtifacts) {
  $asset = $release.assets | Where-Object { $_.name -eq $candidate } | Select-Object -First 1
  if ($asset) {
    break
  }
}
if (-not $asset) {
  throw "Artifact not found in latest stable release: $($PreferredArtifacts -join ', ')"
}

$sha = $null
if ($asset.digest -and $asset.digest.StartsWith('sha256:')) {
  $sha = $asset.digest.Substring(7).ToLower()
}
if (-not $sha) {
  $lines = ($release.body -split "`n")
  for ($i = 0; $i -lt $lines.Length; $i++) {
    if ($lines[$i] -match [Regex]::Escape($asset.name)) {
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
      if ($line -match ([Regex]::Escape($asset.name) + '.*sha256[:\s]*([0-9a-fA-F]{64})')) {
        $sha = $Matches[1].ToLower()
        break
      }
    }
  }
}

$TempDir = New-Item -ItemType Directory -Force -Path ([System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString()))
$ArchivePath = Join-Path $TempDir $asset.name

Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $ArchivePath

if ($sha) {
  $hash = (Get-FileHash -Algorithm SHA256 -Path $ArchivePath).Hash.ToLower()
  if ($hash -ne $sha) {
    throw "sha256 mismatch: expected $sha, got $hash"
  }
}

$InstallDir = Join-Path $Env:USERPROFILE '.local\bin'
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

if ($asset.name.EndsWith('.zip')) {
  Expand-Archive -Path $ArchivePath -DestinationPath $InstallDir -Force
} else {
  tar -xzf $ArchivePath -C $InstallDir
}

$ExePath = Join-Path $InstallDir 'xidlc.exe'
if (-not (Test-Path $ExePath)) {
  throw "xidlc.exe not found after extraction"
}

Write-Host "Installed xidlc to $ExePath (release $($release.tag_name))"
if ($Env:PATH -notlike "*$InstallDir*") {
  Write-Host "Add $InstallDir to your PATH, e.g.:"
  Write-Host "  setx PATH ""$InstallDir;`$Env:PATH"""
}
