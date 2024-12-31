use num::BigInt;
use relentless_dev_server_grpc_entity::counter_pb::counter_client::CounterClient;

// pub mod pb {
//     tonic::include_proto!("counter");
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CounterClient::connect("http://localhost:50051").await?;

    let request1 = tonic::Request::new(1);
    let response1 = client.increment(request1).await?;
    println!("RESPONSE2={:?}", response1);

    let request2 = tonic::Request::new(BigInt::from(2).into());
    let response2: BigInt = client.bincrement(request2).await?.into_inner().into();
    println!("RESPONSE3={:?}", response2);

    Ok(())
}
