use super::truncation::truncate_to_limit;
use crate::util::grapheme_len;

/// Join segments; while over budget, drop highest priority number first.
pub fn join_with_priority_drop(
    segments: &[(u32, String)],
    sep: &str,
    max: usize,
    ellipsis: &str,
) -> String {
    let mut segs: Vec<(u32, String)> = segments
        .iter()
        .filter(|(_, s)| !s.trim().is_empty())
        .cloned()
        .collect();

    loop {
        let joined = join(&segs, sep);
        if grapheme_len(&joined) <= max {
            return joined;
        }
        if segs.len() <= 1 {
            return truncate_to_limit(&joined, max, ellipsis);
        }
        // Drop highest priority value (worse priority).
        if let Some(idx) = segs
            .iter()
            .enumerate()
            .max_by_key(|(_, (p, _))| *p)
            .map(|(i, _)| i)
        {
            segs.remove(idx);
        } else {
            return truncate_to_limit(&joined, max, ellipsis);
        }
    }
}

fn join(segs: &[(u32, String)], sep: &str) -> String {
    segs.iter()
        .map(|(_, s)| s.as_str())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(sep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_worst_priority() {
        let segs = vec![
            (10, "STATUS".into()),
            (90, "X".repeat(140)),
            (20, "MUSIC".into()),
        ];
        let out = join_with_priority_drop(&segs, " | ", 144, "…");
        assert!(out.contains("STATUS"));
        assert!(!out.contains(&"X".repeat(50)) || grapheme_len(&out) <= 144);
        assert!(grapheme_len(&out) <= 144);
    }
}
