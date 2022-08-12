use actix_web::{
    get,
    http::header,
    post,
    web::{self, Json},
    App, HttpResponse, HttpServer, Responder,
};
use dashmap::DashMap;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::AtomicU64};

type Cache = Arc<DashMap<String, Entry>>;

struct Config {
    bind_address: String,
}

struct Entry {
    target: String,
    counter: AtomicU64,
}

impl From<Registration> for Entry {
    fn from(reg: Registration) -> Self {
        Entry { target: reg.target, counter: AtomicU64::new(0) }
    }
}

#[get("/{token}")]
async fn redirect(
    token: web::Path<String>,
    store: web::Data<Cache>,
) -> impl Responder {
    if let Some(entry) = store.get(token.as_str()) {
        entry.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        HttpResponse::TemporaryRedirect()
            .append_header((header::LOCATION, entry.target.to_owned()))
            .finish()
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[derive(Deserialize)]
struct Registration {
    target: String,
}

#[derive(Serialize)]
struct RegistrationResponse {
    token: String,
    target: String,
}

#[post("/admin/tokens")]
async fn create(
    registration: Json<Registration>,
    store: web::Data<Cache>,
) -> impl Responder {
    let token = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>()
        .to_lowercase();
    if let Some(_) = store.get(token.as_str()) {
        return HttpResponse::InternalServerError().finish();
    }
    let target = registration.target.clone();
    let entry = Entry::from(registration.into_inner());
    store.insert(token.clone(), entry);
    HttpResponse::Created().json(RegistrationResponse {
        token,
        target,
    })
}

#[allow(clippy::or_fun_call)]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Config {
        bind_address: std::env::var("BIND_ADDRESS").unwrap_or("0.0.0.0".to_owned())
    };
    let store: Cache = Arc::new(DashMap::with_capacity(1000));
    HttpServer::new(move || {
        let data = web::Data::new(Arc::clone(&store));
        App::new().app_data(data).service(redirect).service(create)
    })
    .bind((config.bind_address, 8080))?
    .run()
    .await
}
