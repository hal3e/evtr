use crate::device::evtr;

mod device;
mod error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> error::Result<()> {
    evtr::run().await
}
