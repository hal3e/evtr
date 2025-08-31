use crate::device::DeviceMonitor;
use crate::device::DeviceSelector;
use ratatui::DefaultTerminal;

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
            // Select a device
            let device = DeviceSelector::select_device(&mut self.terminal).await?;

            // Monitor the selected device
            let should_continue = DeviceMonitor::monitor_device(&mut self.terminal, device).await?;

            if !should_continue {
                break; // User wants to quit the application
            }
            // Otherwise loop back to device selection
        }

        Ok(())
    }
}
