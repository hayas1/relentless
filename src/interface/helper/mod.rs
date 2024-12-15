pub mod coalesce;
// `http` do not support serde https://github.com/hyperium/http/pull/631
// TODO: if do not use http::Uri for destination, move this module to `impl_http`
pub mod http_serde_priv;
pub mod is_default;
pub mod transpose;
