param(
  [Parameter(Mandatory = $true)][string]$LinuxManifestPath,
  [Parameter(Mandatory = $true)][string]$WindowsManifestPath,
  [string]$Toolchain = "pinned-rust-wasm32-unknown-unknown"
)

$ErrorActionPreference = "Stop"

$linux = Get-Content -Raw -Path $LinuxManifestPath | ConvertFrom-Json
$windows = Get-Content -Raw -Path $WindowsManifestPath | ConvertFrom-Json

$mismatches = @()

foreach ($contractName in $linux.contracts.PSObject.Properties.Name) {
  $linuxHash = $linux.contracts.$contractName
  $winHash = $windows.contracts.$contractName

  if ($linuxHash -ne $winHash) {
    $mismatches += [PSCustomObject]@{
      toolchain = $Toolchain
      contract  = $contractName
      linuxSha256 = $linuxHash
      windowsSha256 = $winHash
    }
  }
}

if ($mismatches.Count -gt 0) {
  Write-Error ("WASM checksum mismatch detected under toolchain '{0}': {1}" -f $Toolchain, ($mismatches | ConvertTo-Json -Compress))
  exit 1
}

Write-Output ("All contract WASM checksums match across Linux and Windows ({0})." -f $Toolchain)
