use std::time::SystemTime;

use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("wait");
}

#[derive(Debug, Default)]
pub struct WaitImpl;

#[tonic::async_trait]
impl pb::wait_server::Wait for WaitImpl {
    #[tracing::instrument(err)]
    async fn wait(&self, request: Request<pb::WaitRequest>) -> Result<Response<pb::WaitResponse>, Status> {
        let pb::WaitRequest { now, wait } = request.into_inner();
        let from = Some(now.unwrap_or_else(|| SystemTime::now().into()));
        let duration = wait.unwrap_or_default().try_into().unwrap();
        tokio::time::sleep(duration).await;
        Ok(Response::new(pb::WaitResponse { from, wait }))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pb::{wait_client::WaitClient, wait_server::WaitServer};

    use super::*;

    #[tokio::test]
    async fn test_wait_basic() {
        let server = WaitServer::new(WaitImpl);
        let mut client = WaitClient::new(server);

        let (now, duration) = (SystemTime::now().into(), Duration::from_millis(500).try_into().ok());
        let request = pb::WaitRequest { now: Some(now), wait: duration };
        let response = client.wait(request).await.unwrap().into_inner();
        assert_eq!(response, pb::WaitResponse { from: Some(now), wait: duration },);
    }
}
