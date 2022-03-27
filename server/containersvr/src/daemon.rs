use containerd_client::services::v1::events_client::EventsClient;

use crate::config::Config;

async fn start_inner(config: Config) {
    let channel = containerd_client::connect(&config.url).await.unwrap();
    // let cli = EventsClient::new(channel);
}

pub fn start(config: Config) {
    tokio::spawn(start_inner(config));
}