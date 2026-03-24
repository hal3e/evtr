mod monitor;
mod selector;

use std::mem;

use ratatui::DefaultTerminal;

use crate::error::{ErrorArea, Result};
use monitor::{DeviceMonitor, MonitorExit};
use selector::DeviceSelector;

pub struct Evtr {
    terminal: DefaultTerminal,
    state: State,
}

impl Evtr {
    pub fn new() -> Result<Self> {
        Ok(Self {
            terminal: ratatui::try_init().map_err(|err| ErrorArea::App.io("init terminal", err))?,
            state: State::new(),
        })
    }

    pub async fn run(mut self) -> Result<()> {
        loop {
            self.state = match self.state.take() {
                State::Exit => break,
                State::Select { error_message } => {
                    DeviceSelector::run(&mut self.terminal, error_message).await?
                }
                State::Monitor(device) => {
                    match DeviceMonitor::run(&mut self.terminal, *device).await {
                        Ok(MonitorExit::BackToSelector) => State::new(),
                        Ok(MonitorExit::ExitApp) => State::Exit,
                        Err(err) => State::error(err.to_string()),
                    }
                }
            };
        }

        Ok(())
    }
}

impl Drop for Evtr {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

enum State {
    Exit,
    Select { error_message: Option<String> },
    Monitor(Box<selector::DeviceInfo>),
}

impl State {
    fn new() -> Self {
        Self::Select {
            error_message: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self::Select {
            error_message: Some(message.into()),
        }
    }

    fn take(&mut self) -> Self {
        mem::replace(self, Self::new())
    }
}
