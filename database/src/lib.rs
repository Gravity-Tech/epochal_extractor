pub mod config;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde;
use diesel::prelude::*;
use std::sync::Arc;
pub mod schema;
use crate::schema::extracted_data;
use bigdecimal::BigDecimal;
use crate::schema::poller_states;
use diesel::result::Error;
use uuid::Uuid;
use diesel::PgConnection;
use base64;
use diesel::r2d2::ConnectionManager;
use r2d2;
pub type ConnPool = r2d2::Pool<ConnectionManager<PgConnection>>;
use config::{
    ChainInfo,
    Info,
};

pub fn establish_connection(database_url: &str) -> ConnPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}

pub mod lazy_extractors {
    use super::*;
    use ethcontract::web3;
    use web3::types::*;
    use std::str::FromStr;

    table! {
        relay_swaps (id) {
            id -> Uuid,
            base64bytes -> Nullable<Varchar>,
            origin_chain -> Varchar,
            origin_txn_id -> Varchar,
            destination_txn_id -> Nullable<Varchar>,
            destination_chain -> Nullable<Varchar>,
            creation -> Timestamp,
            amount -> Nullable<Numeric>,
            origin_address -> Nullable<Varchar>,
            destination_address -> Nullable<Varchar>,
            finalize -> Nullable<Timestamp>,
            status -> Varchar,
        }
    }

    #[derive(Clone,Queryable)]
    pub struct RelaySwaps {
        pub id: Uuid,
        pub base64bytes: Option<String>,
        pub origin_chain: String,
        pub origin_txn_id: String,
        pub destination_txn_id: Option<String>,
        pub destination_chain: Option<String>,
        pub creation: chrono::NaiveDateTime,
        pub amount: Option<BigDecimal>,
        pub origin_address: Option<String>,
        pub destination_address: Option<String>,
        pub finalize: Option<chrono::NaiveDateTime>,
        pub status: String, 
    }

    pub fn processor_default(log: &mut Log) -> Vec<u8> {
        let mut acc = Vec::new();
        acc.extend_from_slice(&[log.topics.len() as u8]);
        for t in log.topics.iter() {
            acc.extend_from_slice(t.as_bytes())
        }
        acc.append(log.data.0.as_mut());
        acc
    }

    pub async fn proc_unparsed(
        event_topic0: H256,
        conn: &PgConnection,
        config: Arc<ChainInfo>,
    ) -> Result<(),Error> {
        let rows = relay_swaps::table
            .filter(relay_swaps::status.eq("created"))
            .or_filter(relay_swaps::status.eq("delivered"))
            .load::<RelaySwaps>(conn)
            .unwrap();
        for row in rows {
            let uuid = row.id;
            let txn  = &row.origin_txn_id[..];
            let chain_short = &row.origin_chain[..];

            let rpc_url = match chain_short {
                "ETH" => "",
                "FTM" => "",
                "BNB" => "",
                "PLG" => "",
                "HEC" => "",
                "DAI" => "",
                "AVA" => "",
                _ => panic!("wrong chain name presented"),
            };
            let http = web3::transports::Http::new(rpc_url)
                .expect("err creating http");
            let web3 = web3::Web3::new(http);
            match &row.status[..] {
                "created" => proc_new(row,conn,config.clone()).await.unwrap(),
                "delivered" => delete(row,conn,config.clone()).await.unwrap(),
                _ => panic!(),
                
            }
        }
        Ok(())
    }

