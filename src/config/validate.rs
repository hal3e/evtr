use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Color;

use super::{KeyBinding, SelectorLayoutConfig};
use crate::error::{Error, Result};

pub(crate) fn parse_bindings(values: Vec<String>, field: &str) -> Result<Vec<KeyBinding>> {
    if values.is_empty() {
        return Err(Error::config(format!("{field} must not be empty")));
    }
    values
        .into_iter()
        .map(|value| {
            KeyBinding::parse(&value)
                .map_err(|_| Error::config(format!("invalid key binding in {field}: {value}")))
        })
        .collect()
}

pub(crate) fn validate_unique_key_bindings<'a, I>(entries: I, scope: &str) -> Result<()>
where
    I: IntoIterator<Item = (&'a str, &'a [KeyBinding])>,
{
    let mut seen: HashMap<(KeyCode, KeyModifiers), &str> = HashMap::new();
    for (action, bindings) in entries {
        for binding in bindings {
            let key = (binding.code(), binding.modifiers());
            if let Some(existing) = seen.insert(key, action) {
                return Err(Error::config(format!(
                    "duplicate binding in {scope}: {} is assigned to both {existing} and {action}",
                    binding.display()
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_selector_layout(layout: SelectorLayoutConfig) -> Result<()> {
    if layout.margin_percent > 50 {
        return Err(Error::config(
            "layout.selector.margin_percent must be at most 50",
        ));
    }
    if layout.margin_percent.saturating_mul(2) + layout.content_width_percent != 100 {
        return Err(Error::config(
            "layout.selector.margin_percent * 2 + content_width_percent must equal 100",
        ));
    }
    Ok(())
}

pub(crate) fn require_positive_i32(value: i32, field: &str) -> Result<i32> {
    if value <= 0 {
        return Err(Error::config(format!("{field} must be greater than 0")));
    }
    Ok(value)
}

pub(crate) fn require_positive_usize(value: usize, field: &str) -> Result<usize> {
    if value == 0 {
        return Err(Error::config(format!("{field} must be greater than 0")));
    }
    Ok(value)
}

pub(crate) fn require_range_usize(
    value: usize,
    min: usize,
    max: usize,
    field: &str,
) -> Result<usize> {
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(Error::config(format!(
            "{field} must be between {min} and {max}"
        )))
    }
}

pub(crate) fn require_range_u16(value: u16, min: u16, max: u16, field: &str) -> Result<u16> {
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(Error::config(format!(
            "{field} must be between {min} and {max}"
        )))
    }
}

pub(crate) fn parse_hex_color_field(raw: &str, field: &str) -> Result<Color> {
    parse_hex_color(raw).map_err(|_| Error::config(format!("invalid color for {field}: {raw}")))
}

pub(crate) fn parse_hex_color(raw: &str) -> std::result::Result<Color, ()> {
    let bytes = raw.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return Err(());
    }

    let red = u8::from_str_radix(&raw[1..3], 16).map_err(|_| ())?;
    let green = u8::from_str_radix(&raw[3..5], 16).map_err(|_| ())?;
    let blue = u8::from_str_radix(&raw[5..7], 16).map_err(|_| ())?;
    Ok(Color::Rgb(red, green, blue))
}

#[cfg(test)]
mod tests {
    use super::parse_hex_color;

    #[test]
    fn parse_hex_color_accepts_rgb_values() {
        assert!(parse_hex_color("#112233").is_ok());
        assert!(parse_hex_color("112233").is_err());
    }
}
