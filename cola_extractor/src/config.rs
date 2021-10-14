use serde_yaml::{
    self,
    Value,
    Mapping,
};
use tokio_stream::StreamExt;
use web3::transports::Http;
use tokio_stream::Stream;
use web3::types::*;
use web3::ethabi::{
    Topic,
};

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
use std::sync::Arc;

#[derive(Debug,Clone)]
pub enum ChainType {
    ETH,
    FTM,
    BNB,
    PLG,
    HEC,
    DAI,
    AVA,
    SOL,
}

#[derive(Debug,Clone)]
pub enum DBAction {
    Insert,
    Delete,
}

#[derive(Clone)]
pub struct EventConfig {
    pub event_topic: H256,
    pub topics1: Option<Vec<H256>>,
    pub priority: i32,
    pub port: i32,
    pub db_action: DBAction,
}

#[derive(Clone)]
pub struct ColaConfig {
    pub chain_name: ChainType,
    pub events_data: Vec<EventConfig>,
    pub web3_instance: web3::Web3<Http>,
    pub delay: u64,
    pub retry_delay: u64,
    pub error_limit: u64,
    pub emitter_address: Address,
    pub connection: Arc<DbPool>,
    pub bubble_id: i32,
    pub bubble_name: String,
    pub max_block_range: U64,
}

use telegram_bot::*;
pub struct Logger {
    api: Api,
    chat_id: ChatId,
}

impl Logger {
    pub async fn new(api: Api, chat_id: ChatId) -> Self {
        Logger {
            api,
            chat_id,
        }
    }

    pub async fn err(
        &self,
        message: &str,
    ) {
        self.api.send(self.chat_id.text(message)).await.unwrap();
    }
}

pub async fn parse_config(filename: String) -> Vec<ColaConfig> {
    let data = std::fs::read_to_string(filename).expect("failed to read cola.yaml");
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<PgConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let pool = Arc::new(pool);
    let v: Mapping = serde_yaml::from_str(&data).expect("error parsing yaml file");
    tokio_stream::iter(v)
        .map(|v|{
            let name = v.0.as_str().expect("can't find bubble name");
            let cfg = v.1;
            let chain_name = match cfg["chain_name"].as_str() {
                Some(s) =>
                    match s {
                        "ETH" => ChainType::ETH,
                        "FTM" => ChainType::FTM,
                        "BNB" => ChainType::BNB,
                        "PLG" => ChainType::PLG,
                        "HEC" => ChainType::HEC,
                        "DAI" => ChainType::DAI,
                        "AVA" => ChainType::AVA,
                        "SOL" => ChainType::SOL,
                        _ => panic!("wrong chain name {} presented",s),
                    }
                None => panic!("can't find chain name in {}",name),
            };
            let events_data = match cfg["events_data"].as_sequence() {
                Some(s) => s.into_iter()
                    .map(|v|{
                        EventConfig {
                            event_topic: v["event_topic"]
                                        .as_str()
                                        .expect(&format!(
                                                "missing event topic in {}",
                                                name
                                        ))
                                        .parse()
                                        .expect("error persing event topic"),
                            topics1: v["topics_1"]
                                        .as_sequence()
                                        .map(|v|{
                                            let v: Vec<H256> = v
                                            .into_iter()
                                            .map(|ev|{
                                                let ev: H256 = ev
                                                    .as_str()
                                                    .expect("err getting str topic1")
                                                    .parse()
                                                    .expect("err parsing topic1");
                                                ev
                                            })
                                            .collect();
                                            Some(v)
                                        })
                                        .unwrap_or(None),

                            priority: v["priority"].as_i64().unwrap_or(0) as i32,
                            port: v["port"].as_i64().unwrap_or(8088) as i32,
                            db_action: match v["db_action"].as_str() {
                                Some(s) =>
                                    match s {
                                        "Insert" => DBAction::Insert,
                                        "Delete" => DBAction::Delete,
                                        _ => panic!("no valid db action in {} presented",s),
                                    }
                                None => DBAction::Insert,
                            },
                        }
                    })
                    .collect(),
                None => panic!("can't find topic in {}",name),
            };
            let web3_instance = cfg["rpc_url"]
                    .as_str()
                    .map(|s|{
                        let http = web3::transports::Http::new(s)
                            .expect("err creating http");
                        web3::Web3::new(http)
                    }) 
                    .expect("can't find rpc_url");
            let delay = cfg["delay"].as_u64().unwrap_or(30); 
            let retry_delay = cfg["retry_delay"].as_u64().unwrap_or(0); 
            let error_limit = cfg["request_error_limit"].as_u64().unwrap_or(50); 
            let emitter_address = match cfg["emitter_address"].as_str() {
                Some(s) => s.parse::<Address>().expect("error parsing emmiter address"),
                None => panic!("can't find emitter_address in {}",name),
            };
            let id = match cfg["id"].as_i64() {
                Some(s) => s as i32,
                None => panic!("can't find bubble id in {}",name),
            };
            let max_block_range = match cfg["max_block_range"].as_u64() {
                Some(s) => s.into(),
                None => panic!("can't find bubble id in {}",name),
            };
            ColaConfig {
                bubble_id: id,
                bubble_name: name.to_string(),
                events_data: events_data,
                chain_name: chain_name, 
                web3_instance: web3_instance,
                emitter_address: emitter_address,
                connection: pool.clone(),
                max_block_range: max_block_range,
                delay: delay,
                retry_delay: retry_delay,
                error_limit: error_limit,
            } 
        }).collect().await
}
