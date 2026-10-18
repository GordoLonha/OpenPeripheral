<#
.SYNOPSIS
  Timed USB capture with USBPcap, without needing to kill the elevated USBPcapCMD.

.DESCRIPTION
  USBPcapCMD runs elevated, so a normal shell cannot stop it. Instead we own a
  named pipe, let USBPcapCMD write into it, copy it to a file for N seconds, then
  close the pipe: USBPcapCMD's next write fails and it exits by itself.

.EXAMPLE
  .\tools\capture.ps1 -Hub 4 -Device 7 -Seconds 6 -Out captures\k70_static_red.pcap
#>
param(
    [Parameter(Mandatory)][int]$Hub,
    [int[]]$Device,
    [int]$Seconds = 5,
    [Parameter(Mandatory)][string]$Out
)

$usbpcap = 'C:\Program Files\USBPcap\USBPcapCMD.exe'
$name = "op_cap_" + [guid]::NewGuid().ToString('N')
$pipe = New-Object System.IO.Pipes.NamedPipeServerStream($name, [System.IO.Pipes.PipeDirection]::In, 1,
    [System.IO.Pipes.PipeTransmissionMode]::Byte, [System.IO.Pipes.PipeOptions]::Asynchronous)

$args = @('-d', "\\.\USBPcap$Hub", '--inject-descriptors', '-o', "\\.\pipe\$name")
if ($Device) { $args += @('--devices', ($Device -join ',')) } else { $args += '-A' }
Start-Process $usbpcap -ArgumentList $args -WindowStyle Hidden | Out-Null

$conn = $pipe.BeginWaitForConnection($null, $null)
if (-not $conn.AsyncWaitHandle.WaitOne(120000)) { $pipe.Dispose(); throw "USBPcapCMD did not connect within 2 min (UAC prompt declined or not answered)" }
$pipe.EndWaitForConnection($conn)

$dir = Split-Path -Parent $Out
if ($dir) { New-Item -ItemType Directory -Force $dir | Out-Null }
$file = [System.IO.File]::Create((Join-Path (Get-Location) $Out))
$buf = New-Object byte[] 65536
$deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
Write-Host "capturing $Seconds s -> $Out"
while ([DateTime]::UtcNow -lt $deadline) {
    $read = $pipe.BeginRead($buf, 0, $buf.Length, $null, $null)
    $left = [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds
    if (-not $read.AsyncWaitHandle.WaitOne([Math]::Max($left, 1))) { break }
    $n = $pipe.EndRead($read)
    if ($n -le 0) { break }
    $file.Write($buf, 0, $n)
}
$file.Dispose()
$pipe.Dispose()   # USBPcapCMD exits on its next write
Write-Host ("done: {0:N0} bytes" -f (Get-Item $Out).Length)
