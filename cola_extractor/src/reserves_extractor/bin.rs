use cola::{
    cola_kernel,
    processor_default,
    config::{
        parse_config,
        ColaConfig,
        Logger,
    },
};
use tokio::time::{
    delay_for, 
    Duration
};
use web3::types::{
    Address,
    U256,
};
use base64;
use tokio;
use tokio_stream::StreamExt;
use diesel::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use std::sync::{
    Arc,
    RwLock,
};
use telegram_bot::*;
use database::reserves::*;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    //let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    //let api = Api::new(token);
    //let chat_id: i64 = env::var("TELEGRAM_LOG_CHAT_ID")
    //    .expect("TELEGRAM_LOG_CHAT_ID not set")
    //    .parse()
    //    .expect("err parsing i64");
    //let chat = ChatId::new(chat_id);
    //let logger = Logger::new(api,chat).await;

    let external_db_url = env::var("RESERVES_DB_URL").expect("error ");
    let manager = ConnectionManager::<PgConnection>::new(external_db_url);
    let dest_pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let dest_pool = Arc::new(dest_pool);

    let source_db_url = env::var("EXTRACTOR_DB_URL").expect("error ");
    let manager = ConnectionManager::<PgConnection>::new(source_db_url);
    let source_pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let source_pool = Arc::new(source_pool);

    let port: i32 = env::var("PORT").expect("port nor found").parse().unwrap();
    let mins: u64 = env::var("DELAY_IN_MINS").expect("delay nor found").parse().unwrap();
    reserves_transferer(mins, port, source_pool, dest_pool).await;
}

pub async fn reserves_transferer(
    mins: u64,
    port: i32,
    pool_from: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    pool_to: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
) -> ! { 
    use std::str::FromStr;
    loop {
        let data = get_reserves(port, &pool_from.get().unwrap()).unwrap();
        let data: Vec<(Uuid,String)> = data
            .into_iter()
            .map(|v|{
                let mut b = Vec::new();
                let id = Uuid::new_v4();
                b.extend_from_slice(id.as_bytes());
                b.extend_from_slice(v.chain_short.as_bytes());
                b.extend_from_slice(&v.pool_address.as_bytes()[2..]);
                let modif = v.modifier.iter().sum::<i32>();
                let reserves = U256::from_str(
                    &(v.gton_reserves as u64).to_string()).unwrap();  
                let modif = U256::from_str(&modif.to_string()).unwrap();
                let reserves = reserves * modif / U256::from_str("1000").unwrap();
                let reserves: [u8;32] = reserves.into();
                b.extend_from_slice(&reserves);
                (id,base64::encode(b))
            })
            .collect();
        println!("{:?}",data);   
        extract_reserves(port, data, &pool_to.get().unwrap()).unwrap();
        delay_for(Duration::from_secs(60*mins)).await;
    }
}
