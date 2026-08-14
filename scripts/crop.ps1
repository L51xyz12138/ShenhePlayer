param(
  [string]$In,
  [string]$Out,
  [int]$Width = 620,
  [int]$Height = 420,
  # Anchor the crop to a corner of the source image
  [ValidateSet('bottom-right', 'top-left')]
  [string]$Anchor = 'bottom-right',
  [int]$Scale = 1
)

# Crop a region out of a screenshot so details are readable.
# ASCII-only: Windows PowerShell 5.1 reads BOM-less files as the ANSI codepage.

Add-Type -AssemblyName System.Drawing

$src = [System.Drawing.Image]::FromFile($In)
$ox = 0
$oy = 0
if ($Anchor -eq 'bottom-right') {
  $ox = [Math]::Max(0, $src.Width - $Width - 30)
  $oy = [Math]::Max(0, $src.Height - $Height - 20)
}

$crop = New-Object System.Drawing.Bitmap $Width, $Height
$g = [System.Drawing.Graphics]::FromImage($crop)
$g.DrawImage($src,
  (New-Object System.Drawing.Rectangle 0, 0, $Width, $Height),
  (New-Object System.Drawing.Rectangle $ox, $oy, $Width, $Height),
  [System.Drawing.GraphicsUnit]::Pixel)
$g.Dispose()

if ($Scale -gt 1) {
  $big = New-Object System.Drawing.Bitmap ($Width * $Scale), ($Height * $Scale)
  $g2 = [System.Drawing.Graphics]::FromImage($big)
  $g2.InterpolationMode = 'NearestNeighbor'
  $g2.DrawImage($crop, 0, 0, ($Width * $Scale), ($Height * $Scale))
  $g2.Dispose()
  $big.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
  $big.Dispose()
} else {
  $crop.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
}

$crop.Dispose()
$src.Dispose()
Write-Output ('OK ' + $Out)
