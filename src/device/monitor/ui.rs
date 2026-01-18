const ELLIPSIS: &str = "\u{2026}";

pub fn truncate_utf8(text: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let ellipsis_len = ELLIPSIS.chars().count();
    if max_len <= ellipsis_len {
        return text.chars().take(max_len).collect();
    }
    let mut out = String::new();
    let mut count = 0usize;
    let total_chars = text.chars().count();
    for ch in text.chars() {
        let next_count = count + 1;
        // Reserve space for ellipsis if we will truncate
        if next_count > max_len.saturating_sub(ellipsis_len) && total_chars > max_len {
            out.push_str(ELLIPSIS);
            return out;
        }
        out.push(ch);
        count = next_count;
        if count >= max_len {
            return out;
        }
    }
    out
}

pub fn visible_window(total: usize, offset: usize, capacity: usize) -> (usize, usize) {
    if total == 0 || capacity == 0 {
        return (0, 0);
    }
    // Treat `offset` as the top-most item index. Clamp so that we try to
    // show a full window when near the end, instead of a partial tail.
    let max_start = total.saturating_sub(capacity);
    let start = offset.min(max_start);
    let count = capacity.min(total - start);
    (start, count)
}

#[cfg(test)]
mod tests {
    use super::{ELLIPSIS, truncate_utf8, visible_window};

    #[test]
    fn truncate_utf8_short() {
        assert_eq!(truncate_utf8("hello", 10), "hello");
    }

    #[test]
    fn truncate_utf8_exact() {
        assert_eq!(truncate_utf8("hello", 5), "hello");
    }

    #[test]
    fn truncate_utf8_long() {
        assert_eq!(
            truncate_utf8("hello world", 8),
            format!("hello w{ELLIPSIS}")
        );
    }

    #[test]
    fn truncate_utf8_tiny_widths() {
        assert_eq!(truncate_utf8("hello", 0), "");
        assert_eq!(truncate_utf8("hello", 1), "h");
        assert_eq!(truncate_utf8("hello", 2), format!("h{ELLIPSIS}"));
        assert_eq!(truncate_utf8("hello", 3), format!("he{ELLIPSIS}"));
    }

    #[test]
    fn truncate_utf8_unicode() {
        let text = "\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\u{4e16}\u{754c}";
        assert_eq!(
            truncate_utf8(text, 5),
            format!("\u{3053}\u{3093}\u{306b}\u{3061}{ELLIPSIS}")
        );
    }

    #[test]
    fn visible_window_basic() {
        assert_eq!(visible_window(10, 0, 5), (0, 5));
        assert_eq!(visible_window(10, 8, 5), (5, 5));
        assert_eq!(visible_window(10, 5, 5), (5, 5));
        assert_eq!(visible_window(3, 0, 10), (0, 3));
    }
}
