use actix_web::{web, HttpResponse, Error, Result};
use tera::{Tera, Context};
use actix_web::error::ErrorInternalServerError;
use actix_session::Session;

pub async fn show_dashboard(session: Session, tmpl: web::Data<Tera>) -> Result<HttpResponse, Error> {
    if let Some(username) = session.get::<String>("username").unwrap_or(None) {
        let mut ctx = Context::new();
        ctx.insert("username", &username);
        ctx.insert("welcome_message", &format!("Welcome to your dashboard, {}!", username));
        
        let s = tmpl.render("dashboard.html", &ctx)
            .map_err(|err| {
                eprintln!("Template rendering error: {:?}", err);
                ErrorInternalServerError("Template rendering error")
            })?;
        return Ok(HttpResponse::Ok().content_type("text/html").body(s));
    } else {
        return Ok(HttpResponse::SeeOther()
            .append_header(("Location", "/auth/login"))
            .finish());
    }
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/dashboard")
        .route(web::get().to(show_dashboard))
    );
}
