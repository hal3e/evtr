use std::fmt;

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::KeyBinding;
use crate::config::keymap::key_list;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    pub(crate) selector: SelectorConfig,
    pub(crate) monitor: MonitorConfig,
    pub(crate) theme: ThemeConfig,
    pub(crate) layout: LayoutConfig,
    pub(crate) keys: KeymapConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectorConfig {
    pub(crate) sort: SortOrder,
    pub(crate) page_scroll_size: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SortOrder {
    Path,
    Name,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MonitorConfig {
    pub(crate) page_scroll_steps: usize,
    pub(crate) startup_focus: StartupFocus,
    pub(crate) joystick_invert_y: bool,
    pub(crate) relative_display_range: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StartupFocus {
    Auto,
    Axes,
    Buttons,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThemeConfig {
    pub(crate) palette: ThemePalette,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThemePalette {
    pub(crate) text: Color,
    pub(crate) muted: Color,
    pub(crate) accent: Color,
    pub(crate) accent_strong: Color,
    pub(crate) danger: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutConfig {
    pub(crate) selector: SelectorLayoutConfig,
    pub(crate) monitor: MonitorLayoutConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectorLayoutConfig {
    pub(crate) margin_percent: u16,
    pub(crate) content_width_percent: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MonitorLayoutConfig {
    pub(crate) buttons_per_row: usize,
    pub(crate) main_column_percent: u16,
    pub(crate) joystick_gap: u16,
    pub(crate) axes_box_percent: u16,
    pub(crate) joystick_hat_joystick_percent: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeymapConfig {
    pub(crate) selector: SelectorKeymap,
    pub(crate) monitor: MonitorKeymap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectorKeymap {
    pub(crate) exit: Vec<KeyBinding>,
    pub(crate) back: Vec<KeyBinding>,
    pub(crate) toggle_help: Vec<KeyBinding>,
    pub(crate) refresh: Vec<KeyBinding>,
    pub(crate) select: Vec<KeyBinding>,
    pub(crate) clear_search: Vec<KeyBinding>,
    pub(crate) delete_char: Vec<KeyBinding>,
    pub(crate) move_up: Vec<KeyBinding>,
    pub(crate) move_down: Vec<KeyBinding>,
    pub(crate) page_up: Vec<KeyBinding>,
    pub(crate) page_down: Vec<KeyBinding>,
    pub(crate) home: Vec<KeyBinding>,
    pub(crate) end: Vec<KeyBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MonitorKeymap {
    pub(crate) back: Vec<KeyBinding>,
    pub(crate) exit: Vec<KeyBinding>,
    pub(crate) reset: Vec<KeyBinding>,
    pub(crate) home: Vec<KeyBinding>,
    pub(crate) end: Vec<KeyBinding>,
    pub(crate) scroll_up: Vec<KeyBinding>,
    pub(crate) scroll_down: Vec<KeyBinding>,
    pub(crate) toggle_info: Vec<KeyBinding>,
    pub(crate) toggle_invert_y: Vec<KeyBinding>,
    pub(crate) toggle_help: Vec<KeyBinding>,
    pub(crate) focus_next: Vec<KeyBinding>,
    pub(crate) focus_prev: Vec<KeyBinding>,
    pub(crate) page_up: Vec<KeyBinding>,
    pub(crate) page_down: Vec<KeyBinding>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            selector: SelectorConfig {
                sort: SortOrder::Path,
                page_scroll_size: 10,
            },
            monitor: MonitorConfig {
                page_scroll_steps: 10,
                startup_focus: StartupFocus::Auto,
                joystick_invert_y: true,
                relative_display_range: 1000,
            },
            theme: ThemeConfig {
                palette: ThemePalette {
                    text: Color::Rgb(201, 210, 244),
                    muted: Color::Rgb(61, 66, 90),
                    accent: Color::Rgb(147, 197, 253),
                    accent_strong: Color::Rgb(96, 165, 250),
                    danger: Color::Rgb(248, 113, 113),
                },
            },
            layout: LayoutConfig {
                selector: SelectorLayoutConfig {
                    margin_percent: 20,
                    content_width_percent: 60,
                },
                monitor: MonitorLayoutConfig {
                    buttons_per_row: 3,
                    main_column_percent: 70,
                    joystick_gap: 2,
                    axes_box_percent: 75,
                    joystick_hat_joystick_percent: 70,
                },
            },
            keys: KeymapConfig::default(),
        }
    }
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            selector: SelectorKeymap {
                exit: key_list(&["ctrl-c"]),
                back: key_list(&["esc"]),
                toggle_help: key_list(&["?"]),
                refresh: key_list(&["ctrl-r"]),
                select: key_list(&["enter"]),
                clear_search: key_list(&["ctrl-u"]),
                delete_char: key_list(&["backspace"]),
                move_up: key_list(&["up", "ctrl-p"]),
                move_down: key_list(&["down", "ctrl-n"]),
                page_up: key_list(&["pageup"]),
                page_down: key_list(&["pagedown"]),
                home: key_list(&["home"]),
                end: key_list(&["end"]),
            },
            monitor: MonitorKeymap {
                back: key_list(&["esc"]),
                exit: key_list(&["ctrl-c"]),
                reset: key_list(&["r"]),
                home: key_list(&["home", "g"]),
                end: key_list(&["end", "shift-g"]),
                scroll_up: key_list(&["up", "k"]),
                scroll_down: key_list(&["down", "j"]),
                toggle_info: key_list(&["i"]),
                toggle_invert_y: key_list(&["y"]),
                toggle_help: key_list(&["?"]),
                focus_next: key_list(&["shift-j"]),
                focus_prev: key_list(&["shift-k"]),
                page_up: key_list(&["pageup"]),
                page_down: key_list(&["pagedown"]),
            },
        }
    }
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path => write!(f, "path"),
            Self::Name => write!(f, "name"),
        }
    }
}
