param(
  [string]$Title = 'ShenhePlayer',
  # Button position measured from the window's bottom-right corner
  [int]$FromRight = 270,
  [int]$FromBottom = 90,
  [string]$OutDir = 'D:\code\ShenhePlayer\.tmp'
)

# Drives a real click on the player's track-menu button, then moves the cursor
# up through the gap into the menu, capturing before/after. This is the only
# way to verify a hover/pointerleave bug.
#
# Only ever clicks inside our own player window, and aborts if that window is
# not the foreground window of our own process.
#
# ASCII-only: Windows PowerShell 5.1 reads BOM-less files as the ANSI codepage.

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Poke {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(IntPtr cls, string title);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004;
}
"@

[void][Poke]::SetProcessDPIAware()

$h = [Poke]::FindWindowW([IntPtr]::Zero, $Title)
if ($h -eq [IntPtr]::Zero) { Write-Output 'NOT_FOUND'; exit 1 }

$targetPid = 0
[void][Poke]::GetWindowThreadProcessId($h, [ref]$targetPid)
[void][Poke]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 600

$fg = [Poke]::GetForegroundWindow()
$fgPid = 0
[void][Poke]::GetWindowThreadProcessId($fg, [ref]$fgPid)
if ($fgPid -ne $targetPid) { Write-Output 'NOT_FOREGROUND - aborted'; exit 2 }

$r = New-Object 'Poke+RECT'
[void][Poke]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L
$hgt = $r.B - $r.T

function Save-Shot($name) {
  $bmp = New-Object System.Drawing.Bitmap $w, $hgt
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size($w, $hgt)))
  $bmp.Save((Join-Path $OutDir $name), [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
}

$bx = $r.R - $FromRight
$by = $r.B - $FromBottom

# Wake the auto-hiding chrome first
[void][Poke]::SetCursorPos(($r.L + [int]($w / 2)), ($r.T + [int]($hgt / 2)))
Start-Sleep -Milliseconds 300

# Click the menu button
[void][Poke]::SetCursorPos($bx, $by)
Start-Sleep -Milliseconds 250
[Poke]::mouse_event([Poke]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 60
[Poke]::mouse_event([Poke]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 450
Save-Shot 'menu-open.png'

# Now walk the cursor upward through the gap into the menu body
foreach ($dy in 20, 40, 60, 85) {
  [void][Poke]::SetCursorPos($bx, ($by - $dy))
  Start-Sleep -Milliseconds 120
}
Start-Sleep -Milliseconds 400
Save-Shot 'menu-hover.png'

Write-Output ('OK clicked at ' + $bx + ',' + $by + ' in ' + $w + 'x' + $hgt)
