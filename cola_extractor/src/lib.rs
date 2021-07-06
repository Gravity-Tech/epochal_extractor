use tokio::time::{
    delay_for, 
    Duration
};
pub mod config;

use tokio_stream::StreamExt;
use database;

use web3::types::*;
use web3::ethabi::{
    Topic,
    TopicFilter,
};

use uuid::Uuid;

pub enum DbAction {
    Insert,
    Delete,
}
use config::{
    EventConfig,
    ColaConfig,
};
use base64;
use std::sync::Arc;

use crate::config::ChainType;

pub fn processor_default(mut log: Log) -> (Vec<u8>,u64) {
    let mut acc = Vec::new();
    acc.extend_from_slice(&[log.topics.len() as u8]);
    for t in log.topics {
        acc.extend_from_slice(t.as_bytes())
    }
    acc.append(log.data.0.as_mut());
    (acc,log.block_number.unwrap().as_u64())
}

pub async fn proc_topic(
    prev_block: BlockNumber,
    current_block: BlockNumber,
    current_topic: &EventConfig,
    config: Arc<ColaConfig>,
) -> Vec<Log> {
        let mut topics = TopicFilter::default();
        topics.topic0 = Topic::from(current_topic.event_topic);
        let filter = FilterBuilder::default()
                    .from_block(prev_block) 
                    .to_block(current_block)
                    .address(vec![config.emitter_address])
                    .topic_filter(topics)
                    .build();
        config
                    .web3_instance
                    .eth()
                    .logs(filter)
                    .await
                    .expect(&format!("can't request logs  for {} ",config.bubble_name))

}

pub async fn cola_kernel(
    config: Arc<ColaConfig>,
    processor: &'static (dyn Fn(Log)->(Vec<u8>,u64) + Sync),
) -> ! { 
    loop {
        println!("starting new iteration in {}\n time (utc): {}",
            config.bubble_name,chrono::Utc::now());
        let num = database::load_num(
                config.bubble_id, 
                &config.connection.get()
                    .expect(&format!("error connecting to db in {}",
                        config.bubble_name))
            )
            .expect(&format!("no id for {} in database",config.bubble_name));
        let prev_block = BlockNumber::Number(num.into());
        let num: U64 = num.into();
        let current_block_num = config
                    .web3_instance
                    .eth()
                    .block_number()
                    .await
                    .expect(&format!("can't get current block in {}",
                            config.bubble_name));
        let current_block_num = current_block_num - 10;
        let current_block_num = current_block_num
            .min(num + config.max_block_range);
        let current_block = BlockNumber::Number(current_block_num);

        println!("going from block {} to {} in {}",num,
            current_block_num,config.bubble_name);

        let mut result: Vec<(Uuid,String,i64,i32,i32)> = Vec::new();
        for ev in config.events_data.iter() {
            let r = proc_topic(
                prev_block, 
                current_block,
                ev,
                config.clone(), 
            ).await;
            match ev.db_action {
                config::DBAction::Delete => {
                    let data:Vec<Uuid> = tokio_stream::iter(r)
                        .map(|log| {
                            let s = log.data.0.as_slice();
                            Uuid::from_slice(&s[32..32+16])
                                .expect(&format!("error slicing UUID in {}\n data: {:?}",
                                        config.bubble_name,&s))
                        })
                        .collect()
                        .await;
                    database::delete(
                        (current_block_num.as_u64()+1) as i64, 
                        config.bubble_id, 
                        data,
                       &config.connection
                            .get()
                            .expect(&format!("error delete from db in {} ",
                                    config.bubble_name)),
                    )
                    .expect(&format!("error deleting from db in {}",
                            config.bubble_name));
                }
                config::DBAction::Insert => {
                    let mut data:Vec<(Uuid,String,i64,i32,i32)> = tokio_stream::iter(r)
                        .map(|log| processor(log))
                        .map(|bytes| {
                            let (mut bytes,block) = bytes;
                            let mut b = Vec::new();
                            let id = Uuid::new_v4();
                            let chain = format!("{:?}",config.chain_name);
                            b.extend_from_slice(id.as_bytes());
                            b.extend_from_slice(chain.as_bytes());
                            b.extend_from_slice(config.emitter_address.as_bytes());
                            b.append(bytes.as_mut());
                            (id,base64::encode(b),block as i64,ev.priority,ev.port)
                        })
                        .collect()
                        .await;
                    result.append(&mut data);
                }
            }
        }
        database::push(
            (current_block_num.as_u64()+1) as i64, 
            config.bubble_id, 
            result,
            &config.connection
                    .get()
                    .expect(&format!("error getting connection to db in {}",
                            config.bubble_name))
        )
        .expect(&format!("error adding to db in {}",
                            config.bubble_name));
        delay_for(Duration::from_secs((30) as u64)).await;
    }
}
