use crate::db::schema::{collectors,items};
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Insertable, Serialize, Deserialize)]
#[table_name="collectors"]
pub struct Collector {
    pub id: String,
    pub project: String,
    pub url: String,
}


#[derive(Debug, Clone, Copy)]
pub enum DataType {
    BgpUpdate,
    BgpTableDump,
}

impl DataType {
    pub fn to_string(self) -> String {
        match self {
            DataType::BgpUpdate => {"update".to_string()}
            DataType::BgpTableDump => {"rib".to_string()}
        }
    }
}

#[derive(Debug, Queryable, Insertable, Serialize, Deserialize, Eq, PartialEq)]
#[table_name="items"]
pub struct Item {
    pub ts_start: i64,
    pub ts_end: i64,
    pub collector_id: String,
    pub data_type: String,
    pub url: String,
    pub file_size: i64,
}

#[derive(Debug, Queryable, Serialize, Deserialize)]
pub struct UpdateTime {
    pub timestamp: i64,
    pub collector_id: String,
    pub data_type: String,
    pub project: String,
    pub collector_url: String,
    pub item_url: String,
}