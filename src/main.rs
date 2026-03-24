mod device;
mod error;
mod ui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> error::Result<()> {
    device::Evtr::new()?.run().await
}
