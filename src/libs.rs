use crate::Event;

pub fn dedup_events(events: &mut Vec<Event>) {
    // Sort events by start time, and end time
    events.sort_by(|a, b| (a.start, a.end).cmp(&(b.start, b.end)));

    // Replace the original events with an empty vector and consume it
    let mut merged_events: Vec<Event> = Vec::new();
    std::mem::take(events).into_iter().for_each(|event| {
        match merged_events.last_mut() {
            // If the last event in `merged_events` overlaps with the current event, merge them
            Some(last) if last.end >= event.start => {
                last.end = std::cmp::max(last.end, event.end);
            }
            // Otherwise, push the current event to `merged_events`
            _ => merged_events.push(event),
        }
    });

    // Replace the original events with the merged ones
    *events = merged_events;
}
