mod calendly;
mod google_cal;
pub mod libs;

use actix_files as fs;
use actix_web::{get, web, App, HttpServer, Responder, Result};
use chrono::{DateTime, Local};
use env_logger;
use futures::future::join_all;
use log::{info, LevelFilter};
use reqwest;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::Instant;
use std::{env, error::Error};

enum CalendarType {
    Calendly,
    Google,
}

fn get_calendar_type(scheduling_link: &str) -> CalendarType {
    match scheduling_link.contains("calendly") {
        true => CalendarType::Calendly,
        false => CalendarType::Google,
    }
}

async fn fetch_availability(
    client: &reqwest::Client,
    scheduling_link: &str,
    start_day: &str,
    end_day: &str,
) -> Result<FindTimeResponse, Box<dyn Error>> {
    match get_calendar_type(scheduling_link) {
        CalendarType::Calendly => {
            calendly::fetch(client, scheduling_link, start_day, end_day).await
        }
        CalendarType::Google => {
            google_cal::fetch(client, scheduling_link, start_day, end_day).await
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FindTimeRequest {
    q: String,
    start: String,
    end: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    title: String,
    start: i64,
    end: i64,
    color: String,
}
type FindTimeResponse = Vec<Event>;

#[get("/findtime")]
async fn findtime(
    r: web::Query<FindTimeRequest>,
    client: web::Data<reqwest::Client>,
) -> Result<impl Responder> {
    let start_time = Instant::now();
    info!("Findtime start");
    let start_day = DateTime::parse_from_rfc3339(r.start.as_str())
        .map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?
        .format("%Y-%m-%d")
        .to_string();
    let end_day = DateTime::parse_from_rfc3339(r.end.as_str())
        .map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?
        .format("%Y-%m-%d")
        .to_string();
    info!("start: {}, end: {}", start_day, end_day);

    let scheduling_links = &r.q.split(',').collect::<Vec<&str>>();

    let futures = scheduling_links
        .iter()
        .map(|scheduling_link| fetch_availability(&client, scheduling_link, &start_day, &end_day));
    let responses: Vec<Event> = join_all(futures)
        .await
        .into_iter()
        .filter_map(|r| {
            r.map_err(|e| info!("Error while fetching {}", e.to_string()))
                .ok()
        })
        .enumerate()
        .map(|(i, x)| attach_color(x, i))
        .flatten()
        .collect();

    let elapsed_time = start_time.elapsed();
    info!("Findtime end, elapsed time: {:?}", elapsed_time);

    Ok(web::Json(responses))
}

fn attach_color(events: FindTimeResponse, i: usize) -> FindTimeResponse {
    const COLORS: [&str; 4] = ["#9D68AF", "#32B579", "#E67B73", "#E4C441"];

    events
        .into_iter()
        .map(|event| Event {
            color: COLORS[i % COLORS.len()].to_string(),
            ..event
        })
        .collect()
}

#[get("/")]
async fn index() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("index.html")?)
}
#[get("/main.js")]
async fn js() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("main.js")?)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    info!("Main started");

    let address = env::var("ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    info!("Listening on {}:{}", address, port);

    HttpServer::new(|| {
        App::new()
            .service((index, js, findtime))
            .app_data(web::Data::new(
                reqwest::ClientBuilder::new()
                    .cookie_store(true)
                    .build()
                    .unwrap(),
            ))
    })
    .bind(format!("{}:{}", address, port))?
    .run()
    .await
}
