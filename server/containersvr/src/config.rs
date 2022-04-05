use anyhow::{Error, Result};
use log::info;
use micro_service::cfg::MicroServiceConfig;
use petgraph::{
    algo::is_cyclic_directed,
    visit::{depth_first_search, DfsEvent, IntoNodeIdentifiers},
    Directed,
};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap};

trait Merger<T> {
    fn merge(&mut self, base: &Self);
    fn unwrap_to(&mut self, target: &mut T) -> Result<()>;
}

#[derive(Deserialize, Debug, Clone)]
struct InnerLimitConfig {
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub io: Option<String>,
}

macro_rules! unwrap_field {
    ($self:tt, $target: tt, $name: tt, $default: expr) => {
        if let Some(self_item) = &$self.$name {
            $target.$name = self_item.clone();
        } else {
            $target.$name = $default;
        }
    };
}

macro_rules! unwrap_struct_field {
    ($self:tt, $target: tt, $name: tt) => {
        if let Some(self_item) = &mut $self.$name {
            self_item.unwrap_to(&mut $target.$name)?
        }
    };
}

macro_rules! merge_field {
    ($self:tt, $base: tt, $name: tt) => {
        if $self.$name.is_none() {
            if let Some(base_item) = &$base.$name {
                $self.$name = Some(base_item.clone())
            }
        }
    };
}

macro_rules! merge_struct_field {
    ($self:tt, $base: tt, $name: tt) => {
        if let Some(self_item) = &mut $self.$name {
            if let Some(base_item) = &$base.$name {
                self_item.merge(&base_item)
            }
        } else {
            $self.$name = $base.$name.clone()
        }
    };
}

impl Merger<LimitConfig> for InnerLimitConfig {
    fn merge(&mut self, base: &Self) {
        merge_field!(self, base, cpu);
        merge_field!(self, base, memory);
        merge_field!(self, base, io);
    }
    fn unwrap_to(&mut self, target: &mut LimitConfig) -> Result<()> {
        unwrap_field!(self, target, cpu, "50m".to_owned());
        unwrap_field!(self, target, memory, "50M".to_owned());
        unwrap_field!(self, target, io, "100".to_owned());
        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
struct InnerContainerConfig {
    namespace: Option<String>,

    image: Option<String>,

    limit: Option<InnerLimitConfig>,

    extends: Option<String>,

    runtime: Option<String>,

    snapshotter: Option<String>,
}

impl Merger<ContainerConfig> for InnerContainerConfig {
    fn merge(&mut self, base: &Self) {
        merge_field!(self, base, namespace);
        merge_field!(self, base, image);
        merge_struct_field!(self, base, limit);
        merge_field!(self, base, runtime);
        merge_field!(self, base, snapshotter);
    }
    fn unwrap_to(&mut self, target: &mut ContainerConfig) -> Result<()> {
        unwrap_field!(self, target, namespace, "default".to_owned());
        unwrap_field!(self, target, image, "docker.io/alpine:latest".to_owned());
        unwrap_struct_field!(self, target, limit);
        unwrap_field!(self, target, runtime, "io.containerd.runc.v2".to_owned());
        unwrap_field!(self, target, snapshotter, "native".to_owned());
        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
struct InnerConfig {
    #[serde(rename = "containers")]
    pub container_template: HashMap<String, RefCell<InnerContainerConfig>>,
    pub url: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct LimitConfig {
    pub cpu: String,
    pub memory: String,
    pub io: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ContainerConfig {
    pub namespace: String,

    pub image: String,

    pub limit: LimitConfig,

    pub runtime: String,

    pub snapshotter: String,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub containers: HashMap<String, ContainerConfig>,
    pub url: String,
}

pub async fn read(config: &MicroServiceConfig) -> Result<Config, Error> {
    let config_file = config
        .comm
        .container_config
        .as_ref()
        .ok_or_else(|| Error::msg("config file no set"))?;
    let content = tokio::fs::read_to_string(config_file).await?;
    log::info!("read container config {}", content);

    let cfg: InnerConfig = serde_yaml::from_str(&content)?;
    let mut graph = petgraph::Graph::<(), (), Directed>::new();
    let mut node_map = HashMap::new();
    let mut node_name_map = HashMap::new();
    let mut edge_map = HashMap::new();

    for name in cfg.container_template.keys() {
        let node = graph.add_node(());
        node_map.insert(name, node);
        node_name_map.insert(node, name);
    }

    for (name, container) in cfg.container_template.iter() {
        if let Some(extends) = &container.borrow().extends {
            let b = match node_map.get(extends) {
                Some(idx) => *idx,
                None => continue,
            };
            let a = *node_map.get(name).unwrap();
            let edge = graph.add_edge(a, b, ());
            edge_map.insert(name, edge);
        }
    }

    if is_cyclic_directed(&graph) {
        anyhow::bail!("cyclic extends detected");
    }
    depth_first_search(&graph, graph.node_identifiers(), |event| {
        if let DfsEvent::Discover(node, _) = event {
            let node_name = match node_name_map.get(&node) {
                Some(name) => *name,
                None => return,
            };
            if let Some(container) = cfg.container_template.get(node_name) {
                let mut container = container.borrow_mut();
                if let Some(extends) = &container.extends {
                    // get container template
                    if let Some(base_container) = cfg.container_template.get(extends) {
                        // fill extend
                        let base_container = base_container.borrow();
                        info!("{} merge base {}", node_name, extends);
                        container.merge(&base_container)
                    }
                }
            }
        }
    });

    let mut new_cfg = Config {
        containers: HashMap::new(),
        url: cfg.url,
    };
    for (name, container) in cfg.container_template.iter() {
        let mut final_container = ContainerConfig::default();
        container.borrow_mut().unwrap_to(&mut final_container)?;
        new_cfg.containers.insert(name.clone(), final_container);
    }
    log::info!("final config {:?}", new_cfg);

    Ok(new_cfg)
}
