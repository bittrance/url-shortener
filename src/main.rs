use actix_web::{
    get,
    http::header,
    post,
    web::{self, Json},
    App, HttpResponse, HttpServer, Responder,
};
use dashmap::DashMap;
use futures::{future, FutureExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{
    error::Error,
    sync::{atomic::AtomicI64, atomic::Ordering, Arc},
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::time;

type Cache = DashMap<String, Entry>;

struct Config {
    bind_address: String,
    postgres_host: String,
    postgres_port: u16,
    postgres_db: String,
    postgres_username: String,
    postgres_password: String,
}

struct State {
    cache: Cache,
    pool: Pool<Postgres>,
}

struct Entry {
    target: String,
    counter: AtomicI64,
}

impl From<Registration> for Entry {
    fn from(reg: Registration) -> Self {
        Entry {
            target: reg.target,
            counter: AtomicI64::new(0),
        }
    }
}

#[get("/{token}")]
async fn redirect(token: web::Path<String>, state: web::Data<Arc<State>>) -> impl Responder {
    if let Some(entry) = state.cache.get(token.as_str()) {
        entry
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        HttpResponse::TemporaryRedirect()
            .append_header((header::LOCATION, entry.target.to_owned()))
            .finish()
    } else {
        let result: Result<Option<(String,)>, sqlx::Error> =
            sqlx::query_as("SELECT target FROM tokens WHERE token = $1")
                .bind(token.as_str())
                .fetch_optional(&state.pool)
                .await;
        match result {
            Ok(Some((target,))) => {
                state.cache.insert(
                    token.to_owned(),
                    Entry {
                        target: target.clone(),
                        counter: AtomicI64::new(1),
                    },
                );
                HttpResponse::TemporaryRedirect()
                    .append_header((header::LOCATION, target))
                    .finish()
            }
            Ok(None) => HttpResponse::NotFound().finish(),
            Err(err) => {
                eprintln!("Error: {}", err);
                HttpResponse::InternalServerError().finish()
            }
        }
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
async fn create(registration: Json<Registration>, state: web::Data<Arc<State>>) -> impl Responder {
    let token = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>()
        .to_lowercase();
    if state.cache.get(token.as_str()).is_some() {
        return HttpResponse::InternalServerError().finish();
    }
    let res: Result<Option<(String,)>, sqlx::Error> =
        sqlx::query_as("INSERT INTO tokens (token, target) VALUES ($1, $2)")
            .bind(token.as_str())
            .bind(registration.target.as_str())
            .fetch_optional(&state.pool)
            .await;
    if let Err(err) = res {
        eprint!("Error writing to db: {}", err);
        return HttpResponse::InternalServerError().finish();
    }
    let target = registration.target.clone();
    let entry = Entry::from(registration.into_inner());
    state.cache.insert(token.clone(), entry);
    HttpResponse::Created().json(RegistrationResponse { token, target })
}

fn erase_err<T: 'static + Error>(err: T) -> Box<dyn Error> {
    Box::new(err)
}

async fn setup_cache_maintainer(
    state: Arc<State>,
) -> Result<std::convert::Infallible, std::io::Error> {
    loop {
        time::sleep(Duration::from_millis(10000)).await;
        for entry in state.cache.iter() {
            let count = entry.counter.load(Ordering::Relaxed);
            if count == 0 {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)]
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let res: Result<Option<(String,)>, sqlx::Error> = sqlx::query_as(
                "INSERT INTO counts (token, target, timestamp, count) VALUES ($1, $2, $3, $4)",
            )
            .bind(entry.key())
            .bind(&entry.target)
            .bind(ts)
            .bind(count)
            .fetch_optional(&state.pool)
            .await;
            if let Err(err) = res {
                eprint!("Error writing to db: {}", err);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
            }
            entry
                .counter
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v - count))
                .unwrap();
        }
    }
}

#[allow(clippy::or_fun_call)]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Config {
        bind_address: std::env::var("BIND_ADDRESS").unwrap_or("0.0.0.0".to_owned()),
        postgres_host: std::env::var("POSTGRES_HOST").unwrap_or("127.0.0.1".to_owned()),
        postgres_port: std::env::var("POSTGRES_PORT")
            .map_err(erase_err)
            .and_then(|p| p.parse().map_err(erase_err))
            .unwrap_or(5432),
        postgres_db: std::env::var("POSTGRES_DB").unwrap_or("urlshortener".to_owned()),
        postgres_username: std::env::var("POSTGRES_USERNAME").unwrap_or("url-shortener".to_owned()),
        postgres_password: std::env::var("POSTGRES_PASSWORD")
            .expect("Please set env var POSTGRES_PASSWORD"),
    };
    let cache: Cache = DashMap::with_capacity(1000);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&format!(
            "postgres://{}:{}@{}:{}/{}",
            config.postgres_username,
            config.postgres_password,
            config.postgres_host,
            config.postgres_port,
            config.postgres_db
        ))
        .await
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    let state = Arc::new(State { cache, pool });
    future::try_select(
        Box::pin(setup_cache_maintainer(Arc::clone(&state))),
        HttpServer::new(move || {
            let data = web::Data::new(Arc::clone(&state));
            App::new().app_data(data).service(redirect).service(create)
        })
        .bind((config.bind_address, 8080))?
        .run(),
    )
    .map(|_| Ok(()))
    .await
}
