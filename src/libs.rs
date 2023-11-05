use crate::Event;

pub fn dedup_events(events: &mut Vec<Event>) {
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
