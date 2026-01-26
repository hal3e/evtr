mod monitor;
mod popup;
mod selector;

pub mod evtr {
    use super::monitor::DeviceMonitor;
    use super::selector::DeviceSelector;
    use crate::error::Result;

    pub async fn run() -> Result<()> {
        let mut terminal = ratatui::init();

        let result = async {
            let mut error_msg = None;
            loop {
                let Some(device) = DeviceSelector::run(&mut terminal, error_msg.take()).await?
                else {
                    break;
                };

                if let Err(err) = DeviceMonitor::run(&mut terminal, device).await {
                    error_msg = Some(err.to_string());
                }
            }

            Ok(())
        }
        .await;

        ratatui::restore();

        result
    }
}
