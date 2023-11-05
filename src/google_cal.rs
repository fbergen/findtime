use crate::{libs::dedup_events, Event, FindTimeResponse};
use actix_web::Result;
use chrono::NaiveDate;
use log::info;
use rand;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
struct Slot(Vec<String>, i64);

type GoogleResponse = Vec<Vec<Vec<Slot>>>;

async fn get_info(
    client: &reqwest::Client,
    scheduling_link: &str,
) -> Result<(String, String), Box<dyn Error>> {
    let response = client.get(scheduling_link).send().await?.text().await?;

    let re = Regex::new(r#"\\"([^\\]+)\\",\\"https://calendar"#).unwrap();
    let uuid = match re.captures(response.as_str()) {
        Some(captures) => captures[1].to_string(),
        None => return Err(Box::<dyn std::error::Error>::from("UUID not found")),
    };

    // TODO use a html parser here
    let re2 = Regex::new(r#"property="og:title" content="([^"]+)"#).unwrap();
    let name = match re2.captures(response.as_str()) {
        Some(captures) => captures[1].to_string(),
        None => return Err(Box::<dyn std::error::Error>::from("name not found")),
    };

    Ok((uuid, name))
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
    let (uuid, name) = get_info(&client, scheduling_link).await?;

    let elapsed_time = start_time.elapsed();
    info!(
        "{} UUID: {} name: {} elapsed time: {:?}",
        trace, uuid, name, elapsed_time
    );

    let cal_id = scheduling_link.split("/").last().unwrap();
    let body = format!(
        "[null,null,\"{}\",null,[[{}],[{}]]]",
        cal_id,
        NaiveDate::parse_from_str(start_day, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .timestamp(),
        NaiveDate::parse_from_str(end_day, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .timestamp()
    );

    use reqwest::header;
    let mut headers = header::HeaderMap::new();
    headers.insert("content-type", "application/x-www-form-urlencoded".parse()?);
    headers.insert("origin", "https://calendar.google.com".parse()?);

    let url: String = format!("https://calendar-pa.clients6.google.com/$rpc/google.internal.calendar.v1.AppointmentBookingService/ListAvailableSlots?%24httpHeaders=X-Goog-Api-Key%3A{}%0D%0AX-Goog-AuthUser%3A0%0D%0AContent-Type%3Aapplication%2Fjson%2Bprotobuf", uuid);
    let response = client.post(url).body(body).headers(headers).send().await?;

    let gresponse = response.json::<GoogleResponse>().await?;
    // info!("{} {:?}", trace, gresponse);
    let elapsed_time = start_time.elapsed();
    info!(
        "{} Fetch availability end, elapsed time: {:?}",
        trace, elapsed_time
    );

    let mut events: Vec<Event> = gresponse
        .into_iter()
        .flatten()
        .flatten()
        .map(|x| Event {
            title: name.clone(),
            start: x.0.first().unwrap().parse::<i64>().unwrap() * 1000,
            end: (x.0.first().unwrap().parse::<i64>().unwrap() + x.1 * 60) * 1000,
            color: "TO BE FILLED".to_string(),
        })
        .collect();

    dedup_events(&mut events);
    Ok(events)
}
