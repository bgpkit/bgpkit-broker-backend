from datetime import datetime
from typing import List, Optional

from pydantic import BaseModel, validator
from pony.orm import Database, Required, PrimaryKey, set_sql_debug

db = Database()
db.bind(provider="postgres", user="bgpkit_admin", host="db.broker.bgpkit.com", database="bgpkit_stage")
# set_sql_debug(True)


class Item(db.Entity):
    _table_ = "items"
    ts_start = Required(datetime, sql_type='timestamp without time zone')
    ts_end = Required(datetime, sql_type='timestamp without time zone')
    collector_id = Required(str)
    data_type = Required(str)
    url = PrimaryKey(str)
    rough_size = Required(int)
    exact_size = Required(int)


class ItemModel(BaseModel):
    ts_start: datetime
    ts_end: datetime
    collector_id: str
    data_type: str
    url: str
    rough_size: int
    exact_size: int

    @validator('ts_start', pre=True)
    def validate_ts_start(cls, value: datetime):
        return value.timestamp()

    @validator('ts_end', pre=True)
    def validate_ts_end(cls, value: datetime):
        return value.timestamp()

    class Config:
        orm_mode = True


class SearchResultModel(BaseModel):
    count: Optional[int]
    page: Optional[int]
    page_size: Optional[int]
    error: Optional[str]
    data: Optional[List[ItemModel]]


db.generate_mapping(create_tables=False)
