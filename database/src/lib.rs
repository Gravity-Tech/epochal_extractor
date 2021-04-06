#[macro_use]
extern crate diesel;
use diesel::prelude::*;
pub mod schema;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;
use crate::schema::extracted_data;
use crate::schema::poller_states;
use diesel::result::Error;


pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

#[derive(Insertable,AsChangeset,Debug,Clone)]
#[table_name="extracted_data"]
struct InsertableData {
    base64bytes: String,
}

/// this function uses increment num and pushing data together in one transaction
/// in case any of it fails, we will rollback, so data would be safe and enough stressful
pub fn push(data: Vec<String>, conn: &PgConnection) -> Result<(),Error> {
    conn.build_transaction()
        .read_write()
        .run::<(), diesel::result::Error, _>(|| {
            diesel::insert_into(extracted_data::table)
                .values(data
                    .into_iter()
                    .map(|x| InsertableData {base64bytes: x} )
                    .collect::<Vec<InsertableData>>())
                .execute(conn)?;
            Ok(())
        })?;
    Ok(())
}

pub fn fetch(conn: &PgConnection) -> Result<String,Error> {
    let (id,data) = extracted_data::table
        .order_by(extracted_data::id.asc())
        .get_result::<(i32,String)>(conn)?;
    Ok(data)
}

pub fn delete(rows: Vec<String>, conn: &PgConnection) -> Result<(),Error> {
    for row in rows {
        diesel::delete(extracted_data::table.filter(extracted_data::base64bytes.eq(row)))
            .execute(conn)?;
    }
    Ok(())
}

pub fn increment_num(poller_id: i32,conn: &PgConnection) -> Result<usize,Error> {
    diesel::update(poller_states::table.filter(poller_states::id.eq(poller_id)))
        .set(poller_states::num.eq(poller_states::num + 1))
        .execute(conn)
}

pub fn load_num(poller_id: i32,conn: &PgConnection) -> Result<i32,Error> {
    poller_states::table
        .filter(poller_states::id.eq(poller_id))
        .select(poller_states::num)
        .get_result::<i32>(conn)
}

