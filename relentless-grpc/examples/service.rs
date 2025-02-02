use std::process::ExitCode;

#[tokio::main]
#[cfg(all(feature = "yaml", feature = "console-report"))]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    use relentless::interface::command::{Assault, Relentless};
    use relentless_grpc::{client::GrpcClient, command::GrpcAssault};
    use relentless_grpc_dev_server::service::{
        counter::{pb::counter_server::CounterServer, CounterImpl},
        echo::{pb::echo_server::EchoServer, EchoImpl},
        greeter::{pb::greeter_server::GreeterServer, GreeterImpl},
    };
    use tonic::transport::Server;

    let assault = GrpcAssault::new(Relentless {
        file: vec![
            concat!(env!("CARGO_MANIFEST_DIR"), "/examples/config/assault.yaml").into(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/examples/config/compare.yaml").into(),
        ],
        ..Default::default()
    });
    let (configs, errors) = assault.configs();
    errors.into_iter().for_each(|err| eprintln!("{}", err));

    let destinations = assault.all_destinations(&configs);
    let routes = Server::builder()
        .add_service(GreeterServer::new(GreeterImpl))
        .add_service(CounterServer::new(CounterImpl::default()))
        .add_service(EchoServer::new(EchoImpl))
        .into_service();
    let service = GrpcClient::from_services(&destinations.into_iter().map(|d| (d, routes.clone())).collect()).await?;

    let report = assault.assault_with(configs, service).await?;

    assault.report_with(&report, std::io::stdout())?;
    Ok(assault.exit_code(&report))
}

#[cfg(not(all(feature = "yaml", feature = "console-report")))]
fn main() -> ExitCode {
    eprintln!("Insufficient features for this example");
    ExitCode::FAILURE
}
