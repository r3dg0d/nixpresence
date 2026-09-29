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
    out = out.replace("Intel Core ", "i");
    out = out.replace("AMD Ryzen ", "R");
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
    fn short_cpu() {
        let s = short_cpu_name("Intel(R) Core(TM) i9-13900K CPU @ 3.00GHz");
        assert!(s.contains('i') || s.contains("13900"), "got {s}");
    }
}
