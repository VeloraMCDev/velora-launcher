# Installs job-local PowerShell and Rust dependencies for the Windows launcher.
param([ValidateSet('Launcher')][string]$Mode = 'Launcher')
$ErrorActionPreference = 'Stop'
if (-not [Environment]::OSVersion.Platform.ToString().StartsWith('Win')) {
    throw 'This setup tool requires Windows.'
}
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
if (-not $env:RUNNER_TEMP) { $env:RUNNER_TEMP = $env:TEMP }

function Add-RunnerPath([string]$Directory) {
    $env:PATH = "$Directory;$env:PATH"
    foreach ($pathFile in @($env:GITHUB_PATH, $env:GITEA_PATH)) {
        if ($pathFile) { Add-Content -Path $pathFile -Value $Directory }
    }
}

$pwsh = Get-Command pwsh.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source
if (-not $pwsh) {
    $machinePwsh = Join-Path $env:ProgramFiles 'PowerShell\7\pwsh.exe'
    if (Test-Path $machinePwsh) { $pwsh = $machinePwsh }
}
if (-not $pwsh) {
    $version = '7.6.6'
    $installDir = Join-Path $env:RUNNER_TEMP "scopenet-pwsh-$version"
    $pwsh = Join-Path $installDir 'pwsh.exe'
    if (-not (Test-Path $pwsh)) {
        New-Item -ItemType Directory -Force -Path $installDir | Out-Null
        $archive = Join-Path $env:RUNNER_TEMP "PowerShell-$version-win-x64.zip"
        Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/PowerShell/PowerShell/releases/download/v$version/PowerShell-$version-win-x64.zip" -OutFile $archive
        Expand-Archive -Path $archive -DestinationPath $installDir -Force
        Remove-Item $archive
    }
}
Add-RunnerPath (Split-Path $pwsh)
Write-Host "PowerShell $(& $pwsh -NoProfile -Command '$PSVersionTable.PSVersion.ToString()') is ready."

$node = Get-Command node.exe -ErrorAction SilentlyContinue
$nodeMajor = if ($node) { [int]((& node --version).TrimStart('v').Split('.')[0]) } else { 0 }
if ($nodeMajor -lt 22) {
    $nodeVersion = '22.23.3'
    $nodeArchive = Join-Path $env:RUNNER_TEMP "node-v$nodeVersion-win-x64.zip"
    $nodeRoot = Join-Path $env:RUNNER_TEMP 'scopenet-node'
    $nodeBin = Join-Path $nodeRoot "node-v$nodeVersion-win-x64"
    if (-not (Test-Path (Join-Path $nodeBin 'node.exe'))) {
        Invoke-WebRequest -UseBasicParsing -Uri "https://nodejs.org/dist/v$nodeVersion/node-v$nodeVersion-win-x64.zip" -OutFile $nodeArchive
        Expand-Archive -Path $nodeArchive -DestinationPath $nodeRoot -Force
        Remove-Item $nodeArchive
    }
    Add-RunnerPath $nodeBin
}
Write-Host "Node $(& node --version) is ready."

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
Add-RunnerPath $cargoBin
if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) {
    $installer = Join-Path $env:RUNNER_TEMP 'scopenet-rustup-init.exe'
    Invoke-WebRequest -UseBasicParsing -Uri 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' -OutFile $installer
    & $installer -y --profile minimal --default-toolchain stable
    if ($LASTEXITCODE -ne 0) { throw "rustup installer failed with exit code $LASTEXITCODE" }
    Remove-Item $installer
}
& rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal --component clippy --component rustfmt
if ($LASTEXITCODE -ne 0) { throw "Rust toolchain install failed with exit code $LASTEXITCODE" }
& rustup default stable-x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "Rust toolchain selection failed with exit code $LASTEXITCODE" }

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsPath = if (Test-Path $vswhere) { & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath }
if (-not $vsPath) {
    $installer = Join-Path $env:RUNNER_TEMP 'scopenet-vs-buildtools.exe'
    Invoke-WebRequest -UseBasicParsing -Uri 'https://aka.ms/vs/17/release/vs_buildtools.exe' -OutFile $installer
    $arguments = @('--quiet', '--wait', '--norestart', '--add', 'Microsoft.VisualStudio.Workload.VCTools', '--includeRecommended')
    $process = Start-Process -FilePath $installer -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -eq 3010) { throw 'Microsoft C++ Build Tools installed, but Windows must be restarted before this runner can build.' }
    if ($process.ExitCode -ne 0) { throw "Microsoft C++ Build Tools installation failed with exit code $($process.ExitCode). Run the runner with administrator rights." }
    Remove-Item $installer
    $vsPath = if (Test-Path $vswhere) { & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath }
    if (-not $vsPath) { throw 'Microsoft C++ Build Tools installation did not provide the x64 MSVC compiler.' }
}
Write-Host "Microsoft C++ Build Tools are ready at $vsPath."
foreach ($tool in @('cargo.exe', 'rustc.exe', 'node.exe', 'npm.cmd')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Missing $tool after dependency setup." }
}
if ([int]((& node --version).TrimStart('v').Split('.')[0]) -lt 22) { throw 'Node 22+ is required.' }
Write-Host "Windows $Mode dependencies are ready."
