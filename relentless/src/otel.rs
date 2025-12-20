use clap::Args;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(Args))]
pub struct Otel;
impl Otel {
    pub fn provider(&self) -> crate::Result<SdkTracerProvider> {
        let resource = self.resource();
        let exporter = self.exporter()?;
        let provider = self.tracer_provider(resource, exporter);
        Ok(provider)
    }
    pub fn init_tracing(&self, provider: &SdkTracerProvider) -> crate::Result<()> {
        let tracer = provider.tracer(env!("CARGO_PKG_NAME"));
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
            .with(telemetry);
        tracing::subscriber::set_global_default(subscriber).map_err(crate::Error::boxed)
    }
    pub fn set_global_propagator(&self) {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    }

    pub fn resource(&self) -> Resource {
        Resource::builder()
            .with_attributes(vec![
                opentelemetry::KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
                opentelemetry::KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            ])
            .build()
    }
    pub fn exporter(&self) -> crate::Result<SpanExporter> {
        SpanExporter::builder().with_tonic().build().map_err(crate::Error::boxed)
    }
    pub fn tracer_provider(&self, resource: Resource, exporter: SpanExporter) -> SdkTracerProvider {
        SdkTracerProvider::builder().with_resource(resource).with_batch_exporter(exporter).build()
    }
}
