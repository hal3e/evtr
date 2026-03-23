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
    commands::{SelectorEffect, apply_command, command_for},
    discovery::discover_devices,
    state::SelectorState,
    view::render_selector,
};
use super::State;
use crate::error::{Error, ErrorArea, Result};

const PAGE_SCROLL_SIZE: usize = 10;

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
                .map_err(|err| Error::io(ErrorArea::Selector, "selector draw", err))?;

            match term_events.next().await {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    if let Some(state) = selector.handle_key_press(key) {
                        return Ok(state);
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    return Err(Error::io(ErrorArea::Selector, "terminal event stream", err));
                }
                None => {
                    return Err(Error::stream_ended(
                        ErrorArea::Selector,
                        "terminal event stream",
                    ));
                }
            }
        }
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> Option<State> {
        let mode = self.state.mode();
        if mode.is_browsing() {
            self.state.clear_error_message();
        }
        let effect = apply_command(&mut self.state, command_for(key, mode));
        self.handle_effect(effect)
    }

    fn handle_effect(&mut self, effect: Option<SelectorEffect>) -> Option<State> {
        match effect {
            Some(SelectorEffect::Exit) => Some(State::Exit),
            Some(SelectorEffect::RefreshDevices) => {
                self.refresh_devices();
                None
            }
            Some(SelectorEffect::OpenSelection) => self
                .state
                .take_selected_device()
                .map(|device| State::Monitor(Box::new(device))),
            None => None,
        }
    }
}
