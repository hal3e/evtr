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
    use super::visible_window;

    #[test]
    fn visible_window_basic() {
        assert_eq!(visible_window(10, 0, 5), (0, 5));
        assert_eq!(visible_window(10, 8, 5), (5, 5));
        assert_eq!(visible_window(10, 5, 5), (5, 5));
        assert_eq!(visible_window(3, 0, 10), (0, 3));
    }
}
