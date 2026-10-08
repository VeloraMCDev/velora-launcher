# Run by the operator in Windows PowerShell. Never paste keys/passwords in chat.
[CmdletBinding()]
param([string]$Repository = 'VeloraMCDev/panel')
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'This setup uses Windows user-bound credential encryption.' }
if ($Repository -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') { throw 'Invalid repository name.' }
Get-Command keytool,gh -ErrorAction Stop | Out-Null
$taskDirectory = Join-Path $env:USERPROFILE '.velora\android'
$taskKey = Join-Path $taskDirectory 'velora-player.jks'
$taskCredential = Join-Path $taskDirectory 'signing-password.xml'
New-Item -ItemType Directory -Path $taskDirectory -Force | Out-Null
$taskAcl = New-Object System.Security.AccessControl.DirectorySecurity
$taskAcl.SetAccessRuleProtection($true, $false)
$taskSid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$taskRule = New-Object System.Security.AccessControl.FileSystemAccessRule($taskSid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
$taskAcl.AddAccessRule($taskRule)
Set-Acl -LiteralPath $taskDirectory -AclObject $taskAcl
if ((Test-Path -LiteralPath $taskKey) -ne (Test-Path -LiteralPath $taskCredential)) {
    throw 'Existing partial signing setup found. Nothing was overwritten. Recover the original key/password first.'
}
if (!(Test-Path -LiteralPath $taskKey)) {
    $taskRandom = New-Object byte[] 48
    $taskGenerator = [Security.Cryptography.RandomNumberGenerator]::Create()
    try { $taskGenerator.GetBytes($taskRandom) } finally { $taskGenerator.Dispose() }
    $taskPassword = [Convert]::ToBase64String($taskRandom)
    [Array]::Clear($taskRandom, 0, $taskRandom.Length)
    $env:VELORA_SETUP_KEY_PASSWORD = $taskPassword
    try {
        $taskPriorPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = 'Continue'
            & keytool -genkeypair -keystore $taskKey -storetype JKS -alias velora-player -keyalg RSA -keysize 3072 -validity 10000 -dname 'CN=Velora Player' -storepass:env VELORA_SETUP_KEY_PASSWORD -keypass:env VELORA_SETUP_KEY_PASSWORD -noprompt 2>&1 | Out-Null
            $taskKeyResult = $LASTEXITCODE
        } finally { $ErrorActionPreference = $taskPriorPreference }
        if ($taskKeyResult -ne 0) { throw 'Android key creation failed. No existing key was replaced.' }
        ConvertTo-SecureString $taskPassword -AsPlainText -Force | Export-Clixml -LiteralPath $taskCredential
    } finally {
        Remove-Item Env:\VELORA_SETUP_KEY_PASSWORD -ErrorAction SilentlyContinue
        $taskPassword = $null
    }
}
$taskSecurePassword = Import-Clixml -LiteralPath $taskCredential
$taskPassword = [System.Net.NetworkCredential]::new('', $taskSecurePassword).Password
function Set-TaskGitHubSecret([string]$Name, [string]$Value) {
    # Pass through stdin, never a command-line argument or console output.
    $Value | & gh secret set $Name --repo $Repository
    if ($LASTEXITCODE -ne 0) { throw "Could not configure $Name. The existing local key was retained; rerun this script." }
}
try {
    $taskBytes = [IO.File]::ReadAllBytes($taskKey)
    try { Set-TaskGitHubSecret 'VELORA_ANDROID_KEYSTORE_BASE64' ([Convert]::ToBase64String($taskBytes)) }
    finally { [Array]::Clear($taskBytes, 0, $taskBytes.Length) }
    Set-TaskGitHubSecret 'VELORA_ANDROID_KEYSTORE_PASSWORD' $taskPassword
    Set-TaskGitHubSecret 'VELORA_ANDROID_KEY_PASSWORD' $taskPassword
} finally { $taskPassword = $null }
Write-Host 'DONE. The persistent key stays outside the repository and GitHub stores its encrypted signing secrets.'
Write-Host 'Keep an encrypted backup of this directory. Windows password recovery does not recover DPAPI-encrypted credentials.'
Write-Host 'Use the mobile workflow with sign_android=true and the actual HTTPS player endpoint.'
