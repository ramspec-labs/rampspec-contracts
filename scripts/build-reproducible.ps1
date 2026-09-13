param(
    [string]$OutputDirectory = "artifacts/wasm"
)

$ErrorActionPreference = "Stop"
$env:CARGO_INCREMENTAL = "0"
$env:SOURCE_DATE_EPOCH = "0"
$env:TZ = "UTC"

if (Test-Path -LiteralPath $OutputDirectory) {
    Remove-Item -LiteralPath $OutputDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null

stellar contract build --locked --out-dir $OutputDirectory
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$checksumLines = Get-ChildItem -LiteralPath $OutputDirectory -Filter "*.wasm" |
    Sort-Object -Property Name |
    ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $($_.Name)"
    }
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$checksumPath = Join-Path $OutputDirectory "checksums.sha256"
[System.IO.File]::WriteAllLines((Resolve-Path -LiteralPath $OutputDirectory).Path + "\checksums.sha256", $checksumLines, $utf8NoBom)

$sourceRevision = (git rev-parse HEAD 2>$null)
if ($LASTEXITCODE -ne 0) { $sourceRevision = "uncommitted" }
$provenance = [ordered]@{
    container = "rust:1.93.1-bookworm@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31"
    rust = (rustc --version)
    source_revision = $sourceRevision
    stellar_cli = ((stellar --version | Select-Object -First 1) -replace "`r", "")
    target = "wasm32v1-none"
}
$provenanceJson = $provenance | ConvertTo-Json
$provenancePath = (Resolve-Path -LiteralPath $OutputDirectory).Path + "\provenance.json"
[System.IO.File]::WriteAllText($provenancePath, $provenanceJson + "`n", $utf8NoBom)
