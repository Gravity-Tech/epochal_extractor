use async_trait::async_trait;
use serde_json::Value;
use base64;
use hex;
use evm_event_scanner::EventScannerAdapter;
use chrono;
use poller_core::{Base64Bytes, PollerRuntime};
use std::sync::Arc;
use std::ops::Index;
use std::ops::Div;

#[macro_use]
extern crate derive_builder;

#[derive(Default,Builder)]
pub struct EthScan {
    api_key: String,
    address_from: String,
    topic: String,
    chain_url: String,
    metadata: String,
}

#[derive(Clone)]
struct Data {
    api_key: String,
    address_from: String,
    topic: String,
    chain_url: String,
    metadata: String,
}

#[async_trait]
impl EventScannerAdapter<Data> for EthScan {

    async fn get_move_data(&self) -> Arc<Data> {
        Arc::new(Data {
            api_key: self.api_key.clone(),
            address_from: self.address_from.clone(),
            topic: self.topic.clone(),
            chain_url: self.chain_url.clone(),
            metadata: self.metadata.clone(),
        })
    }
    async fn log_string(&self, number: u128) -> String {
        format!("{}/api?module=logs&action=getLogs&fromBlock={}&toBlock={}&address={}&topic0={}&apikey={}",
            self.chain_url,
            number,
            number,
            self.address_from,
            self.topic,
            self.api_key)
    }
    async fn height_string(&self, number: u128) -> String {
        format!("{}/api?module=block&action=getblocknobytime&timestamp={}&closest=before&apikey={}",
            self.chain_url,
            chrono::Utc::now().timestamp(),
            self.api_key)
    }

    async fn proc_portion(
        my_data: Arc<Data>,
        data: Value,
        thread_num: &str,
        heigth: &str,
    ) -> Base64Bytes {
        let data = data["data"]
            .as_str()
            .unwrap();
        let contract_addr = data[26..66].to_string();
        let sender = data[90..130].to_string();
        let amount = data[130..194].to_string();
        let id = data[194..258].to_string();
        let meta = data[258..322].to_string();

        println!("row {} | contract adr : {}",thread_num,contract_addr);
        println!("row {} | sender       : {}",thread_num,sender);
        println!("row {} | amount       : {}",thread_num,amount);
        println!("row {} | id           : {}",thread_num,id);
        println!("row {} | meta         : {}",thread_num,meta);

        let mut sender = hex::decode(sender).unwrap();
        let mut contract_addr = hex::decode(contract_addr).unwrap();
        let mut amount = hex::decode(amount).unwrap();
        let mut id  = hex::decode(id).unwrap();
        let mut meta  = hex::decode(meta).unwrap();

        fn uint_32(v: &mut Vec<u8>) -> Vec<u8> {
            let mut r = vec![0u8;32-v.len()];
            r.append(v);
            r
        }

        fn address(v: &mut Vec<u8>) -> Vec<u8> {
            let mut r = vec![0u8;20-v.len()];
            r.append(v);
            r
        }

        let mut aggregated_data = Vec::new();
        aggregated_data.append(address(contract_addr.as_mut()).as_mut());
        aggregated_data.append(address(sender.as_mut()).as_mut());
        aggregated_data.append(uint_32(amount.as_mut()).as_mut());
        aggregated_data.append(uint_32(id.as_mut()).as_mut());
        aggregated_data.append(uint_32(meta.as_mut()).as_mut());

        base64::encode(&aggregated_data[..])
    }
}

#[async_trait]
impl PollerRuntime for EthScan {
    async fn new_portion(
        &self, 
        num: u128,
    ) -> Result<Vec<Base64Bytes>, Box<dyn std::error::Error>> {
        self.process_epoch(num).await
    }
}

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    use poller_core;
    use std::env;
    use dotenv;

    dotenv::dotenv().ok();
    let scanner = EthScanBuilder::default()
        .chain_url(env::var("CHAIN_URL").expect("Missing chain url env var"))
        .api_key(env::var("API_KEY").expect("Missing api key env var"))
        .address_from(env::var("TOKEN_ADDRESS").expect("Missing IERC20 token addr"))
        .topic(env::var("TRANSFER_EVENT_TOPIC").expect("Missing event transfer topic"))
        .metadata(env::var("METADATA")
            .unwrap_or("0000000000000000000000000000000000000000000000000000000000000000"
                .to_string()))
        .build()
        .unwrap();
    println!("created scanner");
    let poll_id = env::var("POLLER_ID")
            .expect("Missing poller id env var")
            .parse::<usize>()
            .expect("eror converting poller id") as i32;
    println!("starting poller");
    poller_core::run(scanner, poll_id, poller_core::DatabaseAction::Insert).await
}
