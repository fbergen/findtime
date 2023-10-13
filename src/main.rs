use actix_files as fs;
use actix_web::{
    get, middleware::Logger, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use futures::future::join_all;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct Day {
    date: String,
    status: String,
    spots: Vec<Spot>,
    invitee_events: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Spot {
    status: String,
    start_time: String,
    invitees_remaining: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CalendyReponse {
    availability_timezone: String,
    days: Vec<Day>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AvailabilityResponse {
    scheduling_link: String,
    duration: String,
    availability_timezone: String,
    days: Vec<Day>,
}

async fn get_uuid(scheduling_link: &str) -> Result<(String, String), Box<dyn Error>> {
    let response = reqwest::get(scheduling_link).await?.text().await?;

    let re = Regex::new(r#""uuid":"([^"]+)""#).unwrap();
    let uuid = match re.captures(response.as_str()) {
        Some(captures) => captures[1].to_string(),
        None => return Err(Box::<dyn std::error::Error>::from("UUID not found")),
    };

    let re2 = Regex::new(r#""duration":(\d+)"#).unwrap();
    let duration = match re2.captures(response.as_str()) {
        Some(captures) => captures[1].to_string(),
        None => return Err(Box::<dyn std::error::Error>::from("duration not found")),
    };

    Ok((uuid, duration))
}

async fn fetch_availability(
    scheduling_link: &str,
    start_day: &str,
    end_day: &str,
) -> Result<AvailabilityResponse, Box<dyn Error>> {
    let (uuid, duration) = get_uuid(scheduling_link).await?;
    println!("UUID: {}", uuid);
    let url: String = format!("https://calendly.com/api/booking/event_types/{}/calendar/range?timezone=Europe%2FBerlin&diagnostics=false&range_start={}&range_end={}", uuid, start_day, end_day);

    let response = reqwest::get(url).await?;

    let cresponse = response.json::<CalendyReponse>().await?;
    Ok(AvailabilityResponse {
        scheduling_link: scheduling_link.to_string(),
        duration,
        availability_timezone: cresponse.availability_timezone,
        days: cresponse.days,
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct FindTimeRequest {
    q: String,
    start: String,
    end: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    title: String,
    start: String,
    end: String,
    color: String,
}
type FindTimeResponse = Vec<Event>;

fn calendly_to_events(c: AvailabilityResponse, i: usize) -> Vec<Event> {
    const COLORS: [&str; 4] = ["blue", "red", "green", "yellow"];

    let mut events: FindTimeResponse = Vec::new();
    for day in &c.days {
        for spot in &day.spots {
            if spot.status == "available" {
                events.push(Event {
                    title: "title".to_string(),
                    start: spot.start_time.clone(),
                    end: spot.start_time.clone(),
                    color: COLORS[i % COLORS.len()].to_string(),
                });
            }
        }
    }
    events
}

#[get("/findtime")]
async fn findtime(r: web::Query<FindTimeRequest>) -> Result<impl Responder> {
    let start_day = DateTime::parse_from_rfc3339(r.start.as_str())
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let end_day = DateTime::parse_from_rfc3339(r.end.as_str())
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    println!("start: {}, end: {}", start_day, end_day);

    let scheduling_links = &r.q.split(',').collect::<Vec<&str>>();

    let futures = scheduling_links
        .iter()
        .map(|scheduling_link| fetch_availability(scheduling_link, &start_day, &end_day));
    let responses: Vec<Event> = join_all(futures)
        .await
        .into_iter()
        .map(Result::unwrap)
        .enumerate()
        .map(|(i, x)| calendly_to_events(x, i))
        .flatten()
        .collect();

    Ok(web::Json(responses))
}

#[get("/")]
async fn index() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("index.html")?)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service((index, findtime)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
