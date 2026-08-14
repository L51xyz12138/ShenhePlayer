param(
  [string]$Title = 'ShenhePlayer',
  # Click point, in client coordinates of the target window
  [int]$X = 0,
  [int]$Y = 0,
  [int]$ScrollDown = 0,
  [string]$Out = ''
)

# Click a point inside one of OUR windows, optionally scroll, optionally capture.
# Aborts unless the foreground window belongs to the same process, so it can
# never click or capture anything outside this app.
#
# ASCII-only: Windows PowerShell 5.1 reads BOM-less files as the ANSI codepage.

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Clk {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(IntPtr cls, string title);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004, WHEEL = 0x0800;
}
"@

[void][Clk]::SetProcessDPIAware()

$h = [Clk]::FindWindowW([IntPtr]::Zero, $Title)
if ($h -eq [IntPtr]::Zero) { Write-Output 'NOT_FOUND'; exit 1 }

$targetPid = 0
[void][Clk]::GetWindowThreadProcessId($h, [ref]$targetPid)
[void][Clk]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 500

$fg = [Clk]::GetForegroundWindow()
$fgPid = 0
[void][Clk]::GetWindowThreadProcessId($fg, [ref]$fgPid)
if ($fgPid -ne $targetPid) { Write-Output 'NOT_FOREGROUND - aborted'; exit 2 }

$r = New-Object 'Clk+RECT'
[void][Clk]::GetWindowRect($h, [ref]$r)

if ($X -gt 0 -or $Y -gt 0) {
  [void][Clk]::SetCursorPos(($r.L + $X), ($r.T + $Y))
  Start-Sleep -Milliseconds 200
  [Clk]::mouse_event([Clk]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [Clk]::mouse_event([Clk]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Milliseconds 500
}

if ($ScrollDown -ne 0) {
  for ($i = 0; $i -lt $ScrollDown; $i++) {
    # WHEEL_DELTA = 120; scrolling down is a negative delta, and dwData is a
    # DWORD, so -120 must be passed as its unsigned 32-bit form.
    [Clk]::mouse_event([Clk]::WHEEL, 0, 0, 4294967176, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 80
  }
  Start-Sleep -Milliseconds 400
}

if ($Out -ne '') {
  [void][Clk]::GetWindowRect($h, [ref]$r)
  $w = $r.R - $r.L
  $hgt = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $hgt
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size($w, $hgt)))
  $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
}

Write-Output ('OK window ' + ($r.R - $r.L) + 'x' + ($r.B - $r.T))
