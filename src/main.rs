use actix_files as fs;
use actix_web::{get, web, App, HttpServer, Responder, Result};
use chrono::DateTime;
use futures::future::join_all;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::{env, error::Error};

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
    duration: i64,
    availability_timezone: String,
    days: Vec<Day>,
    title: String,
}

async fn get_uuid(scheduling_link: &str) -> Result<(String, String, String), Box<dyn Error>> {
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

    let re3 = Regex::new(r#""name":"([^"]+)""#).unwrap();
    let name = match re3.captures(response.as_str()) {
        Some(captures) => captures[1].to_string(),
        None => return Err(Box::<dyn std::error::Error>::from("name not found")),
    };

    Ok((uuid, duration, name))
}

async fn fetch_availability(
    scheduling_link: &str,
    start_day: &str,
    end_day: &str,
) -> Result<AvailabilityResponse, Box<dyn Error>> {
    let (uuid, duration, name) = get_uuid(scheduling_link).await?;
    println!("UUID: {}", uuid);
    let url: String = format!("https://calendly.com/api/booking/event_types/{}/calendar/range?timezone=Europe%2FBerlin&diagnostics=false&range_start={}&range_end={}", uuid, start_day, end_day);

    let response = reqwest::get(url).await?;

    let cresponse = response.json::<CalendyReponse>().await?;
    Ok(AvailabilityResponse {
        scheduling_link: scheduling_link.to_string(),
        duration: duration.parse::<i64>().unwrap() * 60 * 1000,
        availability_timezone: cresponse.availability_timezone,
        days: cresponse.days,
        title: name.clone(),
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
    start: i64,
    end: i64,
    color: String,
}
type FindTimeResponse = Vec<Event>;

fn calendly_to_events(c: AvailabilityResponse, i: usize) -> Vec<Event> {
    const COLORS: [&str; 4] = ["#9D68AF", "#32B579", "#E67B73", "#E4C441"];

    let mut events: FindTimeResponse = Vec::new();
    for day in &c.days {
        for spot in &day.spots {
            if spot.status == "available" {
                let start_time = DateTime::parse_from_rfc3339(spot.start_time.as_str())
                    .unwrap()
                    .timestamp_millis();
                let end_time = start_time + c.duration;
                events.push(Event {
                    title: c.title.clone(),
                    start: start_time,
                    end: end_time,
                    color: COLORS[i % COLORS.len()].to_string(),
                });
            }
        }
    }

    dedup_events(&mut events);
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

fn dedup_events(events: &mut Vec<Event>) {
    events.sort_by(|a, b| (a.start, a.end).cmp(&(b.start, b.end)));
    let mut it: i64 = 0;

    if events.len() < 2 {
        return;
    }

    while (it as usize) < events.len() - 2 {
        let i = it as usize;
        if events[i].end >= events[i + 1].start || events[i].start == events[i + 1].start {
            events[i].end = events[i + 1].end;
            events.remove(i + 1);
            it -= 1;
        }
        it += 1;
    }
}

#[get("/")]
async fn index() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("index.html")?)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let address = env::var("ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    println!("Listening on {}:{}", address, port);

    HttpServer::new(|| App::new().service((index, findtime)))
        .bind(format!("{}:{}", address, port))?
        .run()
        .await
}
