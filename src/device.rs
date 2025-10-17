mod monitor;
mod selector;

pub use monitor::*;
use ratatui::DefaultTerminal;
pub use selector::*;

pub struct Evtr {
    terminal: DefaultTerminal,
}

impl Evtr {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let terminal = ratatui::init();

        Ok(Self { terminal })
    }

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.run_loop().await;
        ratatui::restore();

        result
    }

    async fn run_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
