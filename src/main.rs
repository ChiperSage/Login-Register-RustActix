mod auth;
mod dashboard;

use actix_web::{web, App, HttpResponse, HttpServer, Result, Error};
use tera::{Tera, Context};
use sqlx::MySqlPool;
use log::error;
use env_logger;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;

async fn welcome(tmpl: web::Data<Tera>) -> Result<HttpResponse, Error> {
    let s = tmpl
        .render("welcome.html", &Context::new())
        .map_err(|_| actix_web::error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

async fn check_db(pool: web::Data<MySqlPool>) -> Result<HttpResponse, Error> {
    let conn_result = sqlx::query("SELECT 1")
        .execute(pool.get_ref())
        .await;

    match conn_result {
        Ok(_) => Ok(HttpResponse::Ok().body("Database connection successful")),
        Err(err) => {
            error!("Database connection failed: {:?}", err);
            Ok(HttpResponse::InternalServerError().body("Database connection failed"))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let pool = match MySqlPool::connect("mysql://root:@localhost:3306/database_name").await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Failed to create pool: {:?}", err);
            std::process::exit(1);
        }
    };

    let secret_key = Key::generate();

    HttpServer::new(move || {
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
            .unwrap_or_else(|err| {
                eprintln!("Template initialization error: {:?}", err);
                std::process::exit(1);
            });

        App::new()
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(), 
                secret_key.clone(),
            ))
            .app_data(web::Data::new(tera))
            .app_data(web::Data::new(pool.clone()))
            .configure(dashboard::configure_routes)
            .configure(auth::configure_routes)
            .route("/", web::get().to(welcome))
            .route("/check-db", web::get().to(check_db))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
