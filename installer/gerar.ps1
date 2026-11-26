# Gera o instalador do ZoneSnap e os arquivos da release.
#
#   installer\gerar.ps1                 compila, faz o setup, o zip portátil e os hashes
#   installer\gerar.ps1 -PularBuild     usa o exe que já está em target\release
#   installer\gerar.ps1 -SemPortatil    só o setup
#
# Sai tudo em dist\. O nome do setup e do zip levam a versão do Cargo.toml, então
# subir a versão é o único lugar onde ela se muda.

param(
    [switch]$PularBuild,
    [switch]$SemPortatil
)

$ErrorActionPreference = "Stop"
$raiz = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $raiz "dist"

function Achar-Iscc {
    $candidatos = @(
        (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"),
        (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe"),
        (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe")
    )
    foreach ($c in $candidatos) {
        if ($c -and (Test-Path $c)) { return $c }
    }
    return $null
}

$versao = (Select-String -Path (Join-Path $raiz "Cargo.toml") -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
Write-Host "versao: $versao"

if (-not $PularBuild) {
    $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    Write-Host "compilando..."
    & $cargo build --release --manifest-path (Join-Path $raiz "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "cargo build falhou" }
}

$exe = Join-Path $raiz "target\release\zonesnap.exe"
if (-not (Test-Path $exe)) { throw "nao achei $exe; rode o build" }

$iscc = Achar-Iscc
if (-not $iscc) { throw "Inno Setup 6 nao encontrado; instale de https://jrsoftware.org/isdl.php" }

New-Item -ItemType Directory -Force -Path $dist | Out-Null

Write-Host "compilando o instalador com $iscc"
& $iscc "/DMinhaVersao=$versao" (Join-Path $PSScriptRoot "zonesnap.iss")
if ($LASTEXITCODE -ne 0) { throw "ISCC falhou" }

$setup = Join-Path $dist "zonesnap-$versao-setup.exe"
if (-not (Test-Path $setup)) { throw "o ISCC nao gerou $setup" }

if (-not $SemPortatil) {
    # Zip portátil: quem não quer instalador pega o exe e roda de qualquer pasta.
    $temp = Join-Path $dist "portatil"
    if (Test-Path $temp) { Remove-Item $temp -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $temp | Out-Null
    Copy-Item $exe (Join-Path $temp "zonesnap.exe")
    Copy-Item (Join-Path $raiz "README.md") $temp
    Copy-Item (Join-Path $raiz "LICENSE") $temp
    $zip = Join-Path $dist "zonesnap-$versao-portatil.zip"
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Compress-Archive -Path (Join-Path $temp "*") -DestinationPath $zip -CompressionLevel Optimal
    Remove-Item $temp -Recurse -Force
}

$hashes = Join-Path $dist "SHA256SUMS.txt"
$linhas = Get-ChildItem $dist -File | Where-Object { $_.Extension -in @(".exe", ".zip") } | Sort-Object Name | ForEach-Object {
    $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
    "$h  $($_.Name)"
}
Set-Content -Path $hashes -Value $linhas -Encoding ascii

Write-Host ""
Get-ChildItem $dist -File | Sort-Object Name | ForEach-Object {
    "{0,-34} {1,10} KB" -f $_.Name, [math]::Round($_.Length / 1KB, 0)
}
