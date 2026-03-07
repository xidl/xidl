use clap::Parser;
use tokio::io::split;
use tokio::net::TcpStream;
use xidlc_examples::city_jsonrpc::{SmartCityRpcApi, SmartCityRpcApiClient};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4001")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let stream = TcpStream::connect(&args.addr).await?;
    stream.set_nodelay(true)?;
    let (reader, writer) = split(stream);

    let client = SmartCityRpcApiClient::new(reader, writer);

    client.mark_paid("inv-1001".to_string()).await?;
    client.heartbeat().await?;
    client
        .report_trip(
            "order-1".to_string(),
            "rider-2".to_string(),
            "done".to_string(),
        )
        .await?;

    let region = client.get_attribute_region().await?;
    println!("region: {region}");

    client
        .set_attribute_firmware_channel("canary".to_string())
        .await?;
    let channel = client.get_attribute_firmware_channel().await?;
    println!("firmware_channel: {channel}");

    Ok(())
}
