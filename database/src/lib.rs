pub mod config;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde;
use diesel::prelude::*;
use std::sync::Arc;
pub mod schema;
use crate::schema::{
    extracted_data,
    poller_states,
    processed_solana_data_accounts,
};
use bigdecimal::BigDecimal;
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

pub async fn delete(
    row: String,
    conn: &PgConnection,
) -> Result<(),Error> {
    diesel::delete(extracted_data::table)
        .filter(extracted_data::base64bytes.eq(row))
        .execute(conn)?;
    Ok(())
}


#[derive(Queryable,Insertable,Debug,Clone)]
#[table_name="extracted_data"]
struct InsertableData {
    id: Uuid,
    base64bytes: String,
    block_id: chrono::NaiveDateTime,
    priority: i32,
    port: i32,
}

extern crate serde_json;
extern crate hex;

fn into_extractor_bytes(inp: &str,
    token_program_true: &[u8], 
    token_destination_true: &[u8]) -> (Uuid,String) {
    let bin_data = base64::decode(inp).unwrap();
    let to = &bin_data[0..20];
    let chain = &bin_data[64..67];
    use std::convert::TryInto;
    let amount = f64::from_ne_bytes((&bin_data[67..75]).try_into().unwrap());
    //let from = &bin_data[75..107];
    let token_program = &bin_data[107..139];
    let token_destination = &bin_data[139..139+32];
    if token_destination != token_destination_true 
        && token_destination != token_destination_true {
        panic!();
    }

    let id = uuid::Uuid::new_v4();
    let mut by: Vec<u8> = Vec::new();
    by.extend_from_slice(id.as_bytes());
    by.extend_from_slice(chain);
    by.extend_from_slice(&[0u8;20]);
    by.extend_from_slice(&[3u8;1]);
    by.extend_from_slice(
        hex::decode("0xa4f88aed847e87bafdc18210d88464dc24f71fa4bf1b4672710c9bc876bb0044")
        .unwrap()
        .as_ref());
    by.extend_from_slice(&[0u8;4*32]);
    by.extend_from_slice(&[0]); // amount
    by.extend_from_slice(&[0u8;32+29]);
    by.extend_from_slice(chain); 
    by.extend_from_slice(&[0u8;32]);
    by.extend_from_slice(to);
    (id,base64::encode(by))
}

pub async fn get_txn_id(
    account_id: String,
    conn: &PgConnection,
) -> Result<Option<String>,Error> {
    let r = processed_solana_data_accounts::table.filter(
        processed_solana_data_accounts::account_id.eq(&account_id))
        .select(processed_solana_data_accounts::destination_txn_id)
        .get_result::<Option<String>>(conn).unwrap();
    Ok(r)
}

pub async fn push(
    account_id: String,
    rpc_url: String,
    port: i32,
    state_auht: &[u8], 
    market_auth: &[u8],
    conn: &PgConnection,
) -> Result<(),Error> {
    let dat: Vec<i64> = processed_solana_data_accounts::table.filter(
        processed_solana_data_accounts::account_id.eq(&account_id))
        .select(processed_solana_data_accounts::id)
        .get_results(conn).unwrap();
    if dat.len() != 0 {
        panic!();
    }
    println!("valid");
    let client = reqwest::Client::new();
    let res: serde_json::Value = client.post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
                "id": 1,
                "method": "getAccountInfo",
                "params": [
                  account_id,
                  {
                    "encoding": "base64encoded"
                  }
                ]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let res = res["result"]["value"]["data"].as_str().unwrap();
    println!("{:?}",res);

    let (id,base64bytes) = into_extractor_bytes(res,state_auht,market_auth);
    let ct = chrono::Utc::now().naive_utc();
    diesel::insert_into(extracted_data::table)
    .values(InsertableData{
        id,
        base64bytes,
        block_id: ct,
        priority: 0,
        port,
    })
    .execute(conn)?;
    println!("inserted");
    Ok(())
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

#[cfg(test)]
pub mod test {
    use super::*;
    #[test]
    fn parse_value() {

        //let (_,res) = 
    }
}
