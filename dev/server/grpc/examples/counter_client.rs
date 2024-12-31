use num::BigInt;
use relentless_dev_server_grpc::service::counter::pb::{counter_client::CounterClient, CounterRequest};

// pub mod pb {
//     tonic::include_proto!("counter");
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CounterClient::connect("http://localhost:50051").await?;

    let request = tonic::Request::new(CounterRequest { value: 1 });
    let response = client.increment(request).await?;
    println!("RESPONSE1={:?}", response);

    let request2 = tonic::Request::new(2);
    let response2 = client.incr(request2).await?;
    println!("RESPONSE2={:?}", response2);

    let request3 = tonic::Request::new(BigInt::from(3).into());
    let response3: BigInt = client.bincrement(request3).await?.into_inner().into();
    println!("RESPONSE3={:?}", response3);

    Ok(())
}
