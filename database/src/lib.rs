#[macro_use]
extern crate diesel;
use diesel::prelude::*;
pub mod schema;
use crate::schema::extracted_data;
use crate::schema::poller_states;
use diesel::result::Error;
use uuid::Uuid;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2;
pub type ConnPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection(database_url: &str) -> ConnPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
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

/// this function uses increment num and pushing data together in one transaction
/// in case any of it fails, we will rollback, so data would be safe and enough stressful
pub fn push(
    num: i64,
    poller_id: i32,
    data: Vec<(Uuid,String,i64,i32,i32)>, 
    conn: &PgConnection
) -> Result<(),Error> {
    conn.build_transaction()
        .read_write()
        .run::<(), diesel::result::Error, _>(|| {
            let ct = chrono::Utc::now().naive_utc();
            for d in data {
                diesel::insert_into(extracted_data::table)
                    .values(InsertableData{
                        id: d.0,
                        base64bytes: d.1,
                        block_id: ct,
                        priority: d.3,
                        port: d.4,
                    })
                    .execute(conn)?;
            }
            diesel::update(poller_states::table.filter(poller_states::id.eq(poller_id)))
                .set(poller_states::num.eq(num))
                .execute(conn)?;
            Ok(())
        })?;
    Ok(())
}

pub mod reserves {

    use super::*;
    use diesel::sql_types::{
        Double,
        VarChar,
        Array,
        Integer,
    };

    #[derive(Debug,QueryableByName)]
    pub struct FromData {
        #[sql_type="VarChar"]
        pub chain_short: String,
        #[sql_type="VarChar"]
        pub pool_address: String,
        #[sql_type="Array<Integer>"]
        pub modifier: Vec<i32>,
        #[sql_type="Double"]
        pub gton_reserves: f64,

    }

    pub fn get_reserves(
            port: i32, 
            conn_from: &PgConnection,
        ) -> Result<Vec<FromData>,Error> {
            Ok(diesel::sql_query("select
                    c.chain_short,
                    p.gton_reserves,
                    p.modifier,
                    p.pool_address
                from pools p
                    join dexes d on d.id = p.dex_id
                    join chains c on c.id = d.chain_id;")
            .load::<FromData>(conn_from).unwrap())
        }
    pub fn extract_reserves(
            port: i32, 
            data: Vec<(Uuid,String)>,
            conn_to: &PgConnection,
        ) -> Result<(),Error> {
        conn_to.build_transaction()
            .read_write()
            .run::<(), diesel::result::Error, _>(|| {
            let ct = chrono::Utc::now().naive_utc();
            for d in data {
                diesel::insert_into(extracted_data::table)
                    .values(InsertableData{
                        id: d.0,
                        base64bytes: d.1,
                        block_id: ct,
                        priority: 0,
                        port: port,
                    })
                    .execute(conn_to).unwrap();
            }
            Ok(())
        })?;
        Ok(())
    }
    pub fn delete(
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
                Ok(())
            })?;
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

