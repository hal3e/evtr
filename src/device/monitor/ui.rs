pub fn truncate_utf8(text: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut count = 0usize;
    let total_chars = text.chars().count();
    for ch in text.chars() {
        let next_count = count + 1;
        // Reserve space for ellipsis if we will truncate
        if next_count > max_len.saturating_sub(3) && total_chars > max_len {
            out.push_str("...");
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
