mod monitor;
mod selector;

use crate::error::Result;
pub use monitor::DeviceMonitor;
use ratatui::DefaultTerminal;
pub use selector::{DeviceInfo, DeviceSelector};

pub struct Evtr {
    terminal: DefaultTerminal,
}

impl Evtr {
    pub fn new() -> Result<Self> {
        let terminal = ratatui::init();

        Ok(Self { terminal })
    }

    pub async fn run(mut self) -> Result<()> {
        let result = self.run_loop().await;
        ratatui::restore();

        result
    }

    async fn run_loop(&mut self) -> Result<()> {
        loop {
            let Some(device) = DeviceSelector::run(&mut self.terminal).await? else {
                break;
            };

            let should_continue = DeviceMonitor::run(&mut self.terminal, device).await?;

            if !should_continue {
                break;
            }
        }

        Ok(())
    }
}
