# Compilação local de uma versão do New Gods Unlimited (reserva do GitHub Actions).
#
# Uso:  scripts\COMPILAR_VERSAO.bat 1.2.0-teste.1
#
# Versão sem "-teste" vai para "1 - Versão oficial" (a oficial anterior é
# movida para "2 - Versões de teste\_histórico"). Versão com "-teste" vai para
# "2 - Versões de teste".
param(
    [Parameter(Mandatory = $true)][string]$Versao
)
$ErrorActionPreference = 'Stop'

if ($Versao -notmatch '^\d+\.\d+\.\d+(-teste\.\d+)?$') {
    throw "Versão inválida: $Versao. Use 1.2.0 ou 1.2.0-teste.1."
}

$codigo = Split-Path -Parent $PSScriptRoot
$raiz = Split-Path -Parent $codigo
$oficial = Join-Path $raiz '1 - Versão oficial'
$testes = Join-Path $raiz '2 - Versões de teste'
$historico = Join-Path $testes '_histórico'
$vsDevCmd = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat'
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$nome = "New Gods Unlimited V$Versao.exe"
$ehTeste = $Versao -match '-teste'

Push-Location $codigo
try {
    & cmd.exe /c "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && `"$cargo`" build --release --locked --bin new_gods_unlimited"
    if ($LASTEXITCODE -ne 0) { throw "Cargo terminou com código $LASTEXITCODE." }
    $gerado = Join-Path $codigo 'target\release\new_gods_unlimited.exe'

    if ($ehTeste) {
        New-Item -ItemType Directory -Force -Path $testes | Out-Null
        $destino = Join-Path $testes $nome
    }
    else {
        New-Item -ItemType Directory -Force -Path $oficial, $historico | Out-Null
        Get-ChildItem -LiteralPath $oficial -Filter 'New Gods Unlimited V*.exe' | ForEach-Object {
            $pasta = Join-Path $historico ((Get-Date -Format 'yyyy-MM-dd') + ' ' + $_.BaseName)
            New-Item -ItemType Directory -Force -Path $pasta | Out-Null
            Move-Item -LiteralPath $_.FullName -Destination $pasta -Force
        }
        $destino = Join-Path $oficial $nome
    }

    Copy-Item -LiteralPath $gerado -Destination $destino -Force
    Write-Host "Pronto: $destino"
}
finally {
    Pop-Location
}
