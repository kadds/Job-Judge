use crate::{config::ContainerConfig, svr::container::rpc::*};
use anyhow::*;
use core::result::Result::Ok;
use oci_spec::image::{ImageConfiguration, ImageIndex, ImageManifest, Platform, PlatformBuilder};
use oci_spec::runtime::*;
use rand::prelude::IteratorRandom;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tonic::transport::Channel;
use tonic::Request;

type CRIContainerClient = containerd_client::services::v1::containers_client::ContainersClient<Channel>;
type CRITaskClient = containerd_client::services::v1::tasks_client::TasksClient<Channel>;
type CRIImageClient = containerd_client::services::v1::images_client::ImagesClient<Channel>;
type CRISnapshotClient = containerd_client::services::v1::snapshots::snapshots_client::SnapshotsClient<Channel>;
type CRIContentClient = containerd_client::services::v1::content_client::ContentClient<Channel>;

type CRIContainer = containerd_client::services::v1::Container;
type CRIRuntime = containerd_client::services::v1::container::Runtime;
type CRICreateContainerReq = containerd_client::services::v1::CreateContainerRequest;
type CRICreateTaskReq = containerd_client::services::v1::CreateTaskRequest;
type CRIStartTaskReq = containerd_client::services::v1::StartRequest;
type CRIPrepareSnapshotReq = containerd_client::services::v1::snapshots::PrepareSnapshotRequest;
type CRIGetImageReq = containerd_client::services::v1::GetImageRequest;
type CRIReadContentReq = containerd_client::services::v1::ReadContentRequest;

use containerd_client::with_namespace;

pub struct Mgr<'a> {
    cfg: &'a crate::config::Config,
    ch: Option<Channel>,
}

static STRMAP: &str = "1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
fn rand_id() -> String {
    let mut rng = rand::thread_rng();
    let id: String = STRMAP.chars().choose_multiple(&mut rng, 24).into_iter().collect();
    id
}

fn load_oci_spec(cfg: &ContainerConfig, id: &str, req: &StartupReq) -> Result<(Vec<u8>, String)> {
    let args = req.args.clone();
    let resource_devices = vec![
        LinuxDeviceCgroupBuilder::default()
            .typ(LinuxDeviceType::C)
            .major(1)
            .minor(3)
            .allow(true)
            .access("rwm")
            .build()?,
        LinuxDeviceCgroupBuilder::default()
            .typ(LinuxDeviceType::C)
            .major(1)
            .minor(8)
            .allow(true)
            .access("rwm")
            .build()?,
        LinuxDeviceCgroupBuilder::default()
            .typ(LinuxDeviceType::C)
            .major(1)
            .minor(7)
            .allow(true)
            .access("rwm")
            .build()?,
        LinuxDeviceCgroupBuilder::default()
            .typ(LinuxDeviceType::C)
            .major(1)
            .minor(5)
            .allow(true)
            .access("rwm")
            .build()?,
        LinuxDeviceCgroupBuilder::default()
            .typ(LinuxDeviceType::C)
            .major(1)
            .minor(9)
            .allow(true)
            .access("rwm")
            .build()?,
    ];
    let mut mounts = get_default_mounts();
    mounts.push(
        MountBuilder::default()
            .destination("/run")
            .source("tmpfs")
            .typ("tmpfs")
            .options(vec!["nosuid".into(), "strictatime".into(), "mode=755".into(), "size=65536k".into()])
            .build()?,
    );

    let spec = SpecBuilder::default()
        .root(RootBuilder::default().readonly(false).build()?)
        .linux(
            LinuxBuilder::default()
                .cgroups_path(format!("/{}/{}", cfg.namespace, id))
                .resources(LinuxResourcesBuilder::default().devices(resource_devices).build()?)
                .build()?,
        )
        .mounts(mounts)
        .process(ProcessBuilder::default().args(args).build()?)
        .build()?;
    let value = serde_json::to_vec(&spec)?;
    Ok((value, "types.containerd.io/opencontainers/runtime-spec/1/Spec".into()))
}

impl<'a> Mgr<'a> {
    pub fn new(cfg: &'a crate::config::Config) -> Self {
        Self { cfg, ch: None }
    }

    async fn connect(&mut self) -> Result<()> {
        let channel = containerd_client::connect(&self.cfg.url).await?;
        self.ch = Some(channel);
        Ok(())
    }

    fn channel(&self) -> Channel {
        self.ch.as_ref().expect("connect before get client").clone()
    }

    async fn read_content(&self, namespace: &str, digest: String) -> Result<Vec<u8>> {
        let read_content_req = CRIReadContentReq {
            digest,
            offset: 0,
            size: 0,
        };
        let mut cli = CRIContentClient::new(self.channel());
        let mut rsp = cli.read(with_namespace!(read_content_req, namespace)).await?.into_inner();
        while let Some(rsp) = rsp.message().await? {
            let d = rsp.data;
            log::info!("size {} off {}", d.len(), rsp.offset);
            return Ok(d);
        }
        anyhow::bail!("fail to read content")
    }

