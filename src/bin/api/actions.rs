use serde::{Serialize, Deserialize};
use diesel::{ExpressionMethods, PgConnection, QueryDsl, TextExpressionMethods, RunQueryDsl};
use diesel::dsl::count_star;
use diesel::result::Error;
use billboard_backend::models::{Collector, Item, UpdateTime};
use crate::pagination::LoadPaginated;

const MAX_PAGE_SIZE: i64 = 100_000;
const DEFAULT_PAGE_SIZE: i64 = 10;

fn get_default_page() -> Option<i64> { Some(1) }

fn get_default_page_size() -> Option<i64> { Some(DEFAULT_PAGE_SIZE) }

#[derive(Deserialize, Debug)]
pub struct Info {
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    collector: Option<String>,
    project: Option<String>,
    data_type: Option<String>,
    order: Option<String>,
    #[serde(default="get_default_page")]
    page: Option<i64>,
    #[serde(default="get_default_page_size")]
    page_size: Option<i64>,
}

#[derive(Serialize)]
pub struct ItemsResult {
    items: Vec<Item>,
    total_pages: i64,
    page_size: i64,
    current_page: i64,
    count: usize,
}

/// Search items query
///
/// search by:
/// - timestamp: start, end
/// - collector
/// - projects
/// - types
pub fn search_items(conn: &PgConnection, info: Info) -> Result<ItemsResult, Error> {

    // use billboard_backend::db::schema::items::columns::{collector_id, timestamp, data_type};
    // use billboard_backend::db::schema::items::dsl::items;
    use billboard_backend::db::schema::items;
    let mut query = items::table.into_boxed();

    // timestamps filter
    if let Some(start) = info.start_ts {
        query = query.filter(items::timestamp.ge(start));
    }
    if let Some(end) = info.end_ts {
        query = query.filter(items::timestamp.le(end));
    }

    // collector and project
    if let Some(collector) = info.collector {
        query = query.filter(items::collector_id.like(collector));
    }
    if let Some(project) = info.project {
        let pattern = match project.as_str() {
            "routeviews" | "route-views" => "route-views%",
            "riperis" | "ris" | "ripe" | "rrc" => "rrc%",
            _ => ""
        };
        query = query.filter(items::collector_id.like(pattern));
    }

    if let Some(itype) = info.data_type {
        let pattern = match itype.as_str() {
            "update"|"up"|"u" => "update" ,
            "rib"|"tabledump"|"r" => "rib",
            _ => ""
        };
        query = query.filter(items::data_type.like(pattern))
    }

    // if let Some(l) = info.limit {
    //     // limit at MAX_SEARCH_LIMIT
    //     if l < MAX_SEARCH_LIMIT {
    //         query = query.limit(l);
    //     } else {
    //         query = query.limit(MAX_SEARCH_LIMIT);
    //     }
    // } else {
    //     query = query.limit(DEFAULT_SEARCH_LIMIT);
    // }

    if let Some(order) = &info.order {
        match order.to_lowercase().as_str() {
            "asc"|"reverse" => {query = query.order(items::timestamp.asc());},
            _ => {query = query.order(items::timestamp.desc());},
        }
    } else {
        query = query.order(items::timestamp.asc());
    }

    let (current_page, mut page_size) = (info.page.clone().unwrap(), info.page_size.clone().unwrap());
    if page_size>MAX_PAGE_SIZE {
        page_size = MAX_PAGE_SIZE;
    }
    let (items, total_pages) = query.load_with_pagination(&conn, current_page, page_size)?;
    let count = items.len();
    Ok(ItemsResult{
        items,
        total_pages,
        page_size,
        current_page,
        count
    })
}

pub fn get_collectors(conn: &PgConnection) -> Result<Vec<Collector>, Error> {
    use billboard_backend::db::schema::collectors::dsl::*;
    collectors.load::<Collector>(conn)
}

pub fn get_latest_timestamps(conn: &PgConnection) -> Result<Vec<UpdateTime>, Error> {
    use billboard_backend::db::schema::latest_times::dsl::*;
    latest_times.load::<UpdateTime>(conn)
}

pub fn get_total_count(conn: &PgConnection) -> Result<i64, Error> {
    use billboard_backend::db::schema::items::dsl::*;
    items.select(count_star()).first(conn)
}
