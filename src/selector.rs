mod commands;
mod devices;
mod discovery;
mod state;
mod view;

use std::{
    io,
    path::{Path, PathBuf},
};

use crossterm::event::{Event, EventStream as TermEventStream, KeyEvent, KeyEventKind};
use evdev::Device;
use futures::StreamExt;
use ratatui::DefaultTerminal;

use self::{
    commands::command_for,
    devices::DeviceCatalog,
    discovery::discover_devices,
    state::{SelectorState, SelectorTransition},
    view::render_selector,
};
use crate::{
    config,
    error::{ErrorArea, Result},
    evtr::State,
};

pub(crate) use self::devices::device_label;

#[derive(Debug)]
pub(crate) struct DeviceInfo {
    pub(crate) device: Device,
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

pub(crate) struct DeviceSelector {
    devices: DeviceCatalog,
    state: SelectorState,
}

impl DeviceSelector {
    fn new(error_message: Option<String>) -> Self {
        let (devices, discovery_error) = Self::load_devices();
        let selector_config = config::selector();

        Self {
            state: SelectorState::new(
                devices.labels(),
                error_message.or(discovery_error),
                selector_config.page_scroll_size,
            ),
            devices,
        }
    }

    fn open_device(path: &Path) -> io::Result<DeviceInfo> {
        let device = Device::open(path)?;
        let name = device.name().unwrap_or("Unknown Device").to_string();

        Ok(DeviceInfo {
            device,
            name,
            path: path.to_path_buf(),
        })
    }

    fn load_devices() -> (DeviceCatalog, Option<String>) {
        DeviceCatalog::from_discovery(discover_devices(Self::open_device), config::selector().sort)
    }

    fn refresh_devices(&mut self) {
        let (devices, error_message) = Self::load_devices();
        self.state.apply_discovery(devices.labels(), error_message);
        self.devices = devices;
    }

    pub(crate) async fn run(
        terminal: &mut DefaultTerminal,
        error_message: Option<String>,
    ) -> Result<State> {
        let mut selector = Self::new(error_message);
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| {
                    render_selector(
                        &selector.state,
                        &selector.devices,
                        frame.area(),
                        frame.buffer_mut(),
                    );
                })
                .map_err(|err| ErrorArea::Selector.io("selector draw", err))?;

            match term_events.next().await {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    if let Some(state) = selector.handle_key_press(key) {
                        return Ok(state);
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    return Err(ErrorArea::Selector.io("terminal event stream", err));
                }
                None => {
                    return Err(ErrorArea::Selector.stream_ended("terminal event stream"));
                }
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> Option<State> {
        let mode = self.state.mode();
        let transition = self
            .state
            .reduce(command_for(key, mode), self.devices.labels());
        self.handle_transition(transition)
    }

    fn handle_transition(&mut self, transition: SelectorTransition) -> Option<State> {
        match transition {
            SelectorTransition::Stay => None,
            SelectorTransition::Exit => Some(State::Exit),
            SelectorTransition::RefreshDevices => {
                self.refresh_devices();
                None
            }
            SelectorTransition::OpenSelection => self
                .devices
                .take_selected(self.state.selected_device_index())
                .map(|device| State::Monitor(Box::new(device))),
        }
    }
}
