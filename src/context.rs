use std::{collections::HashMap, path::Path, process::ExitCode};

use http_body::Body;
use tower::Service;

use crate::{
    command::{Assault, Cmd, SubCommands},
    config::Config,
    error::{RelentlessError, RelentlessResult},
    outcome::Outcome,
    service::{BytesBody, DefaultHttpClient, FromBodyStructure},
    worker::Control,
    Relentless,
};

// TODO ContextBuilder
#[derive(Debug, Clone)]
pub struct Context<C = (), F = ()> {
    pub cmd: C,
    pub config: F,
}
impl<C, F> Context<C, F> {
    /// TODO document
    pub fn new(cmd: C, config: F) -> Self {
        Self { cmd, config }
    }

    /// TODO document
    pub fn cmd(self, cmd: C) -> Self {
        Self { cmd, ..self }
    }

    /// TODO document
    pub fn config(self, config: F) -> Self {
        Self { config, ..self }
    }
}
impl<C> Context<C, ()> {
    /// TODO document
    pub fn from_cmd(cmd: C) -> Self {
        Self { cmd, config: () }
    }
}
impl<F> Context<Cmd, F> {
    /// TODO document
    pub fn no_color(mut self, no_color: bool) -> Self {
        self.cmd.no_color = no_color;
        self
    }

    /// TODO document
    pub fn apply_no_color(&self) {
        console::set_colors_enabled(!self.cmd.no_color);
    }

    /// TODO document
    pub async fn relentless(self) -> RelentlessResult<ExitCode> {
        self.apply_no_color();
        let Self { cmd, .. } = self;
        let relentless = if let Some(Assault { file, configs_dir, .. }) = cmd.assault() {
            if let Some(dir) = configs_dir {
                Relentless::read_dir(cmd.assault().unwrap(), dir).await?
            } else {
                Relentless::read_paths(cmd.assault().unwrap(), file).await?
            }
        } else {
            todo!("Assault config not found")
        };
        let outcome = relentless.assault(cmd.assault().unwrap()).await?;
        outcome.report(cmd.assault().unwrap())?;
        Ok(outcome.exit_code(false))
    }
}
impl Context<Cmd, Vec<Config>> {
    /// TODO document
    pub fn assault_with_config(config: Vec<Config>) -> Self {
        let cmd = Cmd { subcommand: SubCommands::Assault(Default::default()), no_color: false };
        Self::new(cmd, config)
    }

    /// TODO document
    pub fn assault_with_config_paths<I: IntoIterator<Item = P>, P: AsRef<Path>>(paths: I) -> RelentlessResult<Self> {
        let cmd = Cmd { subcommand: SubCommands::Assault(Default::default()), no_color: false };
        let config = paths.into_iter().map(Config::read).collect::<RelentlessResult<Vec<_>>>()?;
        Ok(Self::new(cmd, config))
    }

    /// TODO document
    pub fn assault_with_config_dir<P: AsRef<Path>>(config_dir: P) -> RelentlessResult<Self> {
        let cmd = Cmd { subcommand: SubCommands::Assault(Default::default()), no_color: false };
        Ok(Self::new(cmd, Config::read_dir(config_dir)?))
    }

    /// TODO document
    pub async fn relentless_with_default_http_client(
        self,
        // ) -> RelentlessResult<Control<DefaultHttpClient<ReqB, BytesBody>, ReqB, BytesBody>>
    ) -> RelentlessResult<Outcome> {
        self.apply_no_color();
        let Self { cmd, config } = self;
        let relentless =
            Control::<DefaultHttpClient<BytesBody, BytesBody>, BytesBody, BytesBody>::with_default_http_client(
                cmd.assault().unwrap(),
                config,
            )
            .await?;

        let outcome = relentless.assault(cmd.assault().unwrap()).await?;
        outcome.report(cmd.assault().unwrap())?;
        // Ok(outcome.exit_code(false))
        Ok(outcome)
    }

    /// TODO document
    pub async fn relentless_with_service<S, ReqB, ResB>(
        self,
        services: Vec<HashMap<String, S>>,
        // ) -> RelentlessResult<Control<S, ReqB, ResB>>
    ) -> RelentlessResult<Outcome>
    where
        ReqB: Body + FromBodyStructure + Send + 'static,
        ReqB::Data: Send + 'static,
        ReqB::Error: std::error::Error + Sync + Send + 'static,
        ResB: Body + Send + 'static,
        ResB::Data: Send + 'static,
        ResB::Error: std::error::Error + Sync + Send + 'static,
        S: Service<http::Request<ReqB>, Response = http::Response<ResB>> + Send + Sync + 'static,
        RelentlessError: From<S::Error>,
    {
        self.apply_no_color();
        let Self { cmd, config } = self;
        let relentless = Control::with_service(config, services)?;

        let outcome = relentless.assault(cmd.assault().unwrap()).await?;
        // outcome.report(cmd.assault().unwrap())?;
        // Ok(outcome.exit_code(false))
        Ok(outcome)
    }
}
