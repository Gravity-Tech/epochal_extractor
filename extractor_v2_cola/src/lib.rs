use tokio::time::{
    delay_for, 
    Duration
};
pub mod config;

use tokio_stream::StreamExt;
use database;

use web3::transports::Http;
use web3::types::*;
use web3::ethabi::{
    Topic,
    TopicFilter,
};
use tokio::prelude::*;

use tokio_stream::Stream;
use serde;
use serde::{
    Serialize,
    Deserialize,
};
use uuid::Uuid;

pub enum DbAction {
    Insert,
    Delete,
}
use config::{
    ColaConfig,
    parse_config,
};
use base64;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use crate::config::ChainType;

pub fn processor_default(mut log: Log) -> Vec<u8> {
    let mut acc = Vec::new();
    for t in log.topics {
        acc.extend_from_slice(t.as_bytes())
    }
    acc.append(log.data.0.as_mut());
    acc
}

pub async fn cola_kernel(
    config: Arc<ColaConfig>,
    processor: &'static (dyn Fn(Log)->Vec<u8> + Sync),
) -> ! { 
    loop {
        let num = database::load_num(
                config.bubble_id, 
                &config.connection.get().unwrap(),
            )
            .unwrap();
        let prev_block = BlockNumber::Number(num.into());
        let current_block_num = config
                    .web3_instance
                    .eth()
                    .block_number()
                    .await
                    .unwrap();
        let current_block = BlockNumber::Number(current_block_num);
        let mut topics = TopicFilter::default();
        topics.topic0 = match  config.chain_name {
            ChainType::FTM => Topic::default(),
            _ => Topic::from(config.event_topic),
        };
        let filter = FilterBuilder::default()
                    .from_block(prev_block) 
                    .to_block(current_block)
                    .address(vec![config.emitter_address])
                    .topic_filter(topics)
                    .build();
        let result: Vec<web3::types::Log> = config
                    .web3_instance
                    .eth()
                    .logs(filter)
                    .await
                    .unwrap();
        println!("{} starting from block {} to block {} ...",config.bubble_name,num,current_block_num);
        match config.chain_name {
            ChainType::FTM => {
                let data:Vec<Uuid> = tokio_stream::iter(result)
                    .map(|log| {
                        let s = log.data.0.as_slice();
                        Uuid::from_slice(s).unwrap()
                    })
                    .collect()
                    .await;
                database::delete(
                    (current_block_num.as_u64()+1) as i64, 
                    config.bubble_id, 
                    data,
                   &config.connection.get().unwrap(),
                )
                .unwrap();
            }
            _ => {
                let data:Vec<(Uuid,String)> = tokio_stream::iter(result)
                    .map(|log| processor(log))
                    .map(|mut bytes| {
                        let mut b = Vec::new();
                        let id = Uuid::new_v4();
                        let chain = format!("{:?}",config.chain_name);
                        b.extend_from_slice(id.as_bytes());
                        b.extend_from_slice(chain.as_bytes());
                        b.extend_from_slice(config.emitter_address.as_bytes());
                        b.append(bytes.as_mut());
                        (id,base64::encode(b))
                    })
                    .collect()
                    .await;
                println!("data portions {}",data.len());
                database::push(
                    (current_block_num.as_u64()+1) as i64, 
                    config.bubble_id, 
                    data,
                   &config.connection.get().unwrap(),
                )
                .unwrap();
            }
        }
        delay_for(Duration::from_secs((60) as u64)).await;
    }
}
