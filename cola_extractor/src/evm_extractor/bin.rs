use cola::{
    cola_kernel,
    processor_default,
    config::{
        parse_config,
        ColaConfig,
        Logger,
    },
};
use tokio;
use tokio_stream::StreamExt;
use std::sync::{
    Arc,
    RwLock,
};
use telegram_bot::*;
use std::env;

#[tokio::main]
async fn main() {
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let api = Api::new(token);
    let chat_id: i64 = env::var("TELEGRAM_LOG_CHAT_ID")
        .expect("TELEGRAM_LOG_CHAT_ID not set")
        .parse()
        .expect("err parsing i64");
    let chat = ChatId::new(chat_id);
    let logger = Logger::new(api,chat).await;
    let configs = parse_config("cola.yaml".to_string()).await;
    let logger = Arc::new(logger);
    let tasks: Vec<tokio::task::JoinHandle<()>> = configs
        .into_iter()
        .map(|cfg|{
            let l = logger.clone();
            tokio::spawn(async {
                let cfg = Arc::new(cfg);
                cola_kernel(cfg.clone(), l, &processor_default).await
            })
        })
        .collect();
    for h in tasks {
        h.await.unwrap();
    }
}
