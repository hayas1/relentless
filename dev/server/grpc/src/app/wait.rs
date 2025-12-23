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
