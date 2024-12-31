use counter::counter_client::CounterClient;
use counter::CounterRequest;

pub mod counter {
    tonic::include_proto!("counter");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CounterClient::connect("http://localhost:50051").await?;
    let request = tonic::Request::new(CounterRequest { value: 1 });
    let response = client.increment(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
