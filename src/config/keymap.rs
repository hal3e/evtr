use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::error::{Error, Result};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct KeyBinding {
    code: KeyCode,
    modifiers: KeyModifiers,
    display: String,
}

impl KeyBinding {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        parse_key_binding(value)
    }

    pub(crate) fn matches(&self, key: KeyEvent) -> bool {
        self.code == key.code && self.modifiers == normalized_modifiers(key.modifiers)
    }

    pub(crate) fn display(&self) -> &str {
        &self.display
    }

    pub(crate) fn code(&self) -> KeyCode {
        self.code
    }

    pub(crate) fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }
}

fn parse_key_binding(raw: &str) -> Result<KeyBinding> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(Error::config("key binding must not be empty"));
    }

    let mut parts: Vec<&str> = normalized.split('-').collect();
    let base = parts
        .pop()
        .ok_or_else(|| Error::config("key binding must include a key"))?;
    let mut modifiers = KeyModifiers::NONE;
    for modifier in parts {
        match modifier {
            "ctrl" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" => modifiers |= KeyModifiers::ALT,
            _ => {
                return Err(Error::config(format!(
                    "unsupported key modifier in binding: {modifier}"
                )));
            }
        }
    }

    let code = match base {
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        value if value.chars().count() == 1 => {
            let mut ch = value
                .chars()
                .next()
                .ok_or_else(|| Error::config("empty key"))?;
            if modifiers.contains(KeyModifiers::SHIFT) && ch.is_ascii_alphabetic() {
                ch = ch.to_ascii_uppercase();
            }
            KeyCode::Char(ch)
        }
        _ => {
            return Err(Error::config(format!(
                "unsupported key binding base: {base}"
            )));
        }
    };

    Ok(KeyBinding {
        code,
        modifiers,
        display: canonical_key_display(code, modifiers),
    })
}

fn canonical_key_display(code: KeyCode, modifiers: KeyModifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    if modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }

    let key = match code {
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Char(ch) => ch.to_string(),
        _ => "Unsupported".to_string(),
    };
    parts.push(key);
    parts.join("-")
}

fn normalized_modifiers(modifiers: KeyModifiers) -> KeyModifiers {
    modifiers & (KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT)
}

pub(crate) fn key_list(specs: &[&str]) -> Vec<KeyBinding> {
    specs
        .iter()
        .map(|spec| KeyBinding::parse(spec).expect("default key binding must parse"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::KeyBinding;

    #[test]
    fn parse_key_binding_supports_modified_and_plain_keys() {
        assert_eq!(KeyBinding::parse("ctrl-c").unwrap().display(), "Ctrl-c");
        assert_eq!(KeyBinding::parse("shift-g").unwrap().display(), "Shift-G");
        assert_eq!(KeyBinding::parse("?").unwrap().display(), "?");
    }
}
