<#
.SYNOPSIS
  Captura guiada do HS80 para engenharia reversa: grava o USB enquanto você mexe no iCUE.

.DESCRIPTION
  Roda como administrador (pede sozinho), grava TODAS as portas USB ao mesmo tempo
  com o USBPcap (não precisa achar o hub), diz o que fazer em cada etapa e no fim
  junta tudo em "hs80-captura.zip" na Área de Trabalho, pronto para enviar.

  As capturas podem conter números de série dos aparelhos USB do PC; nada além do
  tráfego USB é gravado (nem teclado digitado fora das etapas, nem rede).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File .\tools\capture_hs80.ps1
#>
$ErrorActionPreference = 'Stop'

# --- Administrador (o USBPcap exige) ---
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Start-Process powershell -Verb RunAs -ArgumentList @('-NoExit', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    exit
}

$cmd = 'C:\Program Files\USBPcap\USBPcapCMD.exe'
if (-not (Test-Path $cmd)) {
    Write-Host 'USBPcap não encontrado. Instale pelo Wireshark (tela "USB Capture") ou em https://desowin.org/usbpcap/ e reinicie o PC.' -ForegroundColor Red
    exit 1
}

# --- Portas USB (uma por controlador: USBPcap1, USBPcap2, ...) ---
$hubs = @(& $cmd --extcap-interfaces | ForEach-Object {
    if ($_ -match 'interface \{value=([^}]+)\}') { $Matches[1] }
})
if ($hubs.Count -eq 0) {
    Write-Host 'O USBPcap não listou nenhuma porta. Reinicie o PC depois de instalá-lo e rode de novo.' -ForegroundColor Red
    exit 1
}

$out = Join-Path ([Environment]::GetFolderPath('Desktop')) 'hs80-captura'
New-Item -ItemType Directory -Force $out | Out-Null
Get-ChildItem $out -Filter *.pcap -ErrorAction SilentlyContinue | Remove-Item

function Capture([string]$name, [int]$seconds) {
    $procs = foreach ($hub in $hubs) {
        $n = ($hub -replace '\D', '')
        $file = Join-Path $out "$name-hub$n.pcap"
        Start-Process $cmd -ArgumentList @('-d', $hub, '-A', '--inject-descriptors', '-o', "`"$file`"") -WindowStyle Hidden -PassThru
    }
    Start-Sleep -Milliseconds 1500
    [console]::Beep(880, 200)
    Write-Host '  >>> GRAVANDO: faça agora' -ForegroundColor Green
    for ($i = $seconds; $i -gt 0; $i--) { Write-Host -NoNewline "`r  faltam $i s   "; Start-Sleep 1 }
    Write-Host "`r  pare de mexer...      "
    Start-Sleep 3   # o iCUE continua conversando com o fone: empurra o que ficou no buffer
    $procs | ForEach-Object { if (-not $_.HasExited) { Stop-Process -Id $_.Id -Force } }
    [console]::Beep(440, 300)
    Write-Host '  ok' -ForegroundColor Green
}

$steps = @(
    @{ name = '1-parado'; s = 10; text = 'Deixe o iCUE aberto na tela do HS80 e NÃO mexa em nada (nem no fone).' },
    @{ name = '2-sidetone-liga-desliga'; s = 15; text = 'Ligue e desligue o SIDETONE duas vezes, com uns 2 segundos entre cada clique.' },
    @{ name = '3-sidetone-nivel'; s = 15; text = 'Com o sidetone ligado, arraste o NÍVEL devagar do mínimo ao máximo e volte ao mínimo.' },
    @{ name = '4-microfone-volume'; s = 15; text = 'Se o iCUE tiver VOLUME DO MICROFONE (ou Mic Boost), mexa nele do mínimo ao máximo. Se não tiver, só espere.' }
)

Write-Host ''
Write-Host 'Captura do HS80 para o OpenPeripheral' -ForegroundColor Cyan
Write-Host "Portas USB encontradas: $($hubs -join ', ')"
Write-Host 'Antes de começar: fone LIGADO, iCUE aberto na tela do HS80 (onde fica o sidetone).'
Write-Host ''
foreach ($st in $steps) {
    Write-Host "Etapa $($st.name): $($st.text)" -ForegroundColor Yellow
    Read-Host '  Pressione Enter quando estiver pronto' | Out-Null
    Capture $st.name $st.s
    Write-Host ''
}

$zip = Join-Path ([Environment]::GetFolderPath('Desktop')) 'hs80-captura.zip'
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path (Join-Path $out '*.pcap') -DestinationPath $zip
Write-Host "Pronto: $zip" -ForegroundColor Cyan
Write-Host 'Anexe esse arquivo na conversa com o Claude.'
Start-Process explorer.exe "/select,`"$zip`""
