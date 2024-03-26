use storage_kit::manager::{Role, StorageKitBuilder};
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish()
        .try_init()?;

    let _storage_kit_manager = StorageKitBuilder::new().build()?.start(Role::Bsp);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