    pub async fn proc_new(
        row: RelaySwaps,
        conn: &PgConnection,
        config: Arc<ChainInfo>,
    ) -> Result<(),Error> {
        let txn_id = H256::from_str(row.origin_txn_id.as_ref()).unwrap();
        let info = config
            .info
            .get(row.origin_chain.as_ref())
            .unwrap();

        let http = web3::transports::Http::new(&info.rpc_url)
                .expect("err creating http");
        let web3 = web3::Web3::new(http);
        let logs = web3
            .eth()
            .transaction_receipt(txn_id)
            .await
            .unwrap()
            .unwrap()
            .logs;
        let mut logs = logs
            .into_iter()
            .filter(|v| check_log_suitability(v,&row.origin_chain,info))
            .collect::<Vec<Log>>();
        let log = logs
            .get_mut(0)
            .unwrap();

        let mut bytes = processor_default(log);
        let mut b = Vec::new();
        let id = Uuid::new_v4();
        b.extend_from_slice(id.as_bytes());
        b.extend_from_slice(row.origin_chain.as_bytes());
        b.extend_from_slice(log.address.as_bytes());
        b.append(bytes.as_mut());
        let uuid = log.data.0.as_slice();
        let uuid = Uuid::from_slice(&uuid[32..32+16])
            .expect("error getting uuid in event data");

        let _updates = diesel::update(relay_swaps::table)
            .filter(relay_swaps::id.eq(uuid))
            .set((
                    relay_swaps::base64bytes.eq(base64::encode(b)),
                    relay_swaps::status.eq("encoded")))
            .execute(conn)
            .unwrap();

        Ok(())
    }

    pub fn check_log_suitability(
        log: &Log,
        chain_short: &str,
        config: &Info,
    ) -> bool {
        match config.permissions.get(&log.address.to_string()) {
            Some(hashes) => hashes.contains(&log.topics[0].to_string()),
            None => false,
        }
    }

    pub async fn delete(
        row: RelaySwaps,
        conn: &PgConnection,
        config: Arc<ChainInfo>,
    ) -> Result<(),Error> {
        let txn_id = H256::from_str(row.destination_txn_id.as_ref().unwrap()).unwrap();
        let info = config
            .info
            .get(row.destination_chain.as_ref().unwrap())
            .unwrap();

        let http = web3::transports::Http::new(&info.rpc_url)
                .expect("err creating http");
        let web3 = web3::Web3::new(http);
        let logs = web3
            .eth()
            .transaction_receipt(txn_id)
            .await
            .unwrap()
            .unwrap()
            .logs;
        let logs = logs
            .into_iter()
            .filter(|v| check_log_suitability(v,
                    row.destination_chain.as_ref().unwrap(), info))
            .collect::<Vec<Log>>();
        let log = logs
            .get(0)
            .unwrap();

        let uuid = log.data.0.as_slice();
        let uuid = Uuid::from_slice(&uuid[32..32+16])
            .expect("error getting uuid in event data");

        let _updates = diesel::update(relay_swaps::table)
            .filter(relay_swaps::id.eq(uuid))
            .set((
                    relay_swaps::finalize.eq(diesel::dsl::now),
                    relay_swaps::status.eq("finished")))
            .execute(conn)
            .unwrap();

        Ok(())
    }
}

pub fn fetch(port: i32, conn: &PgConnection) -> Result<String,Error> {
    let data = extracted_data::table
        .order_by(&(
                extracted_data::block_id.asc(),
                extracted_data::priority.desc(),
        ))
        .filter(extracted_data::port.eq(port))
        .select(extracted_data::base64bytes)
        .get_result::<String>(conn)?;
    Ok(data)
}

pub fn delete(
    num: i64,
    poller_id: i32,
    ids: Vec<Uuid>, 
    conn: &PgConnection
) -> Result<(),Error> {
    conn.build_transaction()
        .read_write()
        .run::<(), diesel::result::Error, _>(|| {
            for id in ids {
                diesel::delete(extracted_data::table.filter(extracted_data::id.eq(id)))
                .execute(conn)?;
            }
            diesel::update(poller_states::table.filter(poller_states::id.eq(poller_id)))
                .set(poller_states::num.eq(num))
                .execute(conn)?;
            Ok(())
        })?;
    Ok(())
}
pub fn load_num(poller_id: i32,conn: &PgConnection) -> Result<i64,Error> {
    poller_states::table
        .filter(poller_states::id.eq(poller_id))
        .select(poller_states::num)
        .get_result::<i64>(conn)
}

