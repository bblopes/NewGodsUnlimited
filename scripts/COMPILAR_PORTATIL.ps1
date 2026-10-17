$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$vsDevCmd = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat'
$cargo = 'C:\Users\T-GAMER\.cargo\bin\cargo.exe'
$targetDirectory = Join-Path $projectRoot 'target_publicacao'
$deliveryDirectory = Join-Path $projectRoot 'releases\windows\current'
$source = Join-Path $targetDirectory 'release\ecossistema_neural.exe'
$delivery = Join-Path $deliveryDirectory 'Ecossistema-Neural.exe'
$archive = Join-Path $projectRoot 'releases\windows\Ecossistema-Neural-Portatil-Windows-x64.zip'

Push-Location $projectRoot
try {
    & cmd.exe /c "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && `"$cargo`" build --release --bin ecossistema_neural --target-dir target_publicacao"
    if ($LASTEXITCODE -ne 0) { throw "Cargo terminou com codigo $LASTEXITCODE." }

    New-Item -ItemType Directory -Force -Path $deliveryDirectory | Out-Null
    # Se a pessoa estiver rodando a entrega, o Windows bloqueia a troca do
    # arquivo. Ainda assim publicamos uma cópia imediatamente utilizável.
    $publishedDelivery = $delivery
    try {
        Copy-Item -LiteralPath $source -Destination $delivery -Force
    }
    catch {
        $publishedDelivery = Join-Path $deliveryDirectory 'Ecossistema-Neural-atualizado.exe'
        Copy-Item -LiteralPath $source -Destination $publishedDelivery -Force
        Write-Warning "A entrega principal está em uso; publiquei Ecossistema-Neural-atualizado.exe."
    }
    Compress-Archive -LiteralPath $publishedDelivery -DestinationPath $archive -Force
    Write-Host "Pronto: $publishedDelivery"
    Write-Host "Pronto: $archive"
}
finally {
    Pop-Location
}
