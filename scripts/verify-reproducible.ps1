$ErrorActionPreference = "Stop"
$firstDirectory = "artifacts/reproducibility/first"
$secondDirectory = "artifacts/reproducibility/second"

& "$PSScriptRoot/build-reproducible.ps1" -OutputDirectory $firstDirectory
& "$PSScriptRoot/build-reproducible.ps1" -OutputDirectory $secondDirectory

$first = Get-Content -LiteralPath (Join-Path $firstDirectory "checksums.sha256")
$second = Get-Content -LiteralPath (Join-Path $secondDirectory "checksums.sha256")
$difference = Compare-Object -ReferenceObject $first -DifferenceObject $second
if ($difference) {
    $difference | Format-Table | Out-String | Write-Error
    throw "Repeated WASM builds produced different checksums."
}

Write-Output "Repeated WASM builds are byte-for-byte reproducible."
$first
