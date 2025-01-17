// ##### TODO record to sqlite or duckdb with measure.rs #####
// **DEPRECATED** record request / response is experimental feature

use std::{
    fs::File,
    future::Future,
    io::Write,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use tower::{Layer, Service};

use crate::error::IntoResult;

pub trait IoRecord<R> {
    type Error;
    fn extension(&self, r: &R) -> &'static str;
    fn record<W: std::io::Write + Send>(&self, w: &mut W, r: R)
        -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn record_raw<W: std::io::Write + Send>(
        &self,
        w: &mut W,
        r: R,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
pub trait CollectClone<R> {
    type Error;
    fn collect_clone(&self, r: R) -> impl Future<Output = Result<(R, R), Self::Error>> + Send;
}
pub trait RequestIoRecord<R> {
    fn record_dir(&self, r: &R) -> PathBuf;
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct RecordLayer<R> {
    path: Option<PathBuf>,
    recorder: R,
}
impl<R> RecordLayer<R> {
    pub fn new(path: Option<PathBuf>, recorder: R) -> Self {
        Self { path, recorder }
    }
}
impl<S, R> Layer<S> for RecordLayer<R>
where
    R: Clone,
{
    type Service = RecordService<S, R>;
    fn layer(&self, inner: S) -> Self::Service {
        let Self { path, recorder } = self.clone();
        RecordService { path, recorder, inner }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct RecordService<S, R> {
    path: Option<PathBuf>,
    recorder: R,
    inner: S,
}
impl<S, R, Req, Res> Service<Req> for RecordService<S, R>
where
    R: IoRecord<Req>
        + CollectClone<Req>
        + RequestIoRecord<Req>
        + IoRecord<Res>
        + CollectClone<Res>
        + Clone
        + Send
        + 'static,
    <R as IoRecord<Req>>::Error: std::error::Error + Send + Sync + 'static,
    <R as CollectClone<Req>>::Error: std::error::Error + Send + Sync + 'static,
    <R as IoRecord<Res>>::Error: std::error::Error + Send + Sync + 'static,
    <R as CollectClone<Res>>::Error: std::error::Error + Send + Sync + 'static,
    Req: Send + 'static,
    Res: Send + 'static,
    S: Service<Req, Response = Res> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).box_err()
    }

    fn call(&mut self, request: Req) -> Self::Future {
        let paths = (|p: Option<&PathBuf>| {
            // TODO path will be uri ... (if implement template, it will not be in path)
            // TODO timestamp or repeated number
            // TODO join path (absolute) https://github.com/rust-lang/rust/issues/16507
            let dir = p?.join(self.recorder.record_dir(&request));
            std::fs::create_dir_all(&dir).ok()?;
            writeln!(File::create(p?.join(".gitignore")).ok()?, "*").ok()?; // TODO hardcode...
            Some(((dir.join("raw_request"), dir.join("request")), (dir.join("raw_response"), dir.join("response"))))
        })(self.path.as_ref());

        if let Some(((path_raw_req, path_req), (path_raw_res, path_res))) = paths {
            let mut cloned_inner = self.inner.clone();
            let recorder = self.recorder.clone();
            Box::pin(async move {
                let (request, recordable_raw_req) = recorder.collect_clone(request).await.box_err()?;
                recorder
                    .record_raw(&mut File::create(path_raw_req.with_extension("txt")).box_err()?, recordable_raw_req)
                    .await
                    .box_err()?;
                let (request, recordable_req) = recorder.collect_clone(request).await.box_err()?;
                let req_record_extension = recorder.extension(&recordable_req);
                recorder
                    .record(&mut File::create(path_req.with_extension(req_record_extension)).box_err()?, recordable_req)
                    .await
                    .box_err()?;

                let response = cloned_inner.call(request).await.box_err()?;

                let (response, recordable_raw_res) = recorder.collect_clone(response).await.box_err()?;
                recorder
                    .record_raw(&mut File::create(path_raw_res.with_extension("txt")).box_err()?, recordable_raw_res)
                    .await
                    .box_err()?;
                let (response, recordable_res) = recorder.collect_clone(response).await.box_err()?;
                let res_record_extension = recorder.extension(&recordable_res);
                recorder
                    .record(&mut File::create(path_res.with_extension(res_record_extension)).box_err()?, recordable_res)
                    .await
                    .box_err()?;

                Ok(response)
            })
        } else {
            let fut = self.inner.call(request);
            Box::pin(async move { fut.await.box_err() })
        }
    }
}
