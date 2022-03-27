use std::collections::HashMap;
use tonic::Request;
use crate::svr::container::rpc::*;
use anyhow::*;
use containerd_client::services::v1::{container::Runtime, containers_client, Container, CreateContainerRequest};
use rand::prelude::IteratorRandom;
use tonic::transport::Channel;

pub struct Mgr<'a> {
    cfg: &'a crate::config::Config,
}

static STRMAP: &str = "1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
fn rand_id() -> String {
    let mut rng = rand::thread_rng();
    let id: String = STRMAP.chars().choose_multiple(&mut rng, 24).into_iter().collect();
    id
}

impl<'a> Mgr<'a> {
    pub fn new(cfg: &'a crate::config::Config) -> Self {
        Self { cfg }
    }
    async fn prepare_channel(&self) -> Result<containers_client::ContainersClient<Channel>> {
        let channel = containerd_client::connect(&self.cfg.url).await?;
        let cli = containerd_client::services::v1::containers_client::ContainersClient::new(channel);
        Ok(cli)
    }

    pub async fn startup(&self, req: StartupReq) -> Result<StartupRsp> {
        let cfg = self
            .cfg
            .containers
            .get(&req.config_name)
            .ok_or_else(|| anyhow!("config not found"))?;
        let mut cli = self.prepare_channel().await?;
        let id = rand_id();

        let mut labels = HashMap::new();
        labels.insert("template".to_owned(), req.config_name.clone());

        let spec = prost_types::Any {
            type_url: "".to_owned(),
            value: "".into(),
        };

        let req = CreateContainerRequest {
            container: Some(Container {
                id: id.clone(),
                labels: labels,
                image: cfg.image.clone(),
                runtime: Some(Runtime {
                    name: cfg.runtime.clone(),
                    options: None,
                }),
                spec: Some(spec),
                snapshot_key: "".to_string(),
                snapshotter: cfg.snapshotter.clone(),
                created_at: None,
                updated_at: None,
                extensions: HashMap::new(),
            }),
        };
        let req = containerd_client::with_namespace!(req, "judgement");
        log::info!("try create container id {}", id);
        let rsp = cli.create(req).await?;
        let rsp = rsp.into_inner().container;
        if let Some(_) = rsp {
            return Ok(StartupRsp { id: id });
        }
        anyhow::bail!("fail to create container")
    }
}
