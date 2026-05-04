$b = [System.IO.File]::ReadAllBytes("D:\GTPSP-decompile\files\decompiled\Gran Turismo\PSP_GAME\USRDIR\GT.VOL\car\race\00010001")

# Let's see what's at various offsets after the header
function Get-UInt32LE($data, $offset) { return [BitConverter]::ToUInt32($data, $offset) }
function Get-Float($data, $offset) { return [BitConverter]::ToSingle($data, $offset) }

Write-Host "Dumping bytes 0xE4-0x1E4 (first 256 bytes after header):"
for ($row = 0; $row -lt 16; $row++) {
    $base = 0xE4 + $row * 16
    $hex = ""
    $ascii = ""
    for ($col = 0; $col -lt 16; $col++) {
        $by = $b[$base + $col]
        $hex += [string]::Format("{0:X2} ", $by)
        if ($by -ge 32 -and $by -lt 127) { $ascii += [char]$by } else { $ascii += "." }
    }
    Write-Host ([string]::Format("{0:X4}: {1} |{2}|", $base, $hex, $ascii))
}

# Check if there's another 3LDM magic later in the file
Write-Host ""
Write-Host "Searching for '3LDM' magic in file:"
for ($off = 0xE4; $off -lt $b.Length - 4; $off += 4) {
    if ($b[$off] -eq 0x33 -and $b[$off+1] -eq 0x4C -and $b[$off+2] -eq 0x44 -and $b[$off+3] -eq 0x4D) {
        Write-Host "  Found at offset 0x" ([string]::Format("{0:X}", $off))
        # Dump header at this location
        Write-Host "    file_size: " (Get-UInt32LE $b ($off + 4))
        Write-Host "    model_count: " ([BitConverter]::ToUInt16($b, $off + 16))
        Write-Host "    shape_count: " ([BitConverter]::ToUInt16($b, $off + 20))
        Write-Host "    meshes_ptr: " ([string]::Format("0x{0:X}", (Get-UInt32LE $b ($off + 56))))
    }
}

# Let's also check around offset 0x4D4 which was flagged earlier as potential pointer
Write-Host ""
Write-Host "Checking offset 0x4D4:"
for ($i = 0; $i -lt 10; $i++) {
    $base = 0x4D4 + $i * 12
    $x = Get-Float $b $base
    $y = Get-Float $b ($base + 4)
    $z = Get-Float $b ($base + 8)
    Write-Host ([string]::Format("  {0:X}: ({1:F3}, {2:F3}, {3:F3})", $base, $x, $y, $z))
}