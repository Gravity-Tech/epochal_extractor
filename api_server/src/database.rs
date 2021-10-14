use anchor_lang::prelude::*;
use diesel::prelude::*;
use std::{os::unix::prelude::OsStrExt, sync::Arc};
use crate::schema::{
    extracted_data,
    processed_solana_data_accounts,
    solana_txns,
};
use bigdecimal::BigDecimal;
use diesel::result::Error;
use uuid::Uuid;
use diesel::PgConnection;
use base64;
use diesel::r2d2::ConnectionManager;
use web3::types::U256;
use r2d2;
pub type ConnPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection(database_url: &str) -> ConnPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}

pub async fn delete(
    row: String,
    key: String,
    txn: String,
    conn: &PgConnection,
) -> Result<(),Error> {
    diesel::delete(extracted_data::table)
        .filter(extracted_data::base64bytes.eq(row))
        .execute(conn)?;
    diesel::insert_into(solana_txns::table)
        .values(&(
            solana_txns::key.eq(key),
            solana_txns::txn_id.eq(txn),
        ))
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

#[account(zero_copy)]
#[derive(Debug)]
pub struct RelayEvent {
    pub to: [u8;64],
    pub dest_chain: [u8;3],
    pub amount: u64,
    pub from: Pubkey,
    pub token_program: Pubkey,
    pub transfer_destination: Pubkey,
}


fn into_extractor_bytes(inp: &str,
    token_program: &[u8], 
    token_destination: &[u8]) -> (Uuid,String,i32) {
    println!("row data base {:?}",inp);
    let mut bin_data = base64::decode(inp).unwrap();
    let data = RelayEvent::deserialize(&mut &bin_data[8..]).unwrap();
    println!("data {:?}",data);
    if token_destination != &data.transfer_destination.to_bytes() 
        && token_program != &data.token_program.to_bytes() {
        panic!();
    }

    let amount = U256::from(data.amount);
    let mut amount_buffer = [0u8;32];
    amount.to_big_endian(&mut amount_buffer);

    let id = uuid::Uuid::new_v4();
    let mut by: Vec<u8> = Vec::new();
    by.extend_from_slice(id.as_bytes());
    by.extend_from_slice(&data.dest_chain);
    by.extend_from_slice(&[0u8;20]);
    by.extend_from_slice(&[3u8;1]);
    by.extend_from_slice(
        hex::decode("a4f88aed847e87bafdc18210d88464dc24f71fa4bf1b4672710c9bc876bb0044")
        .unwrap()
        .as_ref());
    by.extend_from_slice(&[0u8;4*32]);
    by.extend_from_slice(&amount_buffer); // amount
    by.extend_from_slice(&[0u8;32+29]);
    by.extend_from_slice(&data.dest_chain); 
    by.extend_from_slice(&[0u8;32]);
    by.extend_from_slice(&data.to[0..32]);

    let prt: i32 = match data.dest_chain {
        [0,0,1] =>  8088,
        [0,0,2] =>  8089,
        [0,0,3] =>  8090, 
        [0,0,4] =>  8091,
        [0,0,5] =>  8092,
        [0,0,6] =>  8093,
        [0,0,7] =>  8094,
        [0,0,8] =>  8095,
        [0,0,0] =>  8096,
        _ => panic!(),
    };
    (id,base64::encode(by),prt)
}

pub async fn get_txn_id(
    key: String,
    conn: &PgConnection,
) -> Result<Option<String>,Error> {
    let r = solana_txns::table.filter(
        solana_txns::key.eq(&key))
        .select(solana_txns::txn_id)
        .get_results::<String>(conn).unwrap();
    Ok(match r.len() {
        0 => None,
        _ => Some(r[0].clone()),
    })
}

pub async fn push(
    account_id: String,
    rpc_url: String,
    port: i32,
    state_auht: &[u8], 
    market_auth: &[u8],
    owner: &str,
    conn: &PgConnection,
) -> Result<(),Error> {
    println!("try");
    let dat: Vec<i64> = processed_solana_data_accounts::table.filter(
        processed_solana_data_accounts::account_id.eq(&account_id))
        .select(processed_solana_data_accounts::id)
        .get_results(conn).unwrap();
    println!("try panic");
    if dat.len() != 0 {
        panic!();
    }
    println!("valid");
    let client = reqwest::Client::new();
    let res: serde_json::Value = client.post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
                "id": 1,
                "method": "getAccountInfo",
                "params": [
                  account_id,
                  {
                    "encoding": "base64"
                  }
                ]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    println!("{:?}",res);
    let r = res["result"]["value"]["data"][0].as_str().unwrap();
    let owner_try = res["result"]["value"]["owner"].as_str().unwrap();
    if owner != owner_try {
        panic!();
    }

    let (id,base64bytes,prt) = into_extractor_bytes(r,state_auht,market_auth);
    let ct = chrono::Utc::now().naive_utc();
    diesel::insert_into(extracted_data::table)
    .values(InsertableData{
        id,
        base64bytes,
        block_id: ct,
        priority: 0,
        port: prt,
    })
    .execute(conn)?;
    diesel::insert_into(processed_solana_data_accounts::table)
        .values(processed_solana_data_accounts::account_id.eq(account_id))
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
