param(
  # Source artwork: any format System.Drawing can read (png / jpg / webp via WIC / bmp)
  [Parameter(Mandatory = $true)][string]$In,
  [string]$OutDir = 'D:\code\ShenhePlayer\src-tauri\icons',
  # Background for artwork with transparency, e.g. '#1B2A44'. Empty = keep transparent.
  [string]$Background = '',
  # Shrink the artwork inside the icon, 0 = fill edge to edge
  [double]$Inset = 0.0,
  # Skip the rounded-square mask and keep the full square
  [switch]$NoMask,
  # How much of the source to keep, 1.0 = the whole square.
  # Crop in so the subject still reads at 16-32px in the taskbar.
  [double]$Zoom = 1.0,
  # Where to center that crop, normalized 0..1 over the source image
  [double]$FocusX = 0.5,
  [double]$FocusY = 0.5
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

# GDI+ cannot read WebP / HEIF. WIC can (Windows ships those codecs), so fall
# back to a WPF BitmapDecoder and copy the pixels into a GDI+ bitmap.
function Import-Artwork([string]$path) {
  $full = (Resolve-Path $path).Path
  try {
    $img = [System.Drawing.Image]::FromFile($full)
    if ($img) { return $img }
  } catch {
    # 注意用 Write-Host：函数里 Write-Output 的内容也会算进返回值，
    # 调用方拿到的就变成 [诊断字符串, 位图] 的数组了
    Write-Host '  GDI+ 读不了这个格式，改用 WIC 解码'
  }

  Add-Type -AssemblyName PresentationCore, WindowsBase
  $decoder = [System.Windows.Media.Imaging.BitmapDecoder]::Create(
    (New-Object System.Uri $full),
    [System.Windows.Media.Imaging.BitmapCreateOptions]::PreservePixelFormat,
    [System.Windows.Media.Imaging.BitmapCacheOption]::OnLoad)

  $conv = New-Object System.Windows.Media.Imaging.FormatConvertedBitmap
  $conv.BeginInit()
  $conv.Source = $decoder.Frames[0]
  $conv.DestinationFormat = [System.Windows.Media.PixelFormats]::Bgra32
  $conv.EndInit()

  $w = $conv.PixelWidth
  $h = $conv.PixelHeight
  $stride = $w * 4
  $buf = New-Object byte[] ($stride * $h)
  $conv.CopyPixels($buf, $stride, 0)

  $bmp = New-Object System.Drawing.Bitmap $w, $h,
    ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
  $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::WriteOnly,
    [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  for ($y = 0; $y -lt $h; $y++) {
    [System.Runtime.InteropServices.Marshal]::Copy(
      $buf, $y * $stride, [IntPtr]($data.Scan0.ToInt64() + $y * $data.Stride), $stride)
  }
  $bmp.UnlockBits($data)
  return $bmp
}

$src = Import-Artwork $In
if (-not $src) { Write-Output 'DECODE_FAILED'; exit 1 }
Write-Output ("source: " + $src.Width + "x" + $src.Height)

# Crop to a square around the focus point so nothing gets stretched
$side = [int]([Math]::Min($src.Width, $src.Height) * [Math]::Min(1.0, [Math]::Max(0.05, $Zoom)))
$sx = [int]($src.Width * $FocusX - $side / 2)
$sy = [int]($src.Height * $FocusY - $side / 2)
# Keep the crop inside the image
$sx = [Math]::Max(0, [Math]::Min($sx, $src.Width - $side))
$sy = [Math]::Max(0, [Math]::Min($sy, $src.Height - $side))
Write-Output ("crop: " + $side + "x" + $side + " at " + $sx + "," + $sy)

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
