use relentless::testcase::Testcase;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let testcase = Testcase::import("./examples/assault.yaml").unwrap();
    let result = testcase.run().await.unwrap();
    Ok(result)
}

// use relentless::testcase::{Http, Protocol, Testcase};
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     let testcase = Testcase {
//         name: None,
//         host: std::collections::HashMap::from([(
//             "target".to_string(),
//             "localhost:3000".to_string(),
//         )]),
//         protocol: Protocol::Http(vec![Http {
//             method: "GET".to_string(),
//             pathname: "/".to_string(),
//         }]),
//     };
//     println!("{}", serde_yaml::to_string(&testcase).unwrap());
//     Ok(())
// }
