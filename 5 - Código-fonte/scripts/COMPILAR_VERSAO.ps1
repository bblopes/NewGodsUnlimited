# Compilação local de uma versão do New Gods Unlimited (reserva do GitHub Actions).
#
# Uso:  scripts\COMPILAR_VERSAO.bat 1.2.0
#
# O .exe vai para "2 - Versões de teste". Ao aprovar, ele é movido para
# "1 - Versão oficial".
param(
    [Parameter(Mandatory = $true)][string]$Versao
)
$ErrorActionPreference = 'Stop'

if ($Versao -notmatch '^\d+\.\d+\.\d+$') {
    throw "Versão inválida: $Versao. Use só números, como 1.2.0."
}

$codigo = Split-Path -Parent $PSScriptRoot
$raiz = Split-Path -Parent $codigo
$testes = Join-Path $raiz '2 - Versões de teste'
$vsDevCmd = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat'
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$nome = "New Gods Unlimited V$Versao.exe"

Push-Location $codigo
try {
    & cmd.exe /c "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && `"$cargo`" build --release --locked --bin new_gods_unlimited"
    if ($LASTEXITCODE -ne 0) { throw "Cargo terminou com código $LASTEXITCODE." }
    $gerado = Join-Path $codigo 'target\release\new_gods_unlimited.exe'

    New-Item -ItemType Directory -Force -Path $testes | Out-Null
    $destino = Join-Path $testes $nome

    Copy-Item -LiteralPath $gerado -Destination $destino -Force
    Write-Host "Pronto: $destino"
}
finally {
    Pop-Location
}
