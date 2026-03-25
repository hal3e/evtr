mod error;
mod evtr;
mod monitor;
mod selector;
mod ui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> error::Result<()> {
    evtr::Evtr::new()?.run().await
}
