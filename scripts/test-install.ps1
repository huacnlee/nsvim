# Test install.ps1 with local release fixtures. The test mocks the downloads and the Start menu
# shortcut, so it never changes the real system.
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Run these installer tests on Windows.' }
$installer = Join-Path $PSScriptRoot 'install.ps1'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('nsvim-installer-test-' + [Guid]::NewGuid().ToString('N'))
$oldArchitecture = $env:PROCESSOR_ARCHITECTURE
$oldWowArchitecture = $env:PROCESSOR_ARCHITEW6432
# The mocks run in the child scope of the installer. They change one shared object, because a
# new $script: variable in that scope is not visible here.
$installerTestState = [PSCustomObject]@{
    ShortcutSaved = $null
    LatestCalls = 0
    DownloadCalls = 0
}

function Invoke-RestMethod {
    param([string]$Uri)
    if ($Uri -ne 'https://api.github.com/repos/huacnlee/nsvim/releases/latest') { throw "Unexpected API request: $Uri" }
    $installerTestState.LatestCalls++
    return @{ tag_name = 'v0.4.0' }
}

function Invoke-WebRequest {
    param([switch]$UseBasicParsing, [string]$Uri, [string]$OutFile)
    $prefix = 'https://github.com/huacnlee/nsvim/releases/download/'
    if (-not $Uri.StartsWith($prefix)) { throw "Unexpected download: $Uri" }
    $installerTestState.DownloadCalls++
    $file = Join-Path $testRoot ($Uri.Split('/')[-1])
    if (-not (Test-Path -LiteralPath $file)) { throw "404: $Uri" }
    Copy-Item -LiteralPath $file -Destination $OutFile
}

function New-Object {
    [CmdletBinding(DefaultParameterSetName = 'Net')]
    param(
        [Parameter(Mandatory, Position = 0, ParameterSetName = 'Net')][string]$TypeName,
        [Parameter(Position = 1, ParameterSetName = 'Net')][object[]]$ArgumentList,
        [Parameter(Mandatory, ParameterSetName = 'Com')][string]$ComObject,
        [System.Collections.IDictionary]$Property,
        [switch]$Strict
    )
    # The archive module can also make ordinary .NET objects when it loads.
    if ($PSCmdlet.ParameterSetName -eq 'Net') {
        return Microsoft.PowerShell.Utility\New-Object @PSBoundParameters
    }
    if ($ComObject -ne 'WScript.Shell') { throw "Unexpected COM object: $ComObject" }
    $shell = [PSCustomObject]@{}
    $shell | Add-Member -MemberType ScriptMethod -Name CreateShortcut -Value {
        param($Path)
        $shortcut = [PSCustomObject]@{ Path = $Path; TargetPath = ''; WorkingDirectory = ''; IconLocation = '' }
        $shortcut | Add-Member -MemberType ScriptMethod -Name Save -Value { $installerTestState.ShortcutSaved = $this }
        return $shortcut
    }
    return $shell
}

function Assert-True($Condition, $Message) {
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

function Assert-Fails($Action, $Message) {
    $failure = $null
    try { & $Action } catch { $failure = $_.Exception.Message }
    Assert-True ($null -ne $failure -and $failure.Contains($Message)) "Expected '$Message', received '$failure'"
}

function Write-Release {
    if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive }
    Compress-Archive -Path (Join-Path $payload '*') -DestinationPath $archive
    $digest = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $testRoot 'SHA256SUMS') -Value "$digest  $asset" -Encoding ascii
}

