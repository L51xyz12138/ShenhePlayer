param(
  [string]$Title = 'ShenhePlayer',
  [string]$Out = 'D:\code\ShenhePlayer\.tmp\live.png',
  # Nudge the cursor inside the window first, to wake auto-hiding player chrome
  [switch]$Wake
)

# Real screen sample of one of OUR windows, used to verify GPU-rendered video
# (PrintWindow returns black for D3D content, so it cannot answer that).
#
# Safety: we raise the target window first and then VERIFY it is actually the
# foreground window. If the raise fails we abort without capturing anything, so
# we never grab whatever else happens to be on screen.
#
# ASCII-only: Windows PowerShell 5.1 reads BOM-less files as the ANSI codepage.

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Live {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(IntPtr cls, string title);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

[void][Live]::SetProcessDPIAware()

$h = [Live]::FindWindowW([IntPtr]::Zero, $Title)
if ($h -eq [IntPtr]::Zero) { Write-Output 'NOT_FOUND'; exit 1 }

$targetPid = 0
[void][Live]::GetWindowThreadProcessId($h, [ref]$targetPid)

function Test-OwnedForeground {
  $fg = [Live]::GetForegroundWindow()
  if ($fg -eq [IntPtr]::Zero) { return $false }
  $fgPid = 0
  [void][Live]::GetWindowThreadProcessId($fg, [ref]$fgPid)
  # Accept any window of the same process: the controls live in an owned
  # overlay window, so it - not the player - may hold focus.
  return ($fgPid -eq $targetPid)
}

if (-not (Test-OwnedForeground)) {
  [void][Live]::ShowWindow($h, 9)   # SW_RESTORE
  [void][Live]::SetForegroundWindow($h)
  Start-Sleep -Milliseconds 900
}

if (-not (Test-OwnedForeground)) {
  Write-Output 'NOT_FOREGROUND - aborted without capturing'
  exit 2
}

$r = New-Object 'Live+RECT'
[void][Live]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L
$hgt = $r.B - $r.T
if ($w -le 0 -or $hgt -le 0) { Write-Output 'EMPTY_RECT'; exit 1 }

if ($Wake) {
  $cx = $r.L + [int]($w / 2)
  $cy = $r.T + [int]($hgt / 2)
  [void][Live]::SetCursorPos($cx, $cy)
  Start-Sleep -Milliseconds 120
  [void][Live]::SetCursorPos($cx + 12, $cy + 8)
  Start-Sleep -Milliseconds 450
}

$dir = Split-Path -Parent $Out
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }

$bmp = New-Object System.Drawing.Bitmap $w, $hgt
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size($w, $hgt)))
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()

Write-Output ('OK ' + $w + 'x' + $hgt + ' -> ' + $Out)
