pub mod counter;
pub mod echo;
pub mod health;
pub mod information;
pub mod random;
pub mod root;
pub mod wait;

use std::sync::{Arc, RwLock};

use axum::{
    error_handling::HandleErrorLayer,
    http::{StatusCode, Uri},
    response::Result,
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use tower::ServiceBuilder;
use tower_http::{
    normalize_path::{NormalizePath, NormalizePathLayer},
    trace::TraceLayer,
};

use crate::{
    app::counter::CounterState,
    error::{kind::NotFound, AppErrorDetail, Logged},
    error2::{AppError, APP_DEFAULT_ERROR_CODE},
    runner::RunCommand,
};

#[derive(Debug, Clone, Default)]
pub struct AppRouter {
    pub state: AppState,
}
impl AppRouter {
    pub fn service(self) -> NormalizePath<Router> {
        ServiceBuilder::new().layer(NormalizePathLayer::trim_trailing_slash()).service(
            self.router()
                .layer(
                    ServiceBuilder::new()
                        .layer(TraceLayer::new_for_http())
                        .layer(HandleErrorLayer::new(Self::handle_error)),
                )
                .layer(OtelInResponseLayer)
                .layer(OtelAxumLayer::default()),
        )
    }
    pub fn router(self) -> Router {
        Router::new()
            .route("/", get(root::root))
            .nest("/health", health::route_health())
            .route("/healthz", get(health::health))
            .nest("/echo", echo::route_echo())
            .nest("/information", information::route_information())
            .nest("/counter", counter::route_counter())
            .nest("/wait", wait::route_wait())
            .nest("/random", random::route_random())
            .fallback(Self::not_found)
            .with_state(self.state)
    }

    pub async fn handle_error(err: impl 'static + std::error::Error) -> AppError<String> {
        // TODO this method may not be called ?
        let response = err.to_string();
        AppError::new(err, APP_DEFAULT_ERROR_CODE, response)
    }

    pub async fn not_found(uri: Uri) -> Result<()> {
        Err(AppErrorDetail::<NotFound, _>::new(StatusCode::NOT_FOUND, Logged(""), uri.to_string()))?
    }
}
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub rc: Arc<RunCommand>,
    pub counter: Arc<RwLock<CounterState>>,
}

#[cfg(test)] // TODO do not pub(crate), use feature
pub(crate) mod tests {
    use axum::{
        body::{Body, Bytes, HttpBody},
        http::{Request, Response},
    };
    use serde::de::DeserializeOwned;
    use tower::{Service, ServiceExt};

    pub async fn body_bytes(body: Body) -> Result<Bytes, axum::Error> {
        let limit = body.size_hint().upper().unwrap_or(usize::MAX as u64);
        axum::body::to_bytes(body, limit as usize).await
    }
    pub async fn body_bytes_response(res: Response<Body>) -> Result<Response<Bytes>, axum::Error> {
        let (parts, body) = res.into_parts();
        let bytes = body_bytes(body).await?;
        Ok(Response::from_parts(parts, bytes))
    }
    pub async fn call_bytes2<S>(
        service: &mut S,
        req: Request<Body>,
    ) -> Result<Response<Bytes>, Box<dyn std::error::Error>>
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        Box<dyn std::error::Error>: From<S::Error>,
    {
        let res = service.ready().await?.call(req).await?;
        Ok(body_bytes_response(res).await?)
    }
    pub async fn call2<S, T>(service: &mut S, req: Request<Body>) -> Result<Response<T>, Box<dyn std::error::Error>>
    where
        S: Service<Request<Body>, Response = Response<Body>>,
        Box<dyn std::error::Error>: From<S::Error>,
        T: DeserializeOwned,
    {
        let res = call_bytes2(service, req).await?;
        let des = serde_json::from_slice::<T>(res.body())?;
        Ok(res.map(|_| des))
    }
}
