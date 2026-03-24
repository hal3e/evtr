mod commands;
mod discovery;
mod state;
mod view;

use std::{io, path::Path};

use crossterm::event::{Event, EventStream as TermEventStream, KeyEvent, KeyEventKind};
use evdev::Device;
use futures::StreamExt;
use ratatui::DefaultTerminal;

use self::{
    commands::command_for,
    discovery::discover_devices,
    state::{SelectorState, SelectorTransition},
    view::render_selector,
};
use super::State;
use crate::error::{ErrorArea, Result};

#[derive(Debug)]
pub struct DeviceInfo {
    pub device: Device,
    pub identifier: String,
}

pub struct DeviceSelector {
    state: SelectorState,
}

impl DeviceSelector {
    fn new(error_message: Option<String>) -> Self {
        Self {
            state: SelectorState::new(discover_devices(Self::open_device), error_message),
        }
    }

    fn open_device(path: &Path) -> io::Result<DeviceInfo> {
        let device = Device::open(path)?;
        let name = device.name().unwrap_or("Unknown Device").to_string();
        let path = path.to_string_lossy().to_string();

        Ok(DeviceInfo {
            device,
            identifier: format!("{name} ({path})"),
        })
    }

    fn refresh_devices(&mut self) {
        self.state
            .apply_discovery(discover_devices(Self::open_device));
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
        error_message: Option<String>,
    ) -> Result<State> {
        let mut selector = Self::new(error_message);
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| {
                    render_selector(&selector.state, frame.area(), frame.buffer_mut());
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
        let transition = self.state.reduce(command_for(key, mode));
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
                .state
                .take_selected_device()
                .map(|device| State::Monitor(Box::new(device))),
        }
    }
}
