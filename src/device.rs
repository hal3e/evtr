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
        let mut error_message: Option<String> = None;
        loop {
            let Some(device) =
                DeviceSelector::run(&mut self.terminal, error_message.take()).await?
            else {
                break;
            };

            if let Err(err) = DeviceMonitor::run(&mut self.terminal, device).await {
                error_message = Some(err.to_string());
            }
        }

        Ok(())
    }
}
