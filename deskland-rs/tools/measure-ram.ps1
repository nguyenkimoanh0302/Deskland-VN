# Đo RAM của Deskland (bản C# + WebView2 hoặc bản Rust native) ở trạng thái ổn định.
# Cộng Private Memory + Working Set của mọi tiến trình liên quan, kèm bộ nhớ GPU theo tiến trình
# (Shared Usage quan trọng với GPU tích hợp vì lấy từ RAM hệ thống).
#
#   powershell -File tools\measure-ram.ps1 -Target rust   -Samples 6 -IntervalSec 10
#   powershell -File tools\measure-ram.ps1 -Target csharp -Samples 6 -IntervalSec 10
param(
    [ValidateSet('rust', 'csharp')][string]$Target = 'rust',
    [int]$Samples = 6,
    [int]$IntervalSec = 10
)

function Get-TargetProcesses {
    if ($Target -eq 'rust') {
        return @(Get-Process deskland-rs -ErrorAction SilentlyContinue)
    }
    $host_ = @(Get-Process Deskland -ErrorAction SilentlyContinue)
    # Các tiến trình msedgewebview2 dùng thư mục hồ sơ của Deskland
    $web = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
        Where-Object { $_.CommandLine -like '*DesklandPreview*' } |
        ForEach-Object { Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue }
    return @($host_) + @($web)
}

function Get-GpuMemMB([int[]]$ids) {
    $ded = 0.0; $sh = 0.0
    try {
        $c = Get-Counter '\GPU Process Memory(*)\Dedicated Usage', '\GPU Process Memory(*)\Shared Usage' -ErrorAction Stop
        foreach ($s in $c.CounterSamples) {
            if ($s.InstanceName -match '^pid_(\d+)_') {
                if ($ids -contains [int]$Matches[1]) {
                    if ($s.Path -like '*dedicated usage') { $ded += $s.CookedValue } else { $sh += $s.CookedValue }
                }
            }
        }
    } catch { }
    return @([math]::Round($ded / 1MB, 1), [math]::Round($sh / 1MB, 1))
}

$rows = @()
for ($i = 1; $i -le $Samples; $i++) {
    $ps = Get-TargetProcesses
    if ($ps.Count -eq 0) { Write-Host "Không tìm thấy tiến trình '$Target'"; exit 1 }
    $priv = ($ps | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB
    $ws = ($ps | Measure-Object WorkingSet64 -Sum).Sum / 1MB
    $cpu = ($ps | Measure-Object CPU -Sum).Sum
    $gpu = Get-GpuMemMB ($ps | ForEach-Object Id)
    $rows += [pscustomobject]@{
        Mau = $i; TienTrinh = $ps.Count
        PrivateMB = [math]::Round($priv, 1); WorkingSetMB = [math]::Round($ws, 1)
        GpuDedicatedMB = $gpu[0]; GpuSharedMB = $gpu[1]; CpuGiay = [math]::Round($cpu, 2)
    }
    if ($i -lt $Samples) { Start-Sleep -Seconds $IntervalSec }
}
$rows | Format-Table -AutoSize
$cpuRate = if ($Samples -gt 1) { ($rows[-1].CpuGiay - $rows[0].CpuGiay) / (($Samples - 1) * $IntervalSec) * 100 } else { 0 }
[pscustomobject]@{
    Target = $Target
    TienTrinh = $rows[-1].TienTrinh
    PrivateMB_TB = [math]::Round(($rows | Measure-Object PrivateMB -Average).Average, 1)
    WorkingSetMB_TB = [math]::Round(($rows | Measure-Object WorkingSetMB -Average).Average, 1)
    GpuSharedMB_TB = [math]::Round(($rows | Measure-Object GpuSharedMB -Average).Average, 1)
    GpuDedicatedMB_TB = [math]::Round(($rows | Measure-Object GpuDedicatedMB -Average).Average, 1)
    CPU_PhanTram1Nhan = [math]::Round($cpuRate, 1)
} | Format-List
# Chi tiết từng tiến trình ở mẫu cuối
Get-TargetProcesses | Select-Object Id, ProcessName, @{n = 'PrivateMB'; e = { [math]::Round($_.PrivateMemorySize64 / 1MB, 1) } },
    @{n = 'WorkingSetMB'; e = { [math]::Round($_.WorkingSet64 / 1MB, 1) } } | Sort-Object PrivateMB -Descending | Format-Table -AutoSize
