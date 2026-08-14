param(
  [string]$Title = 'ShenhePlayer Overlay',
  [string]$Out = 'D:\code\ShenhePlayer\.tmp\overlay.png'
)

# Capture a specific top-level window by exact title, via PrintWindow.
# ASCII-only on purpose: Windows PowerShell 5.1 reads BOM-less files as ANSI.

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class WinAny {
  // class must be a real NULL, so take it as IntPtr - PowerShell marshals
  // a $null string as an empty string, which matches nothing.
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(IntPtr cls, string title);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

[void][WinAny]::SetProcessDPIAware()

$h = [WinAny]::FindWindowW([IntPtr]::Zero, $Title)
if ($h -eq [IntPtr]::Zero) { Write-Output 'NOT_FOUND'; exit 1 }
if (-not [WinAny]::IsWindowVisible($h)) { Write-Output 'HIDDEN'; exit 1 }

$r = New-Object 'WinAny+RECT'
[void][WinAny]::GetClientRect($h, [ref]$r)
$w = $r.R - $r.L
$hgt = $r.B - $r.T
if ($w -le 0 -or $hgt -le 0) { Write-Output 'EMPTY_RECT'; exit 1 }

$dir = Split-Path -Parent $Out
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }

$bmp = New-Object System.Drawing.Bitmap $w, $hgt
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $gfx.GetHdc()
$ok = [WinAny]::PrintWindow($h, $hdc, 2)
$gfx.ReleaseHdc($hdc)

# A transparent (layered) window comes back with alpha=0 everywhere, which
# renders as solid black. Force alpha opaque so the drawn RGB is visible.
$rect = New-Object System.Drawing.Rectangle 0, 0, $w, $hgt
$data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadWrite, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$len = $data.Stride * $data.Height
$buf = New-Object byte[] $len
[System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $buf, 0, $len)
for ($i = 3; $i -lt $len; $i += 4) { $buf[$i] = 255 }
[System.Runtime.InteropServices.Marshal]::Copy($buf, 0, $data.Scan0, $len)
$bmp.UnlockBits($data)

$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()

$status = 'PARTIAL'
if ($ok) { $status = 'OK' }
Write-Output ($status + ' ' + $w + 'x' + $hgt + ' -> ' + $Out)
