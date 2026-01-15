mod device;
mod error;

use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    device::Evtr::new()?.run().await
}
