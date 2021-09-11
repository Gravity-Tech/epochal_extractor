use serde_yaml::{
    self,
    Value,
    Mapping,
};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
use std::sync::Arc;

use std::collections::HashMap;
use serde::{
    Serialize,
    Deserialize,
};

#[derive(Clone,Deserialize,Serialize)]
pub struct Info {
    pub permissions: HashMap<String,Vec<String>>,
    pub rpc_url: String,
}
#[derive(Clone,Deserialize,Serialize)]
pub struct ChainInfo {
    pub info: HashMap<String,Info>,
}

pub async fn parse_config(filename: String) -> ChainInfo {
    let data = std::fs::read_to_string(filename).expect("failed to read cola.yaml");
    let v: ChainInfo = serde_yaml::from_str(&data).expect("error parsing yaml file");
    v
}
