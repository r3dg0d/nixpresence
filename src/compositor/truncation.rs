use crate::util::{grapheme_len, truncate_graphemes};

pub fn truncate_to_limit(s: &str, max: usize, ellipsis: &str) -> String {
    truncate_graphemes(s, max, ellipsis)
}

#[allow(dead_code)]
pub fn fits(s: &str, max: usize) -> bool {
    grapheme_len(s) <= max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_144() {
        let s = "a".repeat(200);
        let t = truncate_to_limit(&s, 144, "…");
        assert_eq!(grapheme_len(&t), 144);
        assert!(t.ends_with('…'));
    }
}
