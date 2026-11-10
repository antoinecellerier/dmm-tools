# Builds a Windows .ico from an SVG icon: each size is rendered from the
# vector by headless Edge (sharper at 16 px than scaling a large PNG down) and
# stored as a PNG entry. The build scripts embed the result in the exes, for
# Explorer and shortcuts (crates/dmm-gui/build.rs, crates/dmm-cli/build.rs).
#
#   pwsh scripts/make-ico.ps1 assets/icon.svg assets/icon.ico
#   pwsh scripts/make-ico.ps1 assets/icon-cli.svg assets/icon-cli.ico
param(
    [Parameter(Mandatory)][string]$Svg,
    [Parameter(Mandatory)][string]$Ico
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$sizes = 16, 20, 24, 32, 40, 48, 64, 96, 128, 256
$edge = @(
    "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge not found' }

$svgUri = ([System.Uri](Resolve-Path $Svg).Path).AbsoluteUri
$work = Join-Path ([System.IO.Path]::GetTempPath()) "make-ico-$PID"
New-Item -ItemType Directory -Force $work | Out-Null
try {
    # Every size side by side on one transparent page, one screenshot.
    $x = 0
    $boxes = foreach ($s in $sizes) {
        [pscustomobject]@{ Size = $s; X = $x }
        $x += $s + 8
    }
    $imgs = ($boxes | ForEach-Object {
            "<img src='$svgUri' width='$($_.Size)' height='$($_.Size)' style='position:absolute;left:$($_.X)px;top:0'>"
        }) -join ''
    Set-Content "$work\page.html" "<!doctype html><html style='background:transparent'><body style='margin:0'>$imgs</body></html>"
    $shot = "$work\page.png"
    & $edge --headless --disable-gpu --hide-scrollbars --force-device-scale-factor=1 `
        --default-background-color=00000000 --user-data-dir="$work\profile" `
        --window-size="$x,300" --screenshot="$shot" "$work\page.html" 2>&1 | Out-Null
    if (-not (Test-Path $shot)) { throw 'Edge wrote no screenshot' }

    $page = [System.Drawing.Bitmap]::FromFile($shot)
    $pngs = foreach ($b in $boxes) {
        $crop = $page.Clone((New-Object System.Drawing.Rectangle $b.X, 0, $b.Size, $b.Size), $page.PixelFormat)
        $ms = [System.IO.MemoryStream]::new()
        $crop.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $crop.Dispose()
        , $ms.ToArray()
    }
    $page.Dispose()

    # ICONDIR, one ICONDIRENTRY per size (0 means 256), then the PNGs.
    $buf = [System.IO.MemoryStream]::new()
    $w = [System.IO.BinaryWriter]::new($buf)
    $w.Write([UInt16]0); $w.Write([UInt16]1); $w.Write([UInt16]$sizes.Count)
    $offset = 6 + 16 * $sizes.Count
    for ($i = 0; $i -lt $sizes.Count; $i++) {
        $dim = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
        $w.Write([byte]$dim); $w.Write([byte]$dim); $w.Write([byte]0); $w.Write([byte]0)
        $w.Write([UInt16]1); $w.Write([UInt16]32)
        $w.Write([UInt32]$pngs[$i].Length); $w.Write([UInt32]$offset)
        $offset += $pngs[$i].Length
    }
    foreach ($p in $pngs) { $w.Write($p) }
    $w.Flush()
    [System.IO.File]::WriteAllBytes((Join-Path (Get-Location) $Ico), $buf.ToArray())
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
