use crate::Event;
use actix_web::Result;
use chrono::{DateTime, Local};
use log::info;
use rand;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Instant;

use crate::FindTimeResponse;

#[derive(Debug, Serialize, Deserialize)]
struct Spot {
    status: String,
    start_time: String,
    invitees_remaining: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Day {
    date: String,
    status: String,
    spots: Vec<Spot>,
    invitee_events: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailabilityResponse {
    scheduling_link: String,
    duration: i64,
    availability_timezone: String,
    days: Vec<Day>,
    title: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CalendyReponse {
    availability_timezone: String,
    days: Vec<Day>,
}

async fn get_info(
    client: &reqwest::Client,
    scheduling_link: &str,
) -> Result<(String, String, String), Box<dyn Error>> {
    let response = client.get(scheduling_link).send().await?.text().await?;

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

pub async fn fetch(
    client: &reqwest::Client,
    scheduling_link: &str,
    start_day: &str,
    end_day: &str,
) -> Result<FindTimeResponse, Box<dyn Error>> {
    let start_time = Instant::now();
    let trace = rand::random::<u64>();
    info!("{} Fetch availability start", trace);
    let (uuid, duration, name) = get_info(&client, scheduling_link).await?;

    let elapsed_time = start_time.elapsed();
    info!("{} UUID: {} elapsed time: {:?}", trace, name, elapsed_time);
    let url: String = format!("https://calendly.com/api/booking/event_types/{}/calendar/range?timezone=Europe%2FBerlin&diagnostics=false&range_start={}&range_end={}", uuid, start_day, end_day);

    let response = client.get(url).send().await?;

    let cresponse = response.json::<CalendyReponse>().await?;
    let elapsed_time = start_time.elapsed();
    info!(
        "{} Fetch availability end, elapsed time: {:?}",
        trace, elapsed_time
    );

    let resp = AvailabilityResponse {
        scheduling_link: scheduling_link.to_string(),
        duration: duration.parse::<i64>().unwrap() * 60 * 1000,
        availability_timezone: cresponse.availability_timezone,
        days: cresponse.days,
        title: name.clone(),
    };

    Ok(calendly_to_events(resp))
}

fn calendly_to_events(c: AvailabilityResponse) -> Vec<Event> {
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
                    color: "TO_BE_FILLED".to_string(),
                });
            }
        }
    }

    dedup_events(&mut events);
    events
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
