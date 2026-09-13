$ErrorActionPreference = "Stop"
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("rampspec-specs-" + [guid]::NewGuid())
$wasmDirectory = Join-Path $temporaryRoot "wasm"
$specDirectory = Join-Path $temporaryRoot "specs"

try {
    .\scripts\build-reproducible.ps1 -OutputDirectory $wasmDirectory
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    node scripts\generate-contract-specs.mjs $wasmDirectory $specDirectory
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $bindingDirectory = Join-Path $temporaryRoot "bindings"
    node scripts\generate-typescript-bindings.mjs $wasmDirectory $bindingDirectory
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $expected = Get-ChildItem artifacts\specs -File | Sort-Object Name
    $actual = Get-ChildItem $specDirectory -File | Sort-Object Name
    if (($expected.Name -join "`n") -ne ($actual.Name -join "`n")) {
        throw "generated spec file inventory differs"
    }
    foreach ($file in $expected) {
        $other = Join-Path $specDirectory $file.Name
        if ($file.Name -eq "manifest.json") {
            $expectedManifest = Get-Content -LiteralPath $file.FullName -Raw | ConvertFrom-Json
            $actualManifest = Get-Content -LiteralPath $other -Raw | ConvertFrom-Json
            foreach ($contract in $actualManifest.contracts.PSObject.Properties) {
                if ($contract.Value.wasmSha256 -notmatch '^[0-9a-f]{64}$') {
                    throw "generated WASM hash is invalid: $($contract.Name)"
                }
                $expectedContract = $expectedManifest.contracts.PSObject.Properties[$contract.Name].Value
                if ($expectedContract.wasmSha256 -notmatch '^[0-9a-f]{64}$') {
                    throw "canonical WASM hash is invalid: $($contract.Name)"
                }
                $contract.Value.wasmSha256 = $expectedContract.wasmSha256
            }
            $expectedJson = $expectedManifest | ConvertTo-Json -Depth 20 -Compress
            $actualJson = $actualManifest | ConvertTo-Json -Depth 20 -Compress
            if ($expectedJson -ne $actualJson) {
                throw "generated semantic spec drift: manifest.json"
            }
        }
        elseif ((Get-FileHash $file.FullName -Algorithm SHA256).Hash -ne (Get-FileHash $other -Algorithm SHA256).Hash) {
            throw "generated spec drift: $($file.Name)"
        }
    }
    $expectedBindings = Get-ChildItem artifacts\bindings -Recurse -File |
        Where-Object { $_.FullName -notmatch '\\(node_modules|dist)\\' }
    foreach ($file in $expectedBindings) {
        $relative = $file.FullName.Substring((Resolve-Path artifacts\bindings).Path.Length + 1)
        $other = Join-Path $bindingDirectory $relative
        if (!(Test-Path -LiteralPath $other)) {
            throw "generated binding file missing: $relative"
        }
        if ((Get-FileHash $file.FullName -Algorithm SHA256).Hash -ne (Get-FileHash $other -Algorithm SHA256).Hash) {
            throw "generated binding drift: $relative"
        }
    }
    Write-Output "generated contract specs are current"
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
