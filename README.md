[![Workflow Status](https://github.com/hayas1/relentless/workflows/Master/badge.svg)](https://github.com/hayas1/relentless/actions?query=workflow%3A%22Master%22)
![Maintenance](https://img.shields.io/badge/maintenance-experimental-blue.svg)


<!-- cargo-rdme start -->

Relentless HTTP / GRPC comparison testing tool

## Usage
Main usage of `relentless` is comparison testing for REST API servers with `relentless-http`.
Other usages in [More details](#more-details) section.

### Install
```sh
cargo install --git https://github.com/hayas1/relentless relentless-http
```

### Prepare Config
For example, `compare.yaml`
```yaml
name: basic comparison test
destinations:
  actual: http://localhost:3000
  expect: http://localhost:3000

testcases:
  - target: /
  - target: /health
  - target: /healthz
```

#### Run API for testing
Optional: if there is no API for testing, `relentless-http-dev-server` is provided.
```sh
cargo install --git https://github.com/hayas1/relentless relentless-http-dev-server
relentless-http-dev-server
```

### Run CLI
```sh
relentless -f compare.yaml
```
```plaintext
🚀 basic comparison test 🚀
  actual🌐 http://localhost:3000/
  expect🌐 http://localhost:3000/
  ✅ /
  ✅ /health
  ✅ /healthz

💥 summery of all requests in configs 💥
  pass-rt: 3/3=100.00%    rps: 6req/22.37ms=268.23req/s
  latency: min=2.774ms mean=8.194ms p50=5.219ms p90=22.127ms p99=22.127ms max=22.127ms
```
In this case the `actual` and `expect` are the same server, so the request gets the same response and the test passes. ✅
- Each request is done **concurrently** by default.

#### More details
| | HTTP | GRPC |
| --- | --- | --- |
| Docs | [relentless-http](https://hayas1.github.io/relentless/relentless_http/) |[relentless-grpc](https://hayas1.github.io/relentless/relentless_grpc/) |

## Library Usage
### Install
Often used in dependencies for testing.
```sh
cargo add --dev --git https://github.com/hayas1/relentless relentless-http
```
```toml
[dev-dependencies]
relentless-http = { git = "https://github.com/hayas1/relentless" }
```

### Prepare Config
Same config can be used in both binary and library. See [Binary section](#prepare-config).

#### Run API for testing
Same `relentless-http-dev-server` can be used in both binary and library. See [Binary section](#run-api-for-testing).

### Run Testing
Example <https://github.com/hayas1/relentless/blob/master/relentless-http/examples/service.rs>

## Documents
<https://hayas1.github.io/relentless/relentless>

## Testing
### coverage
<https://hayas1.github.io/relentless/relentless/tarpaulin-report.html>

<!-- cargo-rdme end -->
