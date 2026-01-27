mod device;
mod error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> error::Result<()> {
    device::Evtr::new().run().await
}
