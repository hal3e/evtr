use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::{
    config::validate::{
        parse_bindings, parse_hex_color_field, require_positive_i32, require_positive_usize,
        require_range_u16, require_range_usize, validate_selector_layout,
        validate_unique_key_bindings,
    },
    error::{Error, Result},
};

use super::{
    KeyBinding,
    types::{
        Config, KeymapConfig, LayoutConfig, MonitorConfig, MonitorKeymap, MonitorLayoutConfig,
        SelectorConfig, SelectorKeymap, SelectorLayoutConfig, SortOrder, StartupFocus, ThemeConfig,
        ThemePalette,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ConfigFile {
    selector: SelectorFile,
    monitor: MonitorFile,
    theme: ThemeFile,
    layout: LayoutFile,
    keys: KeymapFile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SelectorFile {
    sort: SortOrder,
    page_scroll_size: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct MonitorFile {
    page_scroll_steps: usize,
    startup_focus: StartupFocus,
    joystick_invert_y: bool,
    relative_display_range: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ThemeFile {
    palette: ThemePaletteFile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ThemePaletteFile {
    text: String,
    muted: String,
    accent: String,
    accent_strong: String,
    danger: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct LayoutFile {
    selector: SelectorLayoutFile,
    monitor: MonitorLayoutFile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SelectorLayoutFile {
    margin_percent: u16,
    content_width_percent: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct MonitorLayoutFile {
    buttons_per_row: usize,
    main_column_percent: u16,
    joystick_gap: u16,
    axes_box_percent: u16,
    joystick_hat_joystick_percent: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct KeymapFile {
    selector: SelectorKeymapFile,
    monitor: MonitorKeymapFile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SelectorKeymapFile {
    exit: Vec<String>,
    back: Vec<String>,
    toggle_help: Vec<String>,
    refresh: Vec<String>,
    select: Vec<String>,
    clear_search: Vec<String>,
    delete_char: Vec<String>,
    move_up: Vec<String>,
    move_down: Vec<String>,
    page_up: Vec<String>,
    page_down: Vec<String>,
    home: Vec<String>,
    end: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct MonitorKeymapFile {
    back: Vec<String>,
    exit: Vec<String>,
    reset: Vec<String>,
    home: Vec<String>,
    end: Vec<String>,
    scroll_up: Vec<String>,
    scroll_down: Vec<String>,
    toggle_info: Vec<String>,
    toggle_invert_y: Vec<String>,
    toggle_help: Vec<String>,
    focus_next: Vec<String>,
    focus_prev: Vec<String>,
    page_up: Vec<String>,
    page_down: Vec<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self::from(&Config::default())
    }
}

impl ConfigFile {
    pub(crate) fn render_default() -> Result<String> {
        toml::to_string(&Self::default())
            .map_err(|err| Error::config(format!("serialize default config: {err}")))
    }
}

impl Default for SelectorFile {
    fn default() -> Self {
        Config::default().selector.into()
    }
}

impl Default for MonitorFile {
    fn default() -> Self {
        Config::default().monitor.into()
    }
}

impl Default for ThemeFile {
    fn default() -> Self {
        Config::default().theme.into()
    }
}

impl Default for ThemePaletteFile {
    fn default() -> Self {
        Config::default().theme.palette.into()
    }
}

impl Default for LayoutFile {
    fn default() -> Self {
        Config::default().layout.into()
    }
}

impl Default for SelectorLayoutFile {
    fn default() -> Self {
        Config::default().layout.selector.into()
    }
}

impl Default for MonitorLayoutFile {
    fn default() -> Self {
        Config::default().layout.monitor.into()
    }
}

impl Default for KeymapFile {
    fn default() -> Self {
        let defaults = Config::default();
        Self::from(&defaults.keys)
    }
}

impl Default for SelectorKeymapFile {
    fn default() -> Self {
        let defaults = Config::default();
        Self::from(&defaults.keys.selector)
    }
}

impl Default for MonitorKeymapFile {
    fn default() -> Self {
        let defaults = Config::default();
        Self::from(&defaults.keys.monitor)
    }
}

impl From<&Config> for ConfigFile {
    fn from(config: &Config) -> Self {
        Self {
            selector: config.selector.into(),
            monitor: config.monitor.into(),
            theme: config.theme.into(),
            layout: config.layout.into(),
            keys: (&config.keys).into(),
        }
    }
}

impl From<SelectorConfig> for SelectorFile {
    fn from(selector: SelectorConfig) -> Self {
        Self {
            sort: selector.sort,
            page_scroll_size: selector.page_scroll_size,
        }
    }
}

impl From<MonitorConfig> for MonitorFile {
    fn from(monitor: MonitorConfig) -> Self {
        Self {
            page_scroll_steps: monitor.page_scroll_steps,
            startup_focus: monitor.startup_focus,
            joystick_invert_y: monitor.joystick_invert_y,
            relative_display_range: monitor.relative_display_range,
        }
    }
}

impl From<ThemeConfig> for ThemeFile {
    fn from(theme: ThemeConfig) -> Self {
        Self {
            palette: theme.palette.into(),
        }
    }
}

impl From<ThemePalette> for ThemePaletteFile {
    fn from(palette: ThemePalette) -> Self {
        Self {
            text: hex_color(palette.text),
            muted: hex_color(palette.muted),
            accent: hex_color(palette.accent),
            accent_strong: hex_color(palette.accent_strong),
            danger: hex_color(palette.danger),
        }
    }
}

impl From<LayoutConfig> for LayoutFile {
    fn from(layout: LayoutConfig) -> Self {
        Self {
            selector: layout.selector.into(),
            monitor: layout.monitor.into(),
        }
    }
}

impl From<SelectorLayoutConfig> for SelectorLayoutFile {
    fn from(layout: SelectorLayoutConfig) -> Self {
        Self {
            margin_percent: layout.margin_percent,
            content_width_percent: layout.content_width_percent,
        }
    }
}

impl From<MonitorLayoutConfig> for MonitorLayoutFile {
    fn from(layout: MonitorLayoutConfig) -> Self {
        Self {
            buttons_per_row: layout.buttons_per_row,
            main_column_percent: layout.main_column_percent,
            joystick_gap: layout.joystick_gap,
            axes_box_percent: layout.axes_box_percent,
            joystick_hat_joystick_percent: layout.joystick_hat_joystick_percent,
        }
    }
}

impl From<&KeymapConfig> for KeymapFile {
    fn from(keymap: &KeymapConfig) -> Self {
        Self {
            selector: (&keymap.selector).into(),
            monitor: (&keymap.monitor).into(),
        }
    }
}

impl From<&SelectorKeymap> for SelectorKeymapFile {
    fn from(keymap: &SelectorKeymap) -> Self {
        Self {
            exit: export_bindings(&keymap.exit),
            back: export_bindings(&keymap.back),
            toggle_help: export_bindings(&keymap.toggle_help),
            refresh: export_bindings(&keymap.refresh),
            select: export_bindings(&keymap.select),
            clear_search: export_bindings(&keymap.clear_search),
            delete_char: export_bindings(&keymap.delete_char),
            move_up: export_bindings(&keymap.move_up),
            move_down: export_bindings(&keymap.move_down),
            page_up: export_bindings(&keymap.page_up),
            page_down: export_bindings(&keymap.page_down),
            home: export_bindings(&keymap.home),
            end: export_bindings(&keymap.end),
        }
    }
}

impl From<&MonitorKeymap> for MonitorKeymapFile {
    fn from(keymap: &MonitorKeymap) -> Self {
        Self {
            back: export_bindings(&keymap.back),
            exit: export_bindings(&keymap.exit),
            reset: export_bindings(&keymap.reset),
            home: export_bindings(&keymap.home),
            end: export_bindings(&keymap.end),
            scroll_up: export_bindings(&keymap.scroll_up),
            scroll_down: export_bindings(&keymap.scroll_down),
            toggle_info: export_bindings(&keymap.toggle_info),
            toggle_invert_y: export_bindings(&keymap.toggle_invert_y),
            toggle_help: export_bindings(&keymap.toggle_help),
            focus_next: export_bindings(&keymap.focus_next),
            focus_prev: export_bindings(&keymap.focus_prev),
            page_up: export_bindings(&keymap.page_up),
            page_down: export_bindings(&keymap.page_down),
        }
    }
}

impl TryFrom<ConfigFile> for Config {
    type Error = Error;

    fn try_from(file: ConfigFile) -> Result<Self> {
        Ok(Self {
            selector: file.selector.try_into()?,
            monitor: file.monitor.try_into()?,
            theme: file.theme.try_into()?,
            layout: file.layout.try_into()?,
            keys: file.keys.try_into()?,
        })
    }
}

impl TryFrom<SelectorFile> for SelectorConfig {
    type Error = Error;

    fn try_from(file: SelectorFile) -> Result<Self> {
        Ok(Self {
            sort: file.sort,
            page_scroll_size: require_positive_i32(
                file.page_scroll_size,
                "selector.page_scroll_size",
            )?,
        })
    }
}

impl TryFrom<MonitorFile> for MonitorConfig {
    type Error = Error;

    fn try_from(file: MonitorFile) -> Result<Self> {
        Ok(Self {
            page_scroll_steps: require_positive_usize(
                file.page_scroll_steps,
                "monitor.page_scroll_steps",
            )?,
            startup_focus: file.startup_focus,
            joystick_invert_y: file.joystick_invert_y,
            relative_display_range: require_positive_i32(
                file.relative_display_range,
                "monitor.relative_display_range",
            )?,
        })
    }
}

impl TryFrom<ThemeFile> for ThemeConfig {
    type Error = Error;

    fn try_from(file: ThemeFile) -> Result<Self> {
        Ok(Self {
            palette: file.palette.try_into()?,
        })
    }
}

impl TryFrom<ThemePaletteFile> for ThemePalette {
    type Error = Error;

    fn try_from(file: ThemePaletteFile) -> Result<Self> {
        Ok(Self {
            text: parse_hex_color_field(&file.text, "theme.palette.text")?,
            muted: parse_hex_color_field(&file.muted, "theme.palette.muted")?,
            accent: parse_hex_color_field(&file.accent, "theme.palette.accent")?,
            accent_strong: parse_hex_color_field(
                &file.accent_strong,
                "theme.palette.accent_strong",
            )?,
            danger: parse_hex_color_field(&file.danger, "theme.palette.danger")?,
        })
    }
}

impl TryFrom<LayoutFile> for LayoutConfig {
    type Error = Error;

    fn try_from(file: LayoutFile) -> Result<Self> {
        Ok(Self {
            selector: file.selector.try_into()?,
            monitor: file.monitor.try_into()?,
        })
    }
}

impl TryFrom<SelectorLayoutFile> for SelectorLayoutConfig {
    type Error = Error;

    fn try_from(file: SelectorLayoutFile) -> Result<Self> {
        let layout = Self {
            margin_percent: file.margin_percent,
            content_width_percent: file.content_width_percent,
        };
        validate_selector_layout(layout)?;
        Ok(layout)
    }
}

impl TryFrom<MonitorLayoutFile> for MonitorLayoutConfig {
    type Error = Error;

    fn try_from(file: MonitorLayoutFile) -> Result<Self> {
        Ok(Self {
            buttons_per_row: require_range_usize(
                file.buttons_per_row,
                1,
                6,
                "layout.monitor.buttons_per_row",
            )?,
            main_column_percent: require_range_u16(
                file.main_column_percent,
                40,
                90,
                "layout.monitor.main_column_percent",
            )?,
            joystick_gap: require_range_u16(
                file.joystick_gap,
                0,
                8,
                "layout.monitor.joystick_gap",
            )?,
            axes_box_percent: require_range_u16(
                file.axes_box_percent,
                1,
                99,
                "layout.monitor.axes_box_percent",
            )?,
            joystick_hat_joystick_percent: require_range_u16(
                file.joystick_hat_joystick_percent,
                1,
                99,
                "layout.monitor.joystick_hat_joystick_percent",
            )?,
        })
    }
}

impl TryFrom<KeymapFile> for KeymapConfig {
    type Error = Error;

    fn try_from(file: KeymapFile) -> Result<Self> {
        Ok(Self {
            selector: file.selector.try_into()?,
            monitor: file.monitor.try_into()?,
        })
    }
}

impl TryFrom<SelectorKeymapFile> for SelectorKeymap {
    type Error = Error;

    fn try_from(file: SelectorKeymapFile) -> Result<Self> {
        let map = Self {
            exit: parse_bindings(file.exit, "keys.selector.exit")?,
            back: parse_bindings(file.back, "keys.selector.back")?,
            toggle_help: parse_bindings(file.toggle_help, "keys.selector.toggle_help")?,
            refresh: parse_bindings(file.refresh, "keys.selector.refresh")?,
            select: parse_bindings(file.select, "keys.selector.select")?,
            clear_search: parse_bindings(file.clear_search, "keys.selector.clear_search")?,
            delete_char: parse_bindings(file.delete_char, "keys.selector.delete_char")?,
            move_up: parse_bindings(file.move_up, "keys.selector.move_up")?,
            move_down: parse_bindings(file.move_down, "keys.selector.move_down")?,
            page_up: parse_bindings(file.page_up, "keys.selector.page_up")?,
            page_down: parse_bindings(file.page_down, "keys.selector.page_down")?,
            home: parse_bindings(file.home, "keys.selector.home")?,
            end: parse_bindings(file.end, "keys.selector.end")?,
        };
        validate_unique_key_bindings(
            [
                ("exit", &map.exit[..]),
                ("back", &map.back[..]),
                ("toggle_help", &map.toggle_help[..]),
                ("refresh", &map.refresh[..]),
                ("select", &map.select[..]),
                ("clear_search", &map.clear_search[..]),
                ("delete_char", &map.delete_char[..]),
                ("move_up", &map.move_up[..]),
                ("move_down", &map.move_down[..]),
                ("page_up", &map.page_up[..]),
                ("page_down", &map.page_down[..]),
                ("home", &map.home[..]),
                ("end", &map.end[..]),
            ],
            "keys.selector",
        )?;
        Ok(map)
    }
}

impl TryFrom<MonitorKeymapFile> for MonitorKeymap {
    type Error = Error;

    fn try_from(file: MonitorKeymapFile) -> Result<Self> {
        let map = Self {
            back: parse_bindings(file.back, "keys.monitor.back")?,
            exit: parse_bindings(file.exit, "keys.monitor.exit")?,
            reset: parse_bindings(file.reset, "keys.monitor.reset")?,
            home: parse_bindings(file.home, "keys.monitor.home")?,
            end: parse_bindings(file.end, "keys.monitor.end")?,
            scroll_up: parse_bindings(file.scroll_up, "keys.monitor.scroll_up")?,
            scroll_down: parse_bindings(file.scroll_down, "keys.monitor.scroll_down")?,
            toggle_info: parse_bindings(file.toggle_info, "keys.monitor.toggle_info")?,
            toggle_invert_y: parse_bindings(file.toggle_invert_y, "keys.monitor.toggle_invert_y")?,
            toggle_help: parse_bindings(file.toggle_help, "keys.monitor.toggle_help")?,
            focus_next: parse_bindings(file.focus_next, "keys.monitor.focus_next")?,
            focus_prev: parse_bindings(file.focus_prev, "keys.monitor.focus_prev")?,
            page_up: parse_bindings(file.page_up, "keys.monitor.page_up")?,
            page_down: parse_bindings(file.page_down, "keys.monitor.page_down")?,
        };
        validate_unique_key_bindings(
            [
                ("back", &map.back[..]),
                ("exit", &map.exit[..]),
                ("reset", &map.reset[..]),
                ("home", &map.home[..]),
                ("end", &map.end[..]),
                ("scroll_up", &map.scroll_up[..]),
                ("scroll_down", &map.scroll_down[..]),
                ("toggle_info", &map.toggle_info[..]),
                ("toggle_invert_y", &map.toggle_invert_y[..]),
                ("toggle_help", &map.toggle_help[..]),
                ("focus_next", &map.focus_next[..]),
                ("focus_prev", &map.focus_prev[..]),
                ("page_up", &map.page_up[..]),
                ("page_down", &map.page_down[..]),
            ],
            "keys.monitor",
        )?;
        Ok(map)
    }
}

fn export_bindings(bindings: &[KeyBinding]) -> Vec<String> {
    bindings
        .iter()
        .map(|binding| binding.display().to_ascii_lowercase())
        .collect()
}

fn hex_color(color: Color) -> String {
    match color {
        Color::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use toml::Value;

    use super::{ConfigFile, KeymapFile, SelectorKeymapFile};
    use crate::config::{Config, SortOrder};

    #[test]
    fn empty_toml_deserializes_to_runtime_defaults() {
        let file = toml::from_str::<ConfigFile>("").unwrap();
        let resolved = Config::try_from(file).unwrap();

        assert_eq!(resolved, Config::default());
    }

    #[test]
    fn partial_sections_inherit_missing_values_from_defaults() {
        let file = toml::from_str::<ConfigFile>("[selector]\nsort = \"name\"\n").unwrap();
        let resolved = Config::try_from(file).unwrap();

        assert_eq!(resolved.selector.sort, SortOrder::Name);
        assert_eq!(resolved.selector.page_scroll_size, 10);
        assert_eq!(
            resolved.monitor.startup_focus,
            crate::config::StartupFocus::Auto
        );
        assert_eq!(resolved.layout.monitor.axes_box_percent, 75);
        assert_eq!(resolved.layout.monitor.joystick_hat_joystick_percent, 70);
    }

    #[test]
    fn duplicate_bindings_are_rejected_during_runtime_conversion() {
        let file = ConfigFile {
            keys: KeymapFile {
                selector: SelectorKeymapFile {
                    exit: vec!["ctrl-c".to_string()],
                    back: vec!["ctrl-c".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let err = Config::try_from(file).unwrap_err();
        assert!(
            err.to_string()
                .contains("duplicate binding in keys.selector")
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(toml::from_str::<ConfigFile>("[selector]\nunknown = 1\n").is_err());
    }

    #[test]
    fn monitor_layout_percentages_must_be_in_range() {
        let axes_err = Config::try_from(
            toml::from_str::<ConfigFile>("[layout.monitor]\naxes_box_percent = 0\n").unwrap(),
        )
        .unwrap_err();
        assert!(
            axes_err
                .to_string()
                .contains("layout.monitor.axes_box_percent")
        );

        let split_err = Config::try_from(
            toml::from_str::<ConfigFile>("[layout.monitor]\njoystick_hat_joystick_percent = 100\n")
                .unwrap(),
        )
        .unwrap_err();
        assert!(
            split_err
                .to_string()
                .contains("layout.monitor.joystick_hat_joystick_percent")
        );
    }

    #[test]
    fn render_default_round_trips_from_runtime_defaults() {
        let rendered = ConfigFile::render_default().unwrap();
        let defaults = Config::default();

        let file = toml::from_str::<ConfigFile>(&rendered).unwrap();
        let resolved = Config::try_from(file).unwrap();
        assert_eq!(resolved, defaults);

        let document = toml::from_str::<Value>(&rendered).unwrap();
        assert_eq!(document["selector"]["sort"].as_str(), Some("path"));
        assert_eq!(document["monitor"]["startup_focus"].as_str(), Some("auto"));
        assert_eq!(
            document["layout"]["monitor"]["axes_box_percent"].as_integer(),
            Some(75)
        );
        assert_eq!(
            document["layout"]["monitor"]["joystick_hat_joystick_percent"].as_integer(),
            Some(70)
        );
        assert_eq!(
            document["keys"]["selector"]["move_up"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["up", "ctrl-p"]
        );
    }
}
