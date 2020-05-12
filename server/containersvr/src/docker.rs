use bollard::container::*;
use bollard::Docker;
use log::{error, warn};
use std::default::Default;

lazy_static! {
    static ref DOCKER: Option<Docker> = {
        match Docker::connect_with_local_defaults() {
            Ok(v) => Some(v),
            Err(err) => {
                error!("Open docker failed because {}", err);
                None
            }
        }
    };
}

pub async fn run(
    name: String,
    image: String,
    mm_limit: u64,
    vcpu_limit: u16,
    cpu_percent_limit: u8,
    io_speed_limit: u32,
) -> Result<(), String> {
    let docker = match *DOCKER {
        Some(v) => v,
        None => {
            return Err("can't open container".to_owned());
        }
    };

    let ctn = match docker
        .create_container(
            Some(CreateContainerOptions { name: name }),
            Config {
                hostname: Some("linux".to_owned()),
                image: Some(image),
                host_config: Some(HostConfig {
                    memory: Some(mm_limit),
                    readonly_rootfs: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
    {
        Ok(v) => v,
        Err(err) => {
            warn!("create docker container failed {}", err);
            return Err("create container error".to_owned());
        }
    };

    if let Some(v) = ctn.warnings {
        warn!("docker warning: {}", v.join(". "));
    }

    if let Err(err) = docker
        .start_container(&name, None::<StartContainerOptions<String>>)
        .await
    {
        warn!("start docker container failed {}", err);
        return Err("start container error".to_owned());
    }

    Ok(())
}

pub async fn stop(name: String) -> Result<(), String> {
    let docker = match *DOCKER {
        Some(v) => v,
        None => {
            return Err("can't open container".to_owned());
        }
    };

    if let Err(err) = docker
        .stop_container(&name, Some(StopContainerOptions { t: 0 }))
        .await
    {
        warn!("stop docker container failed {}", err);
    }

    if let Err(err) = docker
        .remove_container(
            &name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
    {
        warn!("remove docker container failed {}", err);
    }

    Ok(())
}
