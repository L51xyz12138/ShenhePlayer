param(
  # Source artwork: any format System.Drawing can read (png / jpg / webp via WIC / bmp)
  [Parameter(Mandatory = $true)][string]$In,
  [string]$OutDir = 'D:\code\ShenhePlayer\src-tauri\icons',
  # Background for artwork with transparency, e.g. '#1B2A44'. Empty = keep transparent.
  [string]$Background = '',
  # Shrink the artwork inside the icon, 0 = fill edge to edge
  [double]$Inset = 0.0,
  # Skip the rounded-square mask and keep the full square
  [switch]$NoMask
)

# Turn a piece of artwork into the full Windows/Tauri icon set.
#
# Applies the same superellipse (squircle) mask the generated icon used, so a
# character portrait still reads as a proper app icon instead of a raw square.
# Written against System.Drawing so it can ingest whatever format the user has.
#
# ASCII-only: Windows PowerShell 5.1 reads BOM-less files as the ANSI codepage.

Add-Type -AssemblyName System.Drawing

if (-not (Test-Path $In)) { Write-Output "NOT_FOUND: $In"; exit 1 }
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir -Force | Out-Null }

$src = [System.Drawing.Image]::FromFile((Resolve-Path $In))
Write-Output ("source: " + $src.Width + "x" + $src.Height)

# Center-crop to a square so nothing gets stretched
$side = [Math]::Min($src.Width, $src.Height)
$sx = [int](($src.Width - $side) / 2)
$sy = [int](($src.Height - $side) / 2)

function New-IconBitmap([int]$size) {
  $bmp = New-Object System.Drawing.Bitmap $size, $size,
    ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality

  if ($Background -ne '') {
    $c = [System.Drawing.ColorTranslator]::FromHtml($Background)
    $g.Clear($c)
  }

  $pad = [int]($size * $Inset)
  $dest = New-Object System.Drawing.Rectangle $pad, $pad, ($size - 2 * $pad), ($size - 2 * $pad)
  $from = New-Object System.Drawing.Rectangle $sx, $sy, $side, $side
  $g.DrawImage($src, $dest, $from, [System.Drawing.GraphicsUnit]::Pixel)
  $g.Dispose()

  if (-not $NoMask) { Apply-Squircle $bmp $size }
  return $bmp
}

# Superellipse |x|^n + |y|^n = 1 with n=5 is close to Apple's icon silhouette.
# Corners stay curvature-continuous, so it does not read as a rounded rectangle.
function Apply-Squircle([System.Drawing.Bitmap]$bmp, [int]$size) {
  $rect = New-Object System.Drawing.Rectangle 0, 0, $size, $size
  $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadWrite,
    [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $len = $data.Stride * $data.Height
  $buf = New-Object byte[] $len
  [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $buf, 0, $len)

  $half = $size / 2.0
  $n = 5.0
  # Supersample the edge so small sizes do not look jagged
  $ss = 3

  for ($y = 0; $y -lt $size; $y++) {
    $row = $y * $data.Stride
    for ($x = 0; $x -lt $size; $x++) {
      $hits = 0
      for ($j = 0; $j -lt $ss; $j++) {
        for ($i = 0; $i -lt $ss; $i++) {
          $px = ($x + ($i + 0.5) / $ss - $half) / $half
          $py = ($y + ($j + 0.5) / $ss - $half) / $half
          if ([Math]::Pow([Math]::Abs($px), $n) + [Math]::Pow([Math]::Abs($py), $n) -le 1.0) {
            $hits++
          }
        }
      }
      if ($hits -lt ($ss * $ss)) {
        $idx = $row + $x * 4 + 3
        $buf[$idx] = [byte]([Math]::Round($buf[$idx] * ($hits / [double]($ss * $ss))))
      }
    }
  }

  [System.Runtime.InteropServices.Marshal]::Copy($buf, 0, $data.Scan0, $len)
  $bmp.UnlockBits($data)
}

function Save-Png([System.Drawing.Bitmap]$bmp, [string]$name) {
  $bmp.Save((Join-Path $OutDir $name), [System.Drawing.Imaging.ImageFormat]::Png)
}

# Vista+ ICO can embed PNG frames directly
function Write-Ico([int[]]$sizes, [string]$path) {
  $frames = @()
  foreach ($s in $sizes) {
    $bmp = New-IconBitmap $s
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $frames += , @{ size = $s; bytes = $ms.ToArray() }
    $ms.Dispose(); $bmp.Dispose()
  }

  $fs = [System.IO.File]::Create($path)
  $bw = New-Object System.IO.BinaryWriter $fs
  $bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$frames.Count)

  $offset = 6 + 16 * $frames.Count
  foreach ($f in $frames) {
    $dim = if ($f.size -ge 256) { 0 } else { $f.size }
    $bw.Write([byte]$dim); $bw.Write([byte]$dim)
    $bw.Write([byte]0); $bw.Write([byte]0)
    $bw.Write([uint16]1); $bw.Write([uint16]32)
    $bw.Write([uint32]$f.bytes.Length); $bw.Write([uint32]$offset)
    $offset += $f.bytes.Length
  }
  foreach ($f in $frames) { $bw.Write($f.bytes) }
  $bw.Flush(); $bw.Close(); $fs.Close()
}

foreach ($pair in @(@(32, '32x32.png'), @(128, '128x128.png'), @(256, '128x128@2x.png'), @(512, 'icon.png'))) {
  $bmp = New-IconBitmap $pair[0]
  Save-Png $bmp $pair[1]
  $bmp.Dispose()
  Write-Output ("  " + $pair[1] + " (" + $pair[0] + "x" + $pair[0] + ")")
}

Write-Ico @(16, 20, 24, 32, 48, 64, 128, 256) (Join-Path $OutDir 'icon.ico')
Write-Output '  icon.ico (16/20/24/32/48/64/128/256)'

$src.Dispose()
Write-Output ("done -> " + $OutDir)
