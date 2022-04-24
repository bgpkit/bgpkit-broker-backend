use std::env;
use clap::Clap;
use diesel::RunQueryDsl;
use futures::StreamExt;
use log::info;
use bgpkit_broker_backend::db::DbConnection;
use bgpkit_broker_backend::scrapers::check_size;

#[derive(Clap)]
struct Opts {
    /// Database URL string, this overwrites the DATABASE_URL env variable
    #[clap(short, long)]
    db_url: Option<String>,

    /// Only scrape most recent data
    #[clap(short, long)]
    latest: bool,
}

fn main () {

    let blocking_cpus = match num_cpus::get() {
        1 => 1,
        n => n/2,
    };

    info!("using {} cores for parsing html pages", blocking_cpus);
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all()
        .max_blocking_threads(blocking_cpus)
        .build().unwrap();

    // init
    env_logger::init();
    let _ = dotenv::dotenv();

    // parse parameters
    let opts: Opts = Opts::parse();

    // take in database connection string
    let db_url = match opts.db_url.clone() {
        Some(db_url) => db_url,
        None => {
            env::var("DATABASE_URL").expect("DATABASE_URL must be set")
        }
    };

    let conn = DbConnection::new(&db_url);

    use bgpkit_broker_backend::db::schema::items::dsl::*;
    use bgpkit_broker_backend::db::schema::items::url;

    let batch_size = 1000;

    rt.block_on(async {
        loop {

            let files = conn.get_urls_unverified(batch_size);
            let total_files = files.len();
            if files.is_empty() {
                break
            }
            info!("loaded the {} files without file size, start checking sizes", total_files);

            let mut stream =
                futures::stream::iter(files)
                    .map( |file| {
                        check_size(file)
                    } )
                    .buffer_unordered(100);

            let mut updated = vec![];
            while let Some(item_opt) = stream.next().await {
                if let Some(item) = item_opt {
                    updated.push(item);
                }
            }

            info!("updating database for  {} files", updated.len());
            let url_clone = db_url.clone();
            tokio::task::spawn_blocking(move || {
                let conn = DbConnection::new(&url_clone);
                for item in updated{
                    diesel::insert_into(items)
                        .values(&item)
                        .on_conflict(url)
                        .do_update()
                        .set(&item)
                        .execute(&conn.conn).unwrap();
                }
            }).await.unwrap();
            info!("updated {} out of {} files' sizes", total_files, batch_size);
        }
    });
}

