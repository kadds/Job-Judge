use std::collections::HashSet;
use tonic::{Code, Request};

use containerd_client::{
    services::v1::{images_client::ImagesClient, GetImageRequest},
    with_namespace,
};
use log::{error, info};
use tonic::transport::Channel;

use crate::config::Config;

async fn check_images(channel: Channel, config: Config) {
    let mut image_map = HashSet::<(&str, &str)>::new();
    for container in config.containers.values() {
        image_map.insert((&container.image, &container.namespace));
    }
    info!("check images {}", image_map.len());
    let mut cli = ImagesClient::new(channel);

    for (image, ns) in image_map {
        let req = GetImageRequest {
            name: image.to_owned(),
        };

        let exist = match cli.get(with_namespace!(req, ns)).await {
            Ok(v) => v.into_inner().image.is_some(),
            Err(err) => {
                if err.code() == Code::NotFound {
                    false
                } else {
                    error!("get image {} in {} {}", image, ns, err);
                    continue;
                }
            }
        };
        if !exist {
            info!("image {} in {} not exist", image, ns)
        }
    }
    info!("check images ok")
}

async fn start_inner(config: Config) {
    let channel = containerd_client::connect(&config.url).await.unwrap();
    // let cli = EventsClient::new(channel);
    tokio::spawn(check_images(channel, config.clone()));
}

pub fn start(config: Config) {
    tokio::spawn(start_inner(config));
}
