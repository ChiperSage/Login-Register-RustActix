use actix_web::{web, HttpResponse, Error, Result};
use tera::{Tera, Context};
use actix_web::error::ErrorInternalServerError;
use serde::Deserialize;
use sqlx::{MySqlPool, FromRow};
use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use actix_session::Session;

#[derive(FromRow)]
pub struct User {
    pub user_id: i32,
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterFormData {
    pub username: String,
    pub email: String,
    pub password: String,
    pub password_confirm: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub identifier: String,
    pub password: String,
}

pub async fn show_login_form(tmpl: web::Data<Tera>, error: Option<String>) -> Result<HttpResponse, Error> {
    let mut ctx = Context::new();
    if let Some(error_message) = error {
        ctx.insert("error", &error_message);
    }
    let rendered = tmpl.render("login.html", &ctx)
        .map_err(|_| ErrorInternalServerError("Template rendering error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

pub async fn process_login_form(
    form: web::Form<LoginForm>, 
    pool: web::Data<MySqlPool>, 
    tmpl: web::Data<Tera>, 
    session: Session
) -> Result<HttpResponse, Error> {
    let user_record = sqlx::query_as!(
        User,
        r#"
        SELECT user_id, username, email, password 
        FROM users 
        WHERE username = ? OR email = ?
        "#,
        form.identifier,
        form.identifier
    )
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|err| {
        eprintln!("Error fetching user record: {:?}", err);
        ErrorInternalServerError("Database query error")
    })?;

    if let Some(user) = user_record {
        if verify(&form.password, &user.password)
            .map_err(|_| ErrorInternalServerError("Password verification error"))?
        {
            session.insert("username", &user.username)
                .map_err(|_| ErrorInternalServerError("Session insertion error"))?;

            return Ok(HttpResponse::SeeOther()
                .append_header(("Location", "/dashboard"))
                .finish());
        }
    }

    show_login_form(tmpl.clone(), Some("Invalid credentials.".to_string())).await
}

pub async fn logout(session: Session) -> Result<HttpResponse, Error> {
    session.clear();
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/auth/login"))
        .finish())
}

pub async fn show_register_form(tmpl: web::Data<Tera>, error_message: Option<String>) -> Result<HttpResponse, Error> {
    let mut ctx = Context::new();
    if let Some(error) = error_message {
        ctx.insert("error_message", &error);
    }
    let rendered = tmpl.render("register.html", &ctx)
        .map_err(|_| ErrorInternalServerError("Template rendering error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

fn is_valid_email(email: &str) -> bool {
    let re = Regex::new(r"^[\w\-\.]+@([\w\-]+\.)+[a-zA-Z]{2,}$").unwrap();
    re.is_match(email)
}

pub async fn process_register_form(
    form: web::Form<RegisterFormData>, 
    pool: web::Data<MySqlPool>, 
    tmpl: web::Data<Tera>
) -> Result<HttpResponse, Error> {
    // Validate username
    if form.username.len() < 3 || form.username.len() > 20 {
        return show_register_form(tmpl.clone(), Some("Username must be between 3 and 20 characters long.".to_string())).await;
    }
    if form.username.contains(' ') {
        return show_register_form(tmpl.clone(), Some("Username cannot contain spaces.".to_string())).await;
    }

    // Validate password
    if form.password.len() < 8 {
        return show_register_form(tmpl.clone(), Some("Password must be at least 8 characters long.".to_string())).await;
    }
    if form.password != form.password_confirm {
        return show_register_form(tmpl.clone(), Some("Password and confirmation do not match.".to_string())).await;
    }

    // Validate email format
    if !is_valid_email(&form.email) {
        return show_register_form(tmpl.clone(), Some("Invalid email format.".to_string())).await;
    }

    // Check if username exists
    let username_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM users WHERE username = ?",
        form.username
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(|err| {
        eprintln!("Database query error: {:?}", err);
        ErrorInternalServerError("Database query error")
    })?;
    if username_count > 0 {
        return show_register_form(tmpl.clone(), Some("Username is already taken.".to_string())).await;
    }

    // Check if email exists
    let email_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM users WHERE email = ?",
        form.email
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(|err| {
        eprintln!("Database query error: {:?}", err);
        ErrorInternalServerError("Database query error")
    })?;
    if email_count > 0 {
        return show_register_form(tmpl.clone(), Some("Email is already registered.".to_string())).await;
    }

    let hashed_password = hash(&form.password, DEFAULT_COST)
        .map_err(|_| ErrorInternalServerError("Error hashing password"))?;
    
    sqlx::query!(
        "INSERT INTO users (username, email, password) VALUES (?, ?, ?)",
        form.username,
        form.email,
        hashed_password
    )
    .execute(pool.get_ref())
    .await
    .map_err(|err| {
        eprintln!("Database insertion error: {:?}", err);
        ErrorInternalServerError("Failed to register user.")
    })?;

    Ok(HttpResponse::SeeOther().append_header(("Location", "/auth/login")).finish())
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/auth/register")
        .route(web::get().to(show_register_form))
        .route(web::post().to(process_register_form))
    )
    .service(web::resource("/auth/login")
        .route(web::get().to(show_login_form))
        .route(web::post().to(process_login_form))
    )
    .service(web::resource("/auth/logout")
        .route(web::post().to(logout))
    );
}
