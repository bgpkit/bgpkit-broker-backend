mod actions;
mod pagination;

use std::fmt::Display;
use actix_web::{get, middleware, web, App, Error, HttpResponse, HttpServer};

use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use serde::Serialize;
use serde_json::json;
use actions::Info;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

fn prepare_response<T: Serialize, U>(data: Result<T,U>) -> HttpResponse
    where U: Display
{
    match data {
        Ok(data) => {
            HttpResponse::Ok().json(json!({
                "data": data,
                "error": null,
            }))
        }
        Err(error) => {
            HttpResponse::Ok().json(json!({
                "data": null,
                "error": error.to_string(),
            }))
        }
    }
}

async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().json(json!({
            "data": null,
            "error": "endpoint not found, use /v1/meta or /v1/search instead",
    }))
}

/// Search MRT data items.
#[get("/v1/search")]
async fn search_items(
    pool: web::Data<DbPool>,
    info: web::Query<Info>
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");

    // use web::block to offload blocking Diesel code without blocking server thread
    let items = web::block(move || actions::search_items(&conn, info.into_inner()))
        .await;

    Ok(prepare_response(items))
}

/// Meta information.
#[get("/v1/meta/{stats_type}")]
async fn get_meta(
    pool: web::Data<DbPool>,
    stats_type: web::Path<String>
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");

    // use web::block to offload blocking Diesel code without blocking server thread
    match stats_type.clone().as_str() {
        "collectors" => {
            let items = web::block(move || actions::get_collectors(&conn))
                .await;
            Ok(prepare_response(items))
        }
        "latest_times" => {
            let items = web::block(move || actions::get_latest_timestamps(&conn))
                .await;
            Ok(prepare_response(items))
        }
        "total_count" => {
            let items = web::block(move || actions::get_total_count(&conn))
                .await;
            Ok(prepare_response(items))

        }
        _ => {
            let res = HttpResponse::NotFound()
                .body(format!("Unknown query: /v1/meta/{}", stats_type));
            Ok(res)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    dotenv::dotenv().ok();

    // set up database connection pool
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let bind = "0.0.0.0:8080";

    println!("Starting server at: {}", &bind);

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            // set up DB pool to be used with web::Data<Pool> extractor
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .service(search_items)
            .service(get_meta)
            .default_service(
            web::route().to(not_found))
    })
        .bind(&bind)?
        .run()
        .await
}

