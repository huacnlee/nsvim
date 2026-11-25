# Install the published nsvim release for the current Windows user.
[CmdletBinding()]
param(
    [string]$Version = $(if ($env:NSVIM_VERSION) { $env:NSVIM_VERSION } else { 'latest' }),
    [string]$InstallDir = $(if ($env:NSVIM_INSTALL_DIR) { $env:NSVIM_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\nsvim' }),
    # Leave the user PATH as it is. Then terminals and agents run nsvim.exe by its full path.
    [switch]$NoModifyPath = $($env:NSVIM_NO_MODIFY_PATH -eq '1')
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
if ($env:OS -ne 'Windows_NT') { throw 'This installer requires Windows. Use install.sh on macOS or Linux.' }
$architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$target = switch ($architecture) {
    'AMD64' { 'x86_64-pc-windows-msvc' }
    'ARM64' { throw 'Windows releases support x86_64 only. Build from source on ARM64.' }
    default { throw "Unsupported CPU architecture: $architecture" }
}
$repository = 'https://github.com/huacnlee/nsvim'
if ($Version -eq 'latest') {
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/huacnlee/nsvim/releases/latest'
    $Version = $release.tag_name
}
$Version = $Version -replace '^v', ''
if ($Version -notmatch '^\d+\.\d+\.\d+([.-][A-Za-z0-9.-]+)?$') { throw 'Version must be a release version such as 0.4.0 or v0.4.0.' }
$asset = "nsvim-$Version-$target.zip"
$base = "$repository/releases/download/v$Version"
$InstallDir = [IO.Path]::GetFullPath($InstallDir)
$parent = Split-Path -Parent $InstallDir
$work = Join-Path ([IO.Path]::GetTempPath()) ('nsvim-install-' + [Guid]::NewGuid().ToString('N'))
$staged = Join-Path $parent ('.nsvim-install-' + [Guid]::NewGuid().ToString('N'))
$previous = "$staged-previous"
New-Item -ItemType Directory -Path $work | Out-Null
try {
    Write-Host "Downloading nsvim $Version for $target..."
    $archive = Join-Path $work $asset
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile $archive
    } catch {
        throw "Release v$Version has no $asset. Use a later release."
    }
    $checksums = Join-Path $work 'SHA256SUMS'
    Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile $checksums
    $expected = @(Get-Content $checksums | ForEach-Object {
        if ($_ -match '^([a-fA-F0-9]{64})\s+\*?(.+)$' -and $Matches[2] -ceq $asset) { $Matches[1] }
    })
    if ($expected.Count -ne 1) { throw "Missing or ambiguous checksum for $asset" }
    if ((Get-FileHash $archive -Algorithm SHA256).Hash -ine $expected[0]) { throw 'Checksum verification failed. The installer installed nothing.' }
    $unpacked = Join-Path $work 'unpacked'
    # Expand-Archive refuses entries that go outside the destination folder.
    Expand-Archive -LiteralPath $archive -DestinationPath $unpacked
    # The program reads its interface from nsh\bundle.nshc next to nsvim.exe.
    foreach ($file in 'nsvim.exe', 'nsh\bundle.nshc') {
        if (-not (Test-Path -LiteralPath (Join-Path $unpacked $file) -PathType Leaf)) { throw "Release archive is missing $file." }
    }
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    # Replace the folder as one unit, so no file of the old version stays. Windows refuses to
    # move the folder while nsvim runs.
    Copy-Item -LiteralPath $unpacked -Destination $staged -Recurse
    if (Test-Path -LiteralPath $InstallDir) {
        try { Move-Item -LiteralPath $InstallDir -Destination $previous }
        catch { throw "Could not replace $InstallDir. Close nsvim, then run the installer again." }
    }
    try {
        Move-Item -LiteralPath $staged -Destination $InstallDir
    } catch {
        if (Test-Path -LiteralPath $previous) { Move-Item -LiteralPath $previous -Destination $InstallDir }
        throw "Could not replace $InstallDir."
    }
    $startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) 'nsvim.lnk'
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($startMenu)
    $shortcut.TargetPath = Join-Path $InstallDir 'nsvim.exe'
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.IconLocation = "$InstallDir\nsvim.exe,0"
    $shortcut.Save()
    Write-Host "Installed $InstallDir. Open nsvim from the Start menu."
    # Put the folder on the user PATH, so terminals, TUIs and AI agents can run nsvim.
    if ($NoModifyPath) {
        Write-Host "The program is $(Join-Path $InstallDir 'nsvim.exe')."
    } else {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $entries = @($userPath -split ';' | Where-Object { $_ })
        if ($entries -notcontains $InstallDir) {
            [Environment]::SetEnvironmentVariable('Path', (@($entries) + $InstallDir) -join ';', 'User')
        }
        if (($env:Path -split ';') -notcontains $InstallDir) { $env:Path = "$env:Path;$InstallDir" }
        Write-Host 'Run nsvim from a terminal. Open a new terminal first.'
        Write-Host 'To open nsvim with vim and vi, add this to your PowerShell $PROFILE: Set-Alias vim nsvim; Set-Alias vi nsvim'
    }
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $staged -Recurse -Force -ErrorAction SilentlyContinue
    # Keep the previous version when the new one is not in place.
    if (Test-Path -LiteralPath (Join-Path $InstallDir 'nsvim.exe')) {
        Remove-Item -LiteralPath $previous -Recurse -Force -ErrorAction SilentlyContinue
    }
}
