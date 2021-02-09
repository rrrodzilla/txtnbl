use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(shorten).service(redirect))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

#[post("/shorten")]
async fn shorten(location: web::Json<Location>) -> impl Responder {
    println!("Shortening {}...", location.url);
    HttpResponse::Ok().json(ShortenCode {
        code: "test123".to_string(),
    })
}

#[get("/")]
async fn redirect() -> impl Responder {
    println!("Redirecting...");
    HttpResponse::PermanentRedirect()
        .set_header("LOCATION", "https://www.google.com")
        .finish()
}

#[derive(Serialize, Deserialize)]
struct ShortenCode {
    code: String,
}

#[derive(Deserialize)]
struct Location {
    url: String,
}
