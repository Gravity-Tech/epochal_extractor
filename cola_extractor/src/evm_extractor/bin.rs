use cola::{
    cola_kernel,
    processor_default,
    config::{
        parse_config,
        ColaConfig,
    },
};
use tokio;
use tokio_stream::StreamExt;
use std::sync::{
    Arc,
    RwLock,
};

#[tokio::main]
async fn main() {
    let configs = parse_config("cola.yaml".to_string()).await;
    let tasks: Vec<tokio::task::JoinHandle<()>> = configs
        .into_iter()
        .map(|cfg|{
            tokio::spawn(async {
                let cfg = Arc::new(cfg);
                cola_kernel(cfg.clone(), &processor_default).await
            })
        })
        .collect();
    for h in tasks {
        h.await.unwrap();
    }
}
