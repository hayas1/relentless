use std::str::FromStr;

use opentelemetry::propagation::Injector;
use tonic::{
    metadata::{MetadataKey, MetadataMap, MetadataValue},
    service::Interceptor,
};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug)]
pub struct OtelInjector<'a>(&'a mut MetadataMap);
impl<'a> Injector for OtelInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.append(
            MetadataKey::from_bytes(key.as_bytes()).unwrap_or_else(|e| todo!("{e}")),
            MetadataValue::from_str(&value).unwrap_or_else(|e| todo!("{e}")),
        );
    }
}

#[derive(Clone, Debug)]
pub struct OtelInterceptor;
impl Interceptor for OtelInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let cx = tracing::Span::current().context();
        let mut injector = OtelInjector(request.metadata_mut());
        opentelemetry::global::get_text_map_propagator(|propagator| propagator.inject_context(&cx, &mut injector));
        Ok(request)
    }
}
