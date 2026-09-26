# Copyright (c) 2026 unicbm. Licensed under MIT; see ../LICENSE.
[CmdletBinding()]
param(
    [switch]$FixtureTests,
    [string]$DemoPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
function Invoke-Cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) { throw "cargo $($Arguments -join ' ') failed ($LASTEXITCODE)" }
}

if ($FixtureTests -and ([string]::IsNullOrWhiteSpace($DemoPath) -or
    -not (Test-Path -LiteralPath $DemoPath -PathType Leaf))) {
    throw '-FixtureTests requires -DemoPath pointing to the original upstream test_demo.dem; no demo is bundled.'
}
$previousDemo = $env:DEMOPARSER_TEST_DEMO
if ($FixtureTests) { $env:DEMOPARSER_TEST_DEMO = (Resolve-Path -LiteralPath $DemoPath).Path }
Push-Location $repoRoot
try {
    # Format entrypoints/build scripts without rewriting preserved generated and
    # upstream modules. The modified fixture harness is checked separately.
    Invoke-Cargo fmt --all '--' --check --config skip_children=true
    $formatFiles = @(
        'src/parser/src/e2e_test.rs'
    )
    & rustfmt --edition 2021 --check --config-path src/parser/rustfmt.toml @formatFiles
    if ($LASTEXITCODE -ne 0) { throw 'Maintained parser source formatting check failed.' }
    Invoke-Cargo check --workspace --all-targets --release --locked
    Invoke-Cargo test --workspace --release --locked
    # Compile the preserved fixture assertions even when their data is absent.
    Invoke-Cargo test --package parser --lib --release --locked --features external-demo-tests --no-run
    if ($FixtureTests) {
        Invoke-Cargo test --package parser --lib --release --locked --features external-demo-tests e2e_test:: '--' --ignored
    }
    else {
        Write-Host 'External demo regressions compiled, not executed: use -FixtureTests -DemoPath <original test_demo.dem>.'
    }
}
finally {
    Pop-Location
    $env:DEMOPARSER_TEST_DEMO = $previousDemo
}
