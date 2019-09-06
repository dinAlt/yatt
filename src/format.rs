use chrono::prelude::*;
use chrono::Duration;

use crate::core::Node;

pub(crate) fn format_task_name(t: &[Node]) -> String {
    t.iter()
        .map(|n| n.label.clone())
        .collect::<Vec<String>>()
        .join(" -> ")
        .to_string()
}

#[derive(Debug, Default)]
pub struct DateTimeOpts {
    pub olways_long: bool,
    pub no_string_now: bool,
}

pub(crate) fn format_datetime(dt: &DateTime<Utc>) -> String {
    format_datetime_opts(dt, &DateTimeOpts::default())
}

pub(crate) fn format_datetime_opts(dt: &DateTime<Utc>, opts: &DateTimeOpts) -> String {
    let dt: DateTime<Local> = DateTime::from(*dt);
    let delta = Local::now() - dt;

    let pattern = if delta < Duration::seconds(2) && !opts.no_string_now {
        "just now"
    } else if dt.date() == Local::today() && !opts.olways_long {
        "%H:%M:%S"
    } else {
        "%Y-%m-%d %H:%M:%S"
    };

    dt.format(pattern).to_string()
}

#[allow(clippy::many_single_char_names)]
pub(crate) fn format_duration(dur: &Duration) -> String {
    let mut res = Vec::new();

    if dur.is_zero() {
        return "".to_string();
    }

    let (w, d, h, m, s) = (
        dur.num_weeks(),
        (*dur - Duration::weeks(dur.num_weeks())).num_days(),
        (*dur - Duration::days(dur.num_days())).num_hours(),
        (*dur - Duration::hours(dur.num_hours())).num_minutes(),
        (*dur - Duration::minutes(dur.num_minutes())).num_seconds(),
    );

    if w > 0 {
        res.push(format_duration_part(w, "week"));
    }
    if d > 0 {
        res.push(format_duration_part(d, "day"));
    }
    if h > 0 {
        res.push(format_duration_part(h, "hour"));
    }
    if m > 0 {
        res.push(format_duration_part(m, "minute"));
    }
    if s > 0 {
        res.push(format_duration_part(s, "second"));
    }
    let res: Vec<&String> = res.iter().take(3).collect();

    match res.len() {
        1 => res.first().unwrap().to_string(),
        2 => format!("{} and {}", res[0], res[1]),
        3 => format!("{}, {} and {}", res[0], res[1], res[2]),
        _ => "".to_string(),
    }
}

fn format_duration_part(p: i64, w: &str) -> String {
    let mut s = format! {"{} {}", p, w};
    if p > 1 {
        s.push('s');
    }
    s
}
