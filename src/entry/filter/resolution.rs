


pub fn from(s: Vec<u8>) -> Result<(i64, i64), String> {
    let ok = match s.as_slice() {
        b"2K" => (2048, 1080),
        b"4K" => (3840, 2160),
        b"5K" | b"UHD+" => (5120, 2880),
        b"8K" => (7680, 4320),
        b"DCI4K"  => (4096, 2160),
        b"DVGA" => (960, 640),
        b"FHD" => (1920, 1080),
        b"FHD+" => (2160, 1440),
        b"FWVGA" => (854, 480),
        b"HD" => (1280, 720),
        b"HD+" => (1600, 900),
        b"HQVGA" => (240, 160),
        b"HSXGA" => (5120, 4096),
        b"HUXGA" => (6400, 4800),
        b"HVGA" => (480, 320),
        b"HXGA" => (4096, 3072),
        b"QHD" | b"WQHD" => (2560, 1440),
        b"QHD+" => (3200, 1800),
        b"QQVGA" => (160, 120),
        b"QSXGA" => (2560, 2048),
        b"QUXGA" => (3200, 2400),
        b"QVGA" => (320, 240),
        b"QWXGA" => (2048, 1152),
        b"QXGA" => (2048, 1536),
        b"SVGA" => (800, 600),
        b"SXGA" => (1280, 1024),
        b"SXGA+" => (1400, 1050),
        b"UW4K" => (3840, 1600),
        b"UW5K" => (5120, 2160),
        b"UW8K" => (7680, 3200),
        b"UWQHD" => (3440, 1440),
        b"UXGA" => (1600, 1200),
        b"VGA" | b"SD" => (640, 480),
        b"WHSXGA" => (6400, 4096),
        b"WHUXGA" => (7680, 4800),
        b"WHXGA" => (5120, 3200),
        b"WQSXGA" => (3200, 2048),
        b"WQUXGA" => (3840, 2400),
        b"WQVGA" => (400, 240),
        b"WQXGA" => (2560, 1600),
        b"WSVGA" => (1024, 600),
        b"WSXGA+" => (1680, 1050),
        b"WUXGA" => (1920, 1200),
        b"WVGA" => (768, 480),
        b"WXGA" => (1366, 768),
        b"WXGA+" => (1440, 900),
        b"XGA" => (1024, 768),
        b"XGA+" => (1152, 864),
        b"nHD" => (640, 360),
        b"qHD" => (960, 540),
        _ => return Err(format!("Invalid resolution name: {}", String::from_utf8(s).unwrap_or_else(|it| s!(it))))
    };
    Ok(ok)
}

pub fn to(w: i64, h: i64) -> String {
    let result = match (w, h) {
        (1024, 600) => "WSVGA",
        (1024, 768) => "XGA",
        (1152, 864) => "XGA+",
        (1280, 1024) => "SXGA",
        (1280, 720) => "HD",
        (1366, 768) => "WXGA",
        (1400, 1050) => "SXGA+",
        (1440, 900) => "WXGA+",
        (160, 120) => "QQVGA",
        (1600, 1200) => "UXGA",
        (1600, 900) => "HD+",
        (1680, 1050) => "WSXGA+",
        (1920, 1080) => "FHD",
        (1920, 1200) => "WUXGA",
        (2048, 1080) => "2K",
        (2048, 1152) => "QWXGA",
        (2048, 1536) => "QXGA",
        (2160, 1440) => "FHD+",
        (240, 160) => "HQVGA",
        (2560, 1440) => "QHD",
        // (2560, 1440) => "WQHD",
        (2560, 1600) => "WQXGA",
        (2560, 2048) => "QSXGA",
        (320, 240) => "QVGA",
        (3200, 1800) => "QHD+",
        (3200, 2048) => "WQSXGA",
        (3200, 2400) => "QUXGA",
        (3440, 1440) => "UWQHD",
        (3840, 1600) => "UW4K",
        (3840, 2160) => "4K",
        (3840, 2400) => "WQUXGA",
        (400, 240) => "WQVGA",
        (4096, 2160) => "DCI4K",
        (4096, 3072) => "HXGA",
        (480, 320) => "HVGA",
        (5120, 2160) => "UW5K",
        (5120, 2880) => "5K",
        (5120, 3200) => "WHXGA",
        (5120, 4096) => "HSXGA",
        (640, 360) => "nHD",
        // (640, 480) => "SD",
        (640, 480) => "VGA",
        (6400, 4096) => "WHSXGA",
        (6400, 4800) => "HUXGA",
        (768, 480) => "WVGA",
        (7680, 3200) => "UW8K",
        (7680, 4320) => "8K",
        (7680, 4800) => "WHUXGA",
        (800, 600) => "SVGA",
        (854, 480) => "FWVGA",
        (960, 540) => "qHD",
        (960, 640) => "DVGA",
        // (5120, 2880) => "UHD+",
        _ => return format!("{}x{}", w, h),
    };
    o!(result)
}
