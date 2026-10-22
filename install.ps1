# Installs the KLYPIX network engine on its own for Windows (without Reason sign-in: names aren't verified). In PowerShell:
#
#   irm <this url> | iex                                                  install, then set up (start or join a team)
#   & ([scriptblock]::Create((irm <this url>))) <team link>               install, then join that team
#   & ([scriptblock]::Create((irm <this url>))) <team link> --as <name>   the same, non-interactive
#   & ([scriptblock]::Create((irm <this url>))) init <team> --as <name>   install and start a team, non-interactive
#
# From cmd or another shell, wrap it: powershell -c "<the line>".
# Environment: KLYPIX_NETWORK_BIN_DIR (default ~\.local\bin), KLYPIX_NETWORK_VERSION (default latest), KLYPIX_NETWORK_REPO,
# KLYPIX_NETWORK_DOWNLOAD_BASE (a mirror serving the release files), KLYPIX_NETWORK_NO_PROMPT (never ask anything: updates).
& {
  $ErrorActionPreference = 'Stop'
  $ProgressPreference = 'SilentlyContinue'
  function die($msg) { throw "klypix-network install: $msg" }

  $repo = if ($env:KLYPIX_NETWORK_REPO) { $env:KLYPIX_NETWORK_REPO } else { 'dahshanlabs/klypix-network' }
  $bin = if ($env:KLYPIX_NETWORK_BIN_DIR) { $env:KLYPIX_NETWORK_BIN_DIR } else { Join-Path $HOME '.local\bin' }
  $version = if ($env:KLYPIX_NETWORK_VERSION) { $env:KLYPIX_NETWORK_VERSION } else { 'latest' }

  $cpu = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
  $arch = switch ($cpu) { 'AMD64' { 'x86_64' } 'ARM64' { 'aarch64' } default { die "unsupported CPU: $cpu" } }
  $target = "$arch-pc-windows-msvc"

  if ($env:KLYPIX_NETWORK_DOWNLOAD_BASE) { $base = $env:KLYPIX_NETWORK_DOWNLOAD_BASE }
  elseif ($version -eq 'latest') { $base = "https://github.com/$repo/releases/latest/download" }
  else { $base = "https://github.com/$repo/releases/download/$version" }

  $tmp = Join-Path ([IO.Path]::GetTempPath()) "klypix-network-$([guid]::NewGuid().ToString('n'))"
  New-Item -ItemType Directory $tmp | Out-Null
  try {
    Write-Host "Downloading klypix-network for $target..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    $zip = Join-Path $tmp 'klypix-network.zip'
    try { Invoke-WebRequest -UseBasicParsing "$base/klypix-network-$target.zip" -OutFile $zip } catch { die "download failed ($base/klypix-network-$target.zip)" }
    try { Invoke-WebRequest -UseBasicParsing "$base/klypix-network-$target.zip.sha256" -OutFile "$zip.sha256" } catch { die 'checksum download failed; not installing' }
    $want = ([IO.File]::ReadAllText("$zip.sha256") -split '\s+')[0].ToLower()
    $file = [IO.File]::OpenRead($zip)
    try { $got = -join ([Security.Cryptography.SHA256]::Create().ComputeHash($file) | ForEach-Object { $_.ToString('x2') }) } finally { $file.Dispose() }
    if (-not $want -or $want -ne $got) { die 'checksum mismatch; not installing' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [IO.Compression.ZipFile]::ExtractToDirectory($zip, $tmp)

    $new = Join-Path $tmp 'klypix-network.exe'
    & $new --version | Out-Null
    if ($LASTEXITCODE) { die "the downloaded binary doesn't run here; not installing" }
    New-Item -ItemType Directory -Force $bin | Out-Null
    $exe = Join-Path $bin 'klypix-network.exe'
    # Windows can't replace a program that is running, but it can rename it: move the old one aside, and delete
    # the ones set aside before once nothing runs them.
    Get-ChildItem $bin -Filter 'klypix-network.*.old' | Remove-Item -Force -ErrorAction SilentlyContinue
    $old = $null
    if (Test-Path $exe) { $old = Join-Path $bin "klypix-network.$(Get-Random).old"; Move-Item $exe $old }
    try { Move-Item (Join-Path $tmp 'klypix-network.exe') $exe } catch { if ($old) { Move-Item $old $exe }; throw }
  } finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
  }
  Write-Host "Installed $exe ($(& $exe --version))"

  $path = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($path -split ';') -notcontains $bin) {
    [Environment]::SetEnvironmentVariable('Path', (@($path -split ';' | Where-Object { $_ }) + $bin) -join ';', 'User')
    $env:Path = "$env:Path;$bin"
    Write-Host "Added $bin to your PATH (open a new terminal to use it there)."
  }

  # Hand over to klypix-network: it asks a few questions (your name, start or join, which agents) when there's someone to ask.
  if ($args.Count -eq 0) {
    if ($env:KLYPIX_NETWORK_NO_PROMPT -or [Console]::IsInputRedirected) { Write-Host ''; Write-Host 'Next, in a terminal: klypix-network'; return }
    & $exe
  } elseif ($args[0] -match '^(-.*|init|join|setup)$') {
    & $exe @args
  } else {
    & $exe join @args
  }
  if ($LASTEXITCODE) { throw "klypix-network exited with code $LASTEXITCODE" }
} @args
