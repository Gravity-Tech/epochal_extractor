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
}

#[derive(Clone)]
pub struct ColaConfig {
    pub chain_name: ChainType,
    pub event_topic: Vec<H256>,
    pub web3_instance: web3::Web3<Http>,
    pub emitter_address: Address,
    pub connection: Arc<DbPool>,
    pub bubble_id: i32,
    pub bubble_name: String,
    pub priority: i32,
    pub max_block_range: U64,
    pub port: i32,
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
                        _ => panic!("no chain name {} presented",s),
                    }
                None => panic!("can't find chain name in {}",name),
            };
            let event_topic: Vec<H256> = match cfg["event_topic"].as_sequence() {
                Some(s) => s
                        .into_iter()
                        .map(|v| {
                            v
                                .as_str()
                                .unwrap()
                                .parse::<H256>()
                                .unwrap()
                        })
                        .collect(),
                None => panic!("can't find topic in {}",name),
            };
            let web3_instance = match cfg["rpc_url"].as_str() {
                Some(s) => {
                    let http = web3::transports::Http::new(s)
                        .expect("err creating http");
                    web3::Web3::new(http)
                }
                None => panic!("can't find rpc_url in {}",name),
            };
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
            let priority = match cfg["priority"].as_i64() {
                Some(s) => s as i32,
                None => 0,
            };
            let port = match cfg["port"].as_i64() {
                Some(s) => s as i32,
                None => 0,
            };
            ColaConfig {
                bubble_id: id,
                bubble_name: name.to_string(),
                chain_name: chain_name, 
                event_topic: event_topic,
                web3_instance: web3_instance,
                emitter_address: emitter_address,
                connection: pool.clone(),
                priority: priority,
                max_block_range: max_block_range,
                port: port,
            } 
        }).collect().await
}