try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    $env:PROCESSOR_ARCHITECTURE = 'AMD64'
    $env:PROCESSOR_ARCHITEW6432 = $null
    $payload = Join-Path $testRoot 'payload'
    New-Item -ItemType Directory -Path (Join-Path $payload 'nsh') | Out-Null
    Set-Content -LiteralPath (Join-Path $payload 'nsvim.exe') -Value 'fixture executable v1'
    Set-Content -LiteralPath (Join-Path $payload 'nsh\bundle.nshc') -Value 'fixture bytecode'
    Set-Content -LiteralPath (Join-Path $payload 'nsh\app.json') -Value '{"id":"dev.nsvim.app"}'
    Set-Content -LiteralPath (Join-Path $payload 'LICENSE') -Value 'fixture license'
    $asset = 'nsvim-0.4.0-x86_64-pc-windows-msvc.zip'
    $archive = Join-Path $testRoot $asset
    Write-Release
    $destination = Join-Path $testRoot 'install with spaces\nsvim'
    $exe = Join-Path $destination 'nsvim.exe'

    & $installer -InstallDir $destination
    Assert-True ($installerTestState.LatestCalls -eq 1) 'latest release lookup'
    Assert-True ($installerTestState.DownloadCalls -eq 2) 'archive and checksum downloads'
    Assert-True (Test-Path -LiteralPath $exe) 'executable installed'
    Assert-True (Test-Path -LiteralPath (Join-Path $destination 'nsh\bundle.nshc')) 'interface bundle next to the executable'
    Assert-True ($installerTestState.ShortcutSaved.TargetPath -eq $exe) 'shortcut targets the installed executable'
    Assert-True ($installerTestState.ShortcutSaved.WorkingDirectory -eq $destination) 'shortcut working directory'
    Assert-True (@(Get-ChildItem -LiteralPath (Split-Path $destination -Parent) -Force).Count -eq 1) 'no staging folders stay'
    Write-Host 'PASS latest release installation and shortcut'

    Set-Content -LiteralPath $exe -Value 'old executable'
    Set-Content -LiteralPath (Join-Path $destination 'nsh\old.nshc') -Value 'stale file'
    & $installer -Version v0.4.0 -InstallDir $destination
    Assert-True ($installerTestState.LatestCalls -eq 1) 'explicit version avoids latest lookup'
    Assert-True ((Get-Content $exe) -eq 'fixture executable v1') 'upgrade replaces executable'
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $destination 'nsh\old.nshc'))) 'upgrade removes stale files'
    Write-Host 'PASS explicit version upgrade'

    $env:NSVIM_VERSION = '0.4.0'
    $env:NSVIM_INSTALL_DIR = Join-Path $testRoot 'from environment'
    & $installer
    Assert-True (Test-Path -LiteralPath (Join-Path $env:NSVIM_INSTALL_DIR 'nsvim.exe')) 'NSVIM_INSTALL_DIR selects the folder'
    Assert-True ($installerTestState.LatestCalls -eq 1) 'NSVIM_VERSION avoids latest lookup'
    Remove-Item Env:\NSVIM_VERSION, Env:\NSVIM_INSTALL_DIR
    Write-Host 'PASS environment overrides'

    $before = Get-Content $exe
    Set-Content -LiteralPath (Join-Path $testRoot 'SHA256SUMS') -Value ((('0' * 64) + "  $asset")) -Encoding ascii
    Assert-Fails { & $installer -Version 0.4.0 -InstallDir $destination } 'Checksum verification failed'
    Assert-True ((Get-Content $exe) -eq $before) 'failed verification keeps the installed version'
    Write-Host 'PASS checksum rejection keeps the installation'

    Write-Release
    Add-Content -LiteralPath (Join-Path $testRoot 'SHA256SUMS') -Value (Get-Content (Join-Path $testRoot 'SHA256SUMS'))
    Assert-Fails { & $installer -Version 0.4.0 -InstallDir $destination } 'Missing or ambiguous checksum'
    Write-Host 'PASS duplicate checksum rejection'

    Assert-Fails { & $installer -Version 0.3.2 -InstallDir $destination } 'Release v0.3.2 has no'
    Write-Host 'PASS release without installer assets'

    $downloadsBefore = $installerTestState.DownloadCalls
    Assert-Fails { & $installer -Version '../bad' -InstallDir $destination } 'Version must be'
    $env:PROCESSOR_ARCHITECTURE = 'ARM64'
    Assert-Fails { & $installer -Version 0.4.0 -InstallDir $destination } 'x86_64 only'
    Assert-True ($installerTestState.DownloadCalls -eq $downloadsBefore) 'invalid input avoids downloads'
    Write-Host 'PASS invalid version and unsupported architecture'

    $env:PROCESSOR_ARCHITECTURE = 'AMD64'
    Remove-Item -LiteralPath (Join-Path $payload 'nsh') -Recurse
    Write-Release
    Assert-Fails { & $installer -Version 0.4.0 -InstallDir $destination } 'missing nsh\bundle.nshc'
    Assert-True ((Get-Content $exe) -eq $before) 'an incomplete release keeps the installed version'
    Write-Host 'PASS incomplete release rejection'
} finally {
    $env:PROCESSOR_ARCHITECTURE = $oldArchitecture
    $env:PROCESSOR_ARCHITEW6432 = $oldWowArchitecture
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}
