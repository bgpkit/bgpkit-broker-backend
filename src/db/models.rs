use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeStruct;

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Eq, PartialEq, sqlx::FromRow)]
pub struct Item {
    pub ts_start: chrono::NaiveDateTime,
    pub ts_end: chrono::NaiveDateTime,
    pub collector_id: String,
    pub data_type: String,
    pub url: String,
    pub rough_size: i64,
    pub exact_size: i64,
}

impl Serialize for Item {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("Item", 7)?;
        state.serialize_field("ts_start", self.ts_start.format("%Y-%m-%dT%H:%M:%S").to_string().as_str())?;
        state.serialize_field("ts_end", self.ts_end.format("%Y-%m-%dT%H:%M:%S").to_string().as_str())?;
        state.serialize_field("collector_id", self.collector_id.as_str())?;
        state.serialize_field("data_type", self.data_type.as_str())?;
        state.serialize_field("url", self.url.as_str())?;
        state.serialize_field("rough_size", &self.rough_size)?;
        state.serialize_field("exact_size", &self.exact_size)?;
        state.end()
    }
}