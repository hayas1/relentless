use std::sync::Arc;

use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

#[tokio::main]
async fn main() {
    // let uri = http::Uri::from_static("http://127.0.0.1:3000/");
    let uri = http::Uri::from_static("https://google.com/");

    // let stream = TcpStream::connect(uri.authority().unwrap().as_str()).await.unwrap();
    // let io = TokioIo::new(stream);
    // let rt = hyper_util::rt::TokioExecutor::new();
    // let (mut sender, conn) = hyper::client::conn::http2::handshake(rt, io).await.unwrap();
    // tokio::spawn(conn);

    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let mut config = rustls::ClientConfig::builder().with_root_certificates(root_cert_store).with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];

    let connector = TlsConnector::from(Arc::new(config));
    let tcp_stream = TcpStream::connect(format!("{}:443", uri.authority().unwrap().as_str())).await.unwrap();
    let tls_domain = rustls_pki_types::ServerName::try_from(uri.authority().unwrap().as_str())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid dnsname"))
        .unwrap()
        .to_owned();

    let stream = connector.connect(tls_domain, tcp_stream).await.unwrap();
    let io = TokioIo::new(stream);
    let rt = hyper_util::rt::tokio::TokioExecutor::new();

    let (mut sender, conn) = hyper::client::conn::http2::handshake(rt, io).await.unwrap();
    tokio::spawn(conn);

    let req = http::Request::builder()
        .uri(uri)
        .version(hyper::Version::HTTP_2)
        .body(http_body_util::Empty::<bytes::Bytes>::new())
        .unwrap();

    sender.ready().await.unwrap();
    let response = sender.send_request(req).await.unwrap();

    println!("Response: {:?}", response);
}
