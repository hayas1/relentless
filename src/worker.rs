use crate::{
    config::{Attribute, Setting},
    error::{RelentlessError, RelentlessResult},
    outcome::{CaseOutcome, Compare, Evaluator, Status, WorkerOutcome},
};
use reqwest::{Client, Request, Response};
use tokio::task::JoinSet;
use tower::{Layer, Service};

#[derive(Debug)]
pub struct Case<LC> {
    description: Option<String>,
    target: String,
    setting: Setting,
    attr: Attribute,
    layer: Option<LC>,
}
impl<LC> Case<LC> {
    pub fn new(
        description: Option<String>,
        target: String,
        setting: Setting,
        attr: Attribute,
        layer: Option<LC>,
    ) -> Self {
        Self { description, target, setting, attr, layer }
    }

    pub fn description(&self) -> &Option<String> {
        &self.description
    }
    pub fn description_mut(&mut self) -> &mut Option<String> {
        &mut self.description
    }

    pub async fn process<LW>(self, layer: Option<LW>, setting: Setting) -> RelentlessResult<Vec<Response>>
    where
        LC: Layer<Client> + Clone + Send + 'static,
        LC::Service: Service<Request>,
        <LC as Layer<Client>>::Service: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <<LC as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        LW: Layer<Client> + Clone + Send + 'static,
        LW::Service: Service<Request>,
        <LW as Layer<Client>>::Service: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <<LW as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError: From<<<LC as Layer<Client>>::Service as Service<Request>>::Error>
            + From<<<LW as Layer<Client>>::Service as Service<Request>>::Error>,
    {
        let mut join_set = JoinSet::<RelentlessResult<Response>>::new();
        for (name, req) in self.setting.coalesce(setting).requests(&self.target)? {
            let mut client = Client::new();
            let (case_layer, worker_layer) = (self.layer.clone(), layer.clone());
            join_set.spawn(async move {
                let res = match case_layer {
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
    name: Option<String>,
    setting: Setting,
    layer: Option<LW>,
}
impl<LW> Worker<LW> {
    pub fn new(name: Option<String>, setting: Setting, layer: Option<LW>) -> Self {
        Self { name, setting, layer }
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }
    pub fn name_mut(&mut self) -> &mut Option<String> {
        &mut self.name
    }

    pub async fn assault<LC>(self, cases: Vec<Case<LC>>) -> RelentlessResult<WorkerOutcome>
    where
        LW: Layer<Client> + Clone + Send + 'static,
        LW::Service: Service<Request>,
        <LW as Layer<Client>>::Service: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LW as Layer<Client>>::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <<LW as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        LC: Layer<Client> + Clone + Send + 'static,
        LC::Service: Service<Request>,
        <LC as Layer<Client>>::Service: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Future: Send,
        <<LC as Layer<Client>>::Service as Service<Request>>::Response: Into<Response> + Send + 'static,
        <<LC as Layer<Client>>::Service as Service<Request>>::Error: Send + 'static,
        RelentlessError: From<<<LW as Layer<Client>>::Service as Service<Request>>::Error>
            + From<<<LC as Layer<Client>>::Service as Service<Request>>::Error>,
    {
        let mut outcome = Vec::new();
        for case in cases {
            let description = case.description().clone();
            let res = case.process(self.layer.clone(), self.setting.clone()).await?;
            outcome.push(if res.len() == 1 {
                Status::evaluate(description, res).await?
            } else {
                Compare::evaluate(description, res).await?
            });
        }
        Ok(WorkerOutcome::new(self.name, outcome))
    }
}
