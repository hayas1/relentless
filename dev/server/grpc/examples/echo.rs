use prost_types::{Any, Value};
use relentless_dev_server_grpc_entity::echo_pb::{echo_client::EchoClient, EchoAny, EchoAnyValue};
use tonic::Request;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut echo_client = EchoClient::connect("http://localhost:50051").await?;

    let request1 = Request::new(EchoAny { value: Some(Any::from_msg(&"100".to_string()).unwrap()) });
    let response1 = echo_client.echo(request1).await?;
    println!("RESPONSE1={}", response1.into_inner().value.unwrap().to_msg::<String>().unwrap());

    let request2 = Request::new(EchoAnyValue { value: Some(Value::from(200)) });
    let response2 = echo_client.echo_value(request2).await?;
    println!("RESPONSE2={:?}", response2);

    Ok(())
}
