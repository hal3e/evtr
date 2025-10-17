mod device;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    device::Evtr::new()?.run().await
}
