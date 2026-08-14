param([string]$Title = 'ShenhePlayer')

# Verify the mpv host child window exactly covers the player window's client
# area. Any gap here is what shows up as a stray edge (the WebView2 underneath
# defaults to white). Pure measurement - does not touch focus or the screen.
#
# ASCII-only: Windows PowerShell 5.1 reads BOM-less files as the ANSI codepage.

Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Geo {
  public delegate bool Proc(IntPtr h, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(IntPtr cls, string title);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, Proc p, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern int MapWindowPoints(IntPtr from, IntPtr to, ref RECT r, uint count);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }

  public static IntPtr FindHost(IntPtr parent) {
    IntPtr found = IntPtr.Zero;
    EnumChildWindows(parent, (h, l) => {
      var c = new StringBuilder(128); GetClassNameW(h, c, 128);
      if (c.ToString() == "ShenheVideoHost") { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@

[void][Geo]::SetProcessDPIAware()

$player = [Geo]::FindWindowW([IntPtr]::Zero, $Title)
if ($player -eq [IntPtr]::Zero) { Write-Output 'PLAYER_NOT_FOUND'; exit 1 }

$host_ = [Geo]::FindHost($player)
if ($host_ -eq [IntPtr]::Zero) { Write-Output 'HOST_NOT_FOUND'; exit 1 }

$clientRect = New-Object 'Geo+RECT'
[void][Geo]::GetClientRect($player, [ref]$clientRect)

# Host rect in screen coords -> convert to the player's client coords
$hostRect = New-Object 'Geo+RECT'
[void][Geo]::GetWindowRect($host_, [ref]$hostRect)
[void][Geo]::MapWindowPoints([IntPtr]::Zero, $player, [ref]$hostRect, 2)

$cw = $clientRect.R - $clientRect.L
$ch = $clientRect.B - $clientRect.T
$hw = $hostRect.R - $hostRect.L
$hh = $hostRect.B - $hostRect.T

Write-Output ('player client : ' + $cw + 'x' + $ch)
Write-Output ('video host    : ' + $hw + 'x' + $hh + ' at (' + $hostRect.L + ',' + $hostRect.T + ')')
Write-Output ('host visible  : ' + [Geo]::IsWindowVisible($host_))

if ($hostRect.L -eq 0 -and $hostRect.T -eq 0 -and $hw -eq $cw -and $hh -eq $ch) {
  Write-Output 'RESULT: OK - host exactly covers the client area, no gap'
} else {
  $gapR = $cw - $hostRect.R
  $gapB = $ch - $hostRect.B
  Write-Output ('RESULT: GAP - left=' + $hostRect.L + ' top=' + $hostRect.T + ' right=' + $gapR + ' bottom=' + $gapB)
}
