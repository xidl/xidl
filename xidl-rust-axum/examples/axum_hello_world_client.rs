mod imp;

use imp::HelloWorldClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HelloWorldClient::new("http://127.0.0.1:3000");
    client.sayHello("World".to_string()).await?;
    println!("request sent");
    Ok(())
}
