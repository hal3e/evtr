mod cli;
mod config;
mod error;
mod evtr;
mod monitor;
mod selector;
mod ui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> error::Result<()> {
    match cli::initialize()? {
        cli::StartupAction::Run => evtr::Evtr::new()?.run().await,
        cli::StartupAction::Exit => Ok(()),
    }
}
