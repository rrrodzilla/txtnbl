#[macro_use]
extern crate log;

use actix_web::{get, http, post, web, App, HttpResponse, HttpServer, Responder};
use clap::Clap;
use harsh::Harsh;
use pickledb::{PickleDb, PickleDbDumpPolicy};
use serde::{Deserialize, Serialize};

#[derive(Clap)]
#[clap(
    version = "1.0",
    author = "Textnibble Microservices",
    about = "Simple service for creating short url redirects"
)]
struct Opts {
    #[clap(short, long, default_value = "http://localhost:8080/")]
    url_base: String,
    #[clap(short, long, default_value = "8080")]
    port: u32,
    #[clap(short, long, default_value = "default")]
    shard: String,
    #[clap(short, long)]
    delete_on_use: bool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "txtnbl=Info,actix_web=info,actix_server=info");
    env_logger::init();
    let opts: Opts = Opts::parse();

    info!("Config value - url_base: {}", opts.url_base);
    info!("Config value - shard: {}", opts.shard);
    info!("Config value - delete_on_use: {}", opts.delete_on_use);
    info!("Config value - port: {}", opts.port);

    HttpServer::new(|| App::new().service(shorten).service(redirect))
        .bind(format!("127.0.0.1:{}", opts.port))?
        .run()
        .await
}

#[post("/shorten")]
async fn shorten(location: web::Json<Location>) -> impl Responder {
    let opts: Opts = Opts::parse();
    //get the settings for this instance of the service
    let url_base = opts.url_base;
    let project_salt = opts.shard;

    let db_name = format!("{}.db", project_salt);
    //first load the db if it exists, else create a new one
    //    let mut db = PickleDb::new_json(format!("{}.db", project_salt), PickleDbDumpPolicy::AutoDump);
    let mut db = match PickleDb::load_json(&db_name, PickleDbDumpPolicy::AutoDump) {
        Ok(v) => v,
        Err(_e) => PickleDb::new_json(format!("{}.db", project_salt), PickleDbDumpPolicy::AutoDump),
    };

    //get the number of keys in the store
    let next_id: u64 = db.total_keys() as u64;

    //build an id for this based off the number of items in the key-value store
    //since we don't have a database id
    let harsh = Harsh::builder()
        .salt(project_salt)
        .length(6)
        .build()
        .unwrap();
    let id = harsh.encode(&[next_id]);
    info!("SHORTEN => URL: {} => SHORTCODE: {}", location.url, id);

    //now store it in the kv store
    db.set(&id, &location.url).unwrap();
    //return the shortened url
    HttpResponse::Ok().json(ShortenCode {
        code: id.clone(),
        url: String::from(format!("{}{}", url_base, id)),
    })
}

#[get("/{code}")]
async fn redirect(web::Path(code): web::Path<String>) -> impl Responder {
    let opts: Opts = Opts::parse();
    //get the settings for this instance of the service
    let project_salt = opts.shard;

    let db_name = format!("{}.db", project_salt);
    //first load the db if it exists, else create a new one
    //    let mut db = PickleDb::new_json(format!("{}.db", project_salt), PickleDbDumpPolicy::AutoDump);
    let mut db = match PickleDb::load_json(&db_name, PickleDbDumpPolicy::AutoDump) {
        Ok(v) => v,
        Err(_e) => PickleDb::new_json(format!("{}.db", project_salt), PickleDbDumpPolicy::AutoDump),
    };

    //try to get the value for the code provided
    let url: Option<String> = db.get(&code);
    //either redirect to the found url or pass a 404 not found
    let response = match url.clone() {
        Some(u) => HttpResponse::PermanentRedirect()
            .set_header("LOCATION", u)
            .finish(),
        None => HttpResponse::NotFound().finish(),
    };

    //let's check the status code.  if the status is a redirect (308), then we know
    //the code was found and we're about to redirect.  since that's the case, we can
    //delete this value from the store
    if response.status() == http::StatusCode::PERMANENT_REDIRECT {
        info!("REDIRECT => Found url {:?} for code {}", &url, &code);
        if opts.delete_on_use {
            info!("REDIRECT => Deleting used code {}...", &code);
            db.rem(&code).unwrap();
        }
    } else {
        warn!("No url found for code {:?}", &code);
    }

    response
}

#[derive(Serialize, Deserialize)]
struct ShortenCode {
    code: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Location {
    url: String,
}
