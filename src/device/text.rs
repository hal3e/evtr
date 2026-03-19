use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const ELLIPSIS: &str = "\u{2026}";

pub(crate) fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub(crate) fn truncate_display_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let ellipsis_width = display_width(ELLIPSIS);
    if max_width <= ellipsis_width {
        let prefix = take_display_width(text, max_width);
        return if prefix.is_empty() {
            ELLIPSIS.to_string()
        } else {
            prefix
        };
    }

    let prefix = take_display_width(text, max_width - ellipsis_width);
    if prefix.is_empty() {
        ELLIPSIS.to_string()
    } else {
        format!("{prefix}{ELLIPSIS}")
    }
}

fn take_display_width(text: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut width = 0usize;

    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        out.push(ch);
        width += ch_width;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{ELLIPSIS, display_width, truncate_display_width};

    #[test]
    fn display_width_counts_terminal_cells() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("界界"), 4);
        assert_eq!(display_width("e\u{301}"), 1);
    }

    #[test]
    fn truncate_display_width_preserves_ascii_behavior() {
        assert_eq!(truncate_display_width("hello", 10), "hello");
        assert_eq!(truncate_display_width("hello", 5), "hello");
        assert_eq!(
            truncate_display_width("hello world", 8),
            format!("hello w{ELLIPSIS}")
        );
        assert_eq!(truncate_display_width("hello", 0), "");
        assert_eq!(truncate_display_width("hello", 1), "h");
        assert_eq!(truncate_display_width("hello", 2), format!("h{ELLIPSIS}"));
        assert_eq!(truncate_display_width("hello", 3), format!("he{ELLIPSIS}"));
    }

    #[test]
    fn truncate_display_width_handles_wide_characters() {
        assert_eq!(
            truncate_display_width("hello界界", 7),
            format!("hello{ELLIPSIS}")
        );
        assert_eq!(truncate_display_width("界界", 1), ELLIPSIS);
        assert_eq!(truncate_display_width("界界", 2), ELLIPSIS);
    }

    #[test]
    fn truncate_display_width_handles_combining_marks() {
        assert_eq!(truncate_display_width("e\u{301}abc", 4), "e\u{301}abc");
        assert_eq!(
            truncate_display_width("e\u{301}abcdef", 4),
            format!("e\u{301}ab{ELLIPSIS}")
        );
    }
}
