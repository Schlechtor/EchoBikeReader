mod bike_data;
mod central;
mod peripheral;

use bike_data::BikeData;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    println!("Using adapter {} ({})", adapter.name(), adapter.address().await?);

    let (tx, rx) = tokio::sync::watch::channel(BikeData::default());

    tokio::try_join!(
        central::run(tx),
        async {
            peripheral::run(&adapter, rx).await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
        },
    )?;

    Ok(())
}
