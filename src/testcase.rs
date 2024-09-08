use reqwest::{Request, Url};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::Path, str::FromStr};
use tower::Service;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Testcase {
    pub name: Option<String>,
    pub host: HashMap<String, String>,

    #[serde(flatten)]
    pub protocol: Protocol,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Http(Vec<Http>),
    Grpc(Vec<Grpc>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Http {
    pub method: String,
    pub pathname: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grpc {
    // TODO
}

impl Testcase {
    pub fn import<P: AsRef<Path>>(path: P) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_reader(File::open(path).unwrap())
    }

    pub async fn run(&self) -> Result<(), ()> {
        match self.protocol {
            Protocol::Http(ref http) => {
                let requests = http
                    .iter()
                    .map(|h| {
                        self.host.iter().map(|(_, host)| {
                            let method = reqwest::Method::from_str(&h.method).unwrap();
                            let url = Url::parse(host).unwrap().join(&h.pathname).unwrap();
                            Request::new(method.clone(), url)
                        })
                    })
                    .flatten(); // TODO do not flatten (for compare test)
                for r in requests {
                    let client = reqwest::Client::new();
                    let _ = self.request(client, r).await;
                }
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    pub async fn request<S: Service<Request>>(
        &self,
        mut service: S,
        request: Request,
    ) -> Result<S::Response, S::Error> {
        service.call(request).await
    }
}
