use tower::Layer;

#[trait_variant::make(Send)]
pub trait Contract<S>: Sized + Layer<S> {
    type ReqSource;
    type Request;
    type ResSink;
    type Response;
    type Error;

    async fn new(service: S, request: Self::ReqSource) -> Result<Self, Self::Error>;
}
