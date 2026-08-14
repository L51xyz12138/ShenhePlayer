param(
  [string]$Title = 'ShenhePlayer',
  # Menu button position, measured from the window's bottom-right corner
  [int]$FromRight = 331,
  [int]$FromBottom = 90,
  [int]$Notches = 5,
  [string]$OutDir = 'D:\code\ShenhePlayer\.tmp'
)

# Opens a player popup menu, then scrolls the wheel while the cursor is over
# the menu body. Used to verify the wheel scrolls the list instead of being
# swallowed by the volume control.
#
# Only touches our own player window, and aborts unless the foreground window
# belongs to the same process.
#
# ASCII-only: Windows PowerShell 5.1 reads BOM-less files as the ANSI codepage.

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Whl {
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

[void][Whl]::SetProcessDPIAware()

$h = [Whl]::FindWindowW([IntPtr]::Zero, $Title)
if ($h -eq [IntPtr]::Zero) { Write-Output 'NOT_FOUND'; exit 1 }

$targetPid = 0
[void][Whl]::GetWindowThreadProcessId($h, [ref]$targetPid)
[void][Whl]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 600

$fg = [Whl]::GetForegroundWindow()
$fgPid = 0
[void][Whl]::GetWindowThreadProcessId($fg, [ref]$fgPid)
if ($fgPid -ne $targetPid) { Write-Output 'NOT_FOREGROUND - aborted'; exit 2 }

$r = New-Object 'Whl+RECT'
[void][Whl]::GetWindowRect($h, [ref]$r)
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

# Wake the auto-hiding chrome, then open the menu
[void][Whl]::SetCursorPos(($r.L + [int]($w / 2)), ($r.T + [int]($hgt / 2)))
Start-Sleep -Milliseconds 300
[void][Whl]::SetCursorPos($bx, $by)
Start-Sleep -Milliseconds 250
[Whl]::mouse_event([Whl]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 60
[Whl]::mouse_event([Whl]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 500

# Park the cursor inside the menu body, then spin the wheel
[void][Whl]::SetCursorPos($bx, ($by - 80))
Start-Sleep -Milliseconds 300
for ($i = 0; $i -lt $Notches; $i++) {
  [Whl]::mouse_event([Whl]::WHEEL, 0, 0, 4294967176, [IntPtr]::Zero)  # -120, scroll down
  Start-Sleep -Milliseconds 90
}
Start-Sleep -Milliseconds 500
Save-Shot 'wheel-in-menu.png'

Write-Output ('OK menu at ' + $bx + ',' + $by + ' window ' + $w + 'x' + $hgt)
