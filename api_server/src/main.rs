use database;
extern crate env_logger;

use actix_web::{get, middleware, web, App, Error, HttpResponse, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use serde::{Deserialize, Serialize};

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

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

    HttpServer::new(move || {
        App::new()
            // set up DB pool to be used with web::Data<Pool> extractor
            .data(pool.clone())
            .data((port.clone(),))
            .wrap(middleware::Logger::default())
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

