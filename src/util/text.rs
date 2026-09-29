use unicode_segmentation::UnicodeSegmentation;

/// Count Unicode grapheme clusters (VRChat chatbox budget uses graphemes ≈ chars).
pub fn grapheme_len(s: &str) -> usize {
    s.graphemes(true).count()
}

/// Truncate to at most `max` graphemes, appending `ellipsis` when cut.
pub fn truncate_graphemes(s: &str, max: usize, ellipsis: &str) -> String {
    if max == 0 {
        return String::new();
    }
    let g: Vec<&str> = s.graphemes(true).collect();
    if g.len() <= max {
        return s.to_string();
    }
    let ell_len = ellipsis.graphemes(true).count();
    if ell_len >= max {
        return ellipsis.graphemes(true).take(max).collect();
    }
    let keep = max - ell_len;
    let mut out: String = g.into_iter().take(keep).collect();
    out.push_str(ellipsis);
    out
}

/// Shorten CPU model strings for chatbox.
pub fn short_cpu_name(raw: &str) -> String {
    let mut s = raw.to_string();
    for pat in [
        "(R)",
        "(TM)",
        "(C)",
        " CPU",
        " Processor",
        " with Radeon Graphics",
        " with Radeon Vega Graphics",
    ] {
        s = s.replace(pat, "");
    }
    if let Some(idx) = s.find(" @") {
        s.truncate(idx);
    }
    let parts: Vec<_> = s.split_whitespace().collect();
    let mut out = parts.join(" ");
    // Model strings already contain "i9-" / "i7-" — strip the vendor prefix only.
    // Replacing "Intel Core " with "i" produced the bug "ii9-14900k".
    out = out.replace("Intel Core ", "");
    out = out.replace("Intel ", "");
    out = out.replace("AMD Ryzen ", "R");
    out = out.replace("AMD ", "");
    out = out.replace("Apple ", "");
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grapheme_emoji() {
        assert_eq!(grapheme_len("🎵ab"), 3);
        assert_eq!(truncate_graphemes("abcdefghij", 5, "…"), "abcd…");
        assert_eq!(truncate_graphemes("🎵🎵🎵🎵", 3, "…"), "🎵🎵…");
    }

    #[test]
    fn short_cpu_intel_no_double_i() {
        let s = short_cpu_name("Intel(R) Core(TM) i9-14900K CPU @ 3.20GHz");
        assert_eq!(s, "i9-14900K");
        let s2 = short_cpu_name("Intel(R) Core(TM) i9-13900K CPU @ 3.00GHz");
        assert_eq!(s2, "i9-13900K");
        assert!(!s.contains("ii"), "double-i regression: {s}");
    }

    #[test]
    fn short_cpu_amd_ryzen() {
        let s = short_cpu_name("AMD Ryzen 9 7950X 16-Core Processor");
        assert!(s.starts_with('R'), "got {s}");
        assert!(s.contains("7950"), "got {s}");
    }
}
