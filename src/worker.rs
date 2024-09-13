use crate::{
    config::Setting,
    error::{RelentlessError, RelentlessResult},
};
use reqwest::{Client, Request, Response};
use tokio::task::JoinSet;
use tower::{Layer, Service};

#[derive(Debug)]
pub struct Unit<LU> {
    pub description: Option<String>,
    pub target: String,
    pub layer: Option<LU>,
    pub setting: Option<Setting>,
}
impl<LU> Unit<LU> {
    pub fn new(
        description: Option<String>,
        target: String,
        layer: Option<LU>,
        setting: Option<Setting>,
    ) -> Self {
        Self {
            description,
            target,
            layer,
            setting,
        }
    }

    pub async fn process<LW>(
        self,
        layer: Option<LW>,
        setting: Option<Setting>,
    ) -> RelentlessResult<Vec<Response>>
    where
        LU: Layer<Client> + Clone + Send + 'static,
        LU::Service: Service<Request>,
        <LU as Layer<Client>>::Service: Send,
        <<LU as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LU as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<LU as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        LW: Layer<Client> + Clone + Send + 'static,
        LW::Service: Service<Request>,
        <LW as Layer<Client>>::Service: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<LW as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError: From<<<LU as Layer<Client>>::Service as Service<Request>>::Error>
            + From<<<LW as Layer<Client>>::Service as Service<Request>>::Error>,
    {
        let mut join_set = JoinSet::<RelentlessResult<Response>>::new();
        for (name, req) in self
            .setting
            .unwrap_or_default()
            .coalesce(setting.unwrap_or_default())
            .requests(&self.target)?
        {
            let mut client = Client::new();
            let (unit_layer, worker_layer) = (self.layer.clone(), layer.clone());
            join_set.spawn(async move {
                let res = match unit_layer {
                    Some(layer) => layer.layer(client).call(req).await?.into(),
                    None => match worker_layer {
                        Some(layer) => layer.layer(client).call(req).await?.into(),
                        None => client.call(req).await?,
                    },
                };
                Ok(res)
            });
        }

        let mut response = Vec::new();
        while let Some(res) = join_set.join_next().await {
            response.push(res??);
        }
        Ok(response)
    }
}

pub struct Worker<LW> {
    pub name: Option<String>,
    pub layer: Option<LW>,
    pub setting: Option<Setting>,
}
impl<LW> Worker<LW> {
    pub fn new(name: Option<String>, layer: Option<LW>, setting: Option<Setting>) -> Self {
        Self {
            name,
            layer,
            setting,
        }
    }

    pub async fn assault<LC>(self, units: Vec<Unit<LC>>) -> RelentlessResult<()>
    where
        LW: Layer<Client> + Clone + Send + 'static,
        LW::Service: Service<Request>,
        <LW as Layer<Client>>::Service: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<LW as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        LC: Layer<Client> + Clone + Send + 'static,
        LC::Service: Service<Request>,
        <LC as Layer<Client>>::Service: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Response:
            Into<Response> + Send + 'static,
        <<LC as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError: From<<<LW as Layer<Client>>::Service as Service<Request>>::Error>
            + From<<<LC as Layer<Client>>::Service as Service<Request>>::Error>,
    {
        for unit in units {
            let _res = unit
                .process(self.layer.clone(), self.setting.clone())
                .await?;
        }
        Ok(())
    }
}
