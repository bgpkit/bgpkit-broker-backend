pub mod routeviews;
pub mod riperis;

use crate::models::*;
use crate::errors::*;
use regex::Regex;
use chrono::NaiveDateTime;

pub use routeviews::RouteViewsScraper;
pub use riperis::RipeRisScraper;
use crate::db::DbConnection;
