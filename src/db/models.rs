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

#[derive(Debug, Clone, Queryable, Insertable, Identifiable, Eq, PartialEq, AsChangeset)]
#[table_name="items"]
#[primary_key(url)]
pub struct Item {
    pub ts_start: chrono::NaiveDateTime,
    pub ts_end: chrono::NaiveDateTime,
    pub collector_id: String,
    pub data_type: String,
    pub url: String,
    pub rough_size: i64,
    pub exact_size: i64,
}