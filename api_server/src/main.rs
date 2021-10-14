pub mod database;
extern crate env_logger;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde;
pub mod schema;
///
use actix_web::{post, get, middleware, web, App, Error, HttpResponse, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use serde::{Deserialize, Serialize};
use serde_json;
use base64;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Serialize,Deserialize)]
struct Delete {
    base64bytes: String,
    pass: String,
    key: String,
    txn: String,
}

#[derive(Serialize,Deserialize)]
struct NewSolana {
    accountId: String,
}

#[get("/delete")]
async fn delete(
    pool: web::Data<DbPool>,
    data: web::Query<Delete>,
    port: web::Data<(i32,)>,
    sec: web::Data<(Vec<u8>,Vec<u8>,String,String,String)>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");
    if sec.2 != data.pass {
        return Err(HttpResponse::NotFound().finish())?;
    }
    database::delete(data.base64bytes.clone(), data.key.clone(), data.txn.clone(), &conn)
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::NotFound().finish()
        })?;
    Ok(HttpResponse::Ok().json(()))
}

#[derive(Serialize,Deserialize)]
struct Key {
    key: String,
}

#[get("/solana_txn_id")]
async fn solana_txn_id(
    pool: web::Data<DbPool>,
    data: web::Query<Key>,
    port: web::Data<(i32,)>,
    sec: web::Data<(Vec<u8>,Vec<u8>,String,String,String)>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "txn_id": database::get_txn_id(data.key.clone(), &conn).await.unwrap(),
    })))
}

#[get("/insert")]
async fn insert(
    pool: web::Data<DbPool>,
    data: web::Query<NewSolana>,
    port: web::Data<(i32,)>,
    sec: web::Data<(Vec<u8>,Vec<u8>,String,String,String)>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");

    println!("lol");
    // use web::block to offload blocking Diesel code without blocking server thread
    let _ = database::push(
            data.accountId.clone(), 
            sec.3.clone(),
            port.0,
            &sec.0,
            &sec.1,
            &sec.4,
            &conn,
        )

        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::InternalServerError().finish()
        })?;
    Ok(HttpResponse::Ok().json(()))
}

#[derive(Serialize,Deserialize)]
#[allow(non_snake_case)]
struct Data {
    Type: String,
    Value: String, 
}

#[get("/extract")]
async fn extract(
    pool: web::Data<DbPool>,
    port: web::Data<(i32,)>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");

    // use web::block to offload blocking Diesel code without blocking server thread
    let portion = web::block(move || database::fetch(port.0, &conn))
        .await
        .map_err(|e| {
            eprintln!("{}", e);
            HttpResponse::NotFound().finish()
        })?;
    Ok(HttpResponse::Ok().json( Data {
        Type: "base64".to_string(),
        Value: portion,
    }))
}

#[derive(Serialize,Deserialize)]
#[allow(non_snake_case)]
struct Info {
    Tag: String,
    Description: String, 
}

#[get("/info")]
async fn info() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(Info{
        Tag: "Eth-Opera-Stablecoins".to_string(),
        Description: "Ethereum IERC20 token extractor".to_string(),
    }))
}

async fn aggregate(
    data_arr: web::Json<Vec<Data>>
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(Data {
        Type: "base64".to_string(),
        Value: data_arr[0].Value.clone(),
    }))
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    dotenv::dotenv().ok();
    let delete_key = std::env::var("DELETE_PASS").expect("no delete pass set");
    let token = std::env::var("TOKEN_ADDRESS").expect("s set");
    let port_addr = std::env::var("PORT_WALLET").expect("no delete pass set");
    let rpc_url = std::env::var("RPC_URL").expect("no delete pass set");
    let owner = std::env::var("OWNER").expect("no delete pass set");

    let token = bs58::decode(token).into_vec().unwrap();
    let port_addr = bs58::decode(port_addr).into_vec().unwrap();
    let data = (token,port_addr,delete_key,rpc_url,owner);
    // set up database connection pool
    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let port: i32 = std::env::var("SERVER_BIND_PORT")
        .expect("SERVER_BIND_PORT")
        .parse()
        .unwrap();
    let manager = ConnectionManager::<PgConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let bind = "0.0.0.0:8088";
    println!("Starting server at: {}", &bind);
    println!("solloha!");

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .data(data.clone())
            .data((port.clone(),))
            .wrap(middleware::Logger::default())
            .service(insert)
            .service(solana_txn_id)
            .service(delete)
            .service(extract)
            .service(info)
            .service(
                web::resource("/aggregate").route(
                    web::post().to(aggregate))
            )
    })
    .bind(&bind)?
    .run()
    .await
}

