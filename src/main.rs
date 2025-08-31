mod device;
mod evtr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    evtr::Evtr::new()?.run().await
}