    async fn search_image_digest(&self, image: &str, namespace: &str) -> Result<String> {
        // Step 1. get image digest
        let get_image_req = CRIGetImageReq { name: image.into() };
        let mut cli = CRIImageClient::new(self.channel());
        let rsp = cli.get(with_namespace!(get_image_req, namespace)).await?.into_inner();
        let image_digest = if let Some(image) = rsp.image {
            image
                .target
                .ok_or_else(|| anyhow::anyhow!("fail to get image digest"))
                .map(|v| v.digest)?
        } else {
            anyhow::bail!("fail to get image info")
        };

        log::info!("get image {} info {}", image, image_digest);

        // Step 2. get image content manifests
        let config_index: ImageIndex = serde_json::from_slice(&self.read_content(namespace, image_digest).await?)?;

        let manifest_item = config_index
            .manifests()
            .into_iter()
            .filter(|file| match file.platform() {
                Some(v) => v.architecture().to_string() == "amd64" && v.os().to_string() == "linux",
                None => false,
            })
            .next()
            .ok_or_else(|| anyhow::anyhow!("fail to load specific manifest"))?;

        // Step 3. load image manifest from specific platform filter
        let layer_item: ImageManifest =
            serde_json::from_slice(&self.read_content(namespace, manifest_item.digest().to_owned()).await?)?;

        // Step 3. load image configuration (layer) from image
        let config: ImageConfiguration =
            serde_json::from_slice(&self.read_content(namespace, layer_item.config().digest().to_owned()).await?)?;

        // Step 4. calculate finalize digest
        let mut iter = config.rootfs().diff_ids().into_iter();
        let mut prev_digest: String = iter.next().map_or_else(|| String::new(), |v| v.clone());
        while let Some(v) = iter.next() {
            let mut hasher = Sha256::new();
            hasher.update(prev_digest);
            hasher.update(" ");
            hasher.update(v);
            let sha = hex::encode(hasher.finalize());
            prev_digest = format!("sha256:{}", sha)
        }
        log::info!("load {} diff digest {}", image, prev_digest);
        Ok(prev_digest)
    }

    pub async fn startup(&mut self, req: StartupReq) -> Result<StartupRsp> {
        let cfg = self
            .cfg
            .containers
            .get(&req.config_name)
            .ok_or_else(|| anyhow!("config not found"))?;

        let _ = self.connect().await?;
        let id = rand_id();

        // Step 1. load image's snapshot digest
        let snapshot_base = self.search_image_digest(&cfg.image, &cfg.namespace).await?;

        // Step 2. create container
        self.create_container(req, cfg, &id).await?;

        // Step 4. get mounts from snapshot service
        let mounts = self.load_mounts(&cfg, &id, snapshot_base).await?;

        // Step 5. create task
        let create_task_req = CRICreateTaskReq {
            container_id: id.clone(),
            terminal: false,
            rootfs: mounts,
            checkpoint: None,
            options: None,
            stdin: "".into(),
            stdout: "".into(),
            stderr: "".into(),
        };

        let mut cli = CRITaskClient::new(self.channel());
        let rsp = cli.create(with_namespace!(create_task_req, cfg.namespace)).await?.into_inner();
        if rsp.pid == 0 {
            anyhow::bail!("fail to create task")
        }
        log::info!("create task id {}", id);

        // Step 6. start task
        let start_task_req = CRIStartTaskReq {
            container_id: id.clone(),
            exec_id: "".into(),
        };
        let rsp = cli.start(with_namespace!(start_task_req, cfg.namespace)).await?.into_inner();
        if rsp.pid == 0 {
            anyhow::bail!("fail to start task")
        }
        log::info!("start task id {} {}", id, rsp.pid);

        return Ok(StartupRsp { id: id });
    }

    async fn load_mounts(
        &self,
        cfg: &ContainerConfig,
        id: &str,
        snapshot_base: String,
    ) -> Result<Vec<containerd_client::types::Mount>> {
        let view_snapshot_req = CRIPrepareSnapshotReq {
            snapshotter: cfg.snapshotter.clone(),
            key: id.to_owned(),
            parent: snapshot_base,
            labels: HashMap::new(),
        };
        let mut cli = CRISnapshotClient::new(self.channel());
        let rsp = cli
            .prepare(with_namespace!(view_snapshot_req, cfg.namespace))
            .await?
            .into_inner();

        log::info!("get mounts {} {}", id, rsp.mounts.len());
        Ok(rsp.mounts)
    }

    async fn create_container(&self, req: StartupReq, cfg: &ContainerConfig, id: &String) -> Result<()> {
        let mut labels = HashMap::new();
        labels.insert("io.github/job-judge".into(), req.config_name.clone());
        labels.insert("io.containerd.image.config.stop-signal".into(), "SIGTERM".into());
        let (spec, type_url) = load_oci_spec(cfg, id, &req)?;
        let spec = prost_types::Any {
            type_url: type_url,
            value: spec,
        };
        let create_req = CRICreateContainerReq {
            container: Some(CRIContainer {
                id: id.clone(),
                labels: labels,
                image: cfg.image.clone(),
                runtime: Some(CRIRuntime {
                    name: cfg.runtime.clone(),
                    options: Some(prost_types::Any {
                        type_url: "containerd.runc.v1.Options".into(),
                        value: Vec::new(),
                    }),
                }),
                spec: Some(spec),
                snapshot_key: id.clone(),
                snapshotter: cfg.snapshotter.clone(),
                created_at: None,
                updated_at: None,
                extensions: HashMap::new(),
            }),
        };
        let mut cli = CRIContainerClient::new(self.channel());
        let rsp = cli.create(with_namespace!(create_req, cfg.namespace)).await?.into_inner();
        if rsp.container.is_none() {
            anyhow::bail!("fail to create container")
        }
        log::info!("create container {}", id);
        Ok(())
    }
}
