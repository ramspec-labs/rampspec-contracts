$ErrorActionPreference = "Stop"
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("rampspec-specs-" + [guid]::NewGuid())
$wasmDirectory = Join-Path $temporaryRoot "wasm"
$specDirectory = Join-Path $temporaryRoot "specs"

try {
    .\scripts\build-reproducible.ps1 -OutputDirectory $wasmDirectory
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    node scripts\generate-contract-specs.mjs $wasmDirectory $specDirectory
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $expected = Get-ChildItem artifacts\specs -File | Sort-Object Name
    $actual = Get-ChildItem $specDirectory -File | Sort-Object Name
    if (($expected.Name -join "`n") -ne ($actual.Name -join "`n")) {
        throw "generated spec file inventory differs"
    }
    foreach ($file in $expected) {
        $other = Join-Path $specDirectory $file.Name
        if ((Get-FileHash $file.FullName -Algorithm SHA256).Hash -ne (Get-FileHash $other -Algorithm SHA256).Hash) {
            throw "generated spec drift: $($file.Name)"
        }
    }
    Write-Output "generated contract specs are current"
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
