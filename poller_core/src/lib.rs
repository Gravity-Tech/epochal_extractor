use std::future::Future;
use async_trait::async_trait;
use database;

pub type Base64Bytes = String;
pub enum DatabaseAction {
    Insert,
    Delete,
}

#[async_trait]
pub trait PollerRuntime {
    async fn new_portion(
        &self, 
        num: u128,
    ) -> Result<Vec<Base64Bytes>, Box<dyn std::error::Error>>;
}


pub async fn run(
    func: impl PollerRuntime,
    poll_id: i32, 
    action: DatabaseAction,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = database::establish_connection();
    let mut epoch = database::load_num(poll_id, &db)? as u128;
    loop {
        println!("epoch {}",epoch+1);
        let data_portions = func.new_portion(epoch+1).await?; 
        println!("reach data {:?}", data_portions);
        match action {
            DatabaseAction::Insert => database::push(data_portions, &db)?,
            DatabaseAction::Delete => database::delete(data_portions, &db)?,
            
        };
        database::increment_num(poll_id, &db)?;
        epoch += 1;
    }
}

