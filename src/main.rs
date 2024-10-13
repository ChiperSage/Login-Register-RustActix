mod auth;
mod dashboard; // Import the dashboard module

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

    // Attempt to connect to the database
    let pool = match MySqlPool::connect("mysql://root:@localhost:3306/rust_auth").await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Failed to create pool: {:?}", err);
            std::process::exit(1); // Exit if the database connection fails
        }
    };

    let secret_key = Key::generate(); // Generate secret key for session

    // Start the HTTP server
    HttpServer::new(move || {
        // Initialize Tera template engine
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
            .unwrap_or_else(|err| {
                eprintln!("Template initialization error: {:?}", err);
                std::process::exit(1);
            });

        // App configuration
        App::new()
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(), // Use the session store
                secret_key.clone(), // Provide the secret key for encryption
            ))
            .app_data(web::Data::new(tera)) // Templating engine
            .app_data(web::Data::new(pool.clone())) // Database pool
            .configure(dashboard::configure_routes) // Configure dashboard routes
            .configure(auth::configure_routes) // Configure auth routes
            .route("/", web::get().to(welcome)) // Welcome route
            .route("/check-db", web::get().to(check_db)) // Check database connection route
    })
    .bind("127.0.0.1:8080")? // Bind server to local address
    .run()
    .await
}
