[![Workflow Status](https://github.com/hayas1/relentless/workflows/Master/badge.svg)](https://github.com/hayas1/relentless/actions?query=workflow%3A%22Master%22)
![Maintenance](https://img.shields.io/badge/maintenance-experimental-blue.svg)

# relentless

Relentless HTTP load testing / comparison testing tool

## Binary Usage
### Install
```sh
cargo install --git https://github.com/hayas1/relentless relentless
``````

## Prepare Config
```yaml:path/to/config.yaml
name: basic comparison test
destinations:
  actual: http://localhost:3000
  expect: http://localhost:3000

testcase:
  - target: /
  - target: /health
  - target: /healthz
```
...more examples in <https://github.com/hayas1/relentless/tree/master/examples/config>

####
If you have not api for testing, you can use `example-http-server`
```sh
cargo install --git https://github.com/hayas1/relentless example-http-server
example-http-server
```

### Run CLI
```sh
relentless -f path/to/config.yaml
🚀 basic comparison test 🚀
actual🌐 http://localhost:3000
expect🌐 http://localhost:3000
✅ /
✅ /health
✅ /healthz
```
In this case the actual and expected are the same server, so the request gets the same response and the test passes.

## Documents
<https://hayas1.github.io/relentless/relentless>

## Testing
### coverage
<https://hayas1.github.io/relentless/tarpaulin-report.html>
