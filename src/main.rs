mod device;
mod error;

use crate::device::{DeviceMonitor, DeviceSelector};

#[tokio::main]
async fn main() -> error::Result<()> {
    let mut terminal = ratatui::init();

    let mut error_msg = None;
    loop {
        let Some(device) = DeviceSelector::run(&mut terminal, error_msg.take()).await? else {
            break;
        };

        if let Err(err) = DeviceMonitor::run(&mut terminal, device).await {
            error_msg = Some(err.to_string());
        }
    }

    ratatui::restore();

    Ok(())
}
