use chrono::Duration;
use regex::*;

use super::*;

pub fn parse_duration(s: &str) -> CliResult<Duration> {
    lazy_static! {
        static ref RE_PARSE_DURATION: Regex =
            Regex::new(r"((?P<w>\d+).*(w))?((?P<d>\d+).*(d))?((?P<h>\d+).*(h))?((?P<m>\d+).*(m))?((?P<s>\d+).*(s))?").unwrap();
    }

    let s = s.replace(" ", "");

    let caps = RE_PARSE_DURATION.captures(&s);
    if !caps.is_some() {
        return Err(CliError::Parse {
            message: "unable to parse duration".to_string(),
        });
    }
    let caps = caps.unwrap();

    let mut dur = Duration::zero();

    if let Some(v) = caps.name("w") {
        dur = dur + Duration::weeks(v.as_str().parse().unwrap());
    }
    if let Some(v) = caps.name("d") {
        dur = dur + Duration::days(v.as_str().parse().unwrap());
    }
    if let Some(v) = caps.name("h") {
        dur = dur + Duration::hours(v.as_str().parse().unwrap());
    }
    if let Some(v) = caps.name("m") {
        dur = dur + Duration::minutes(v.as_str().parse().unwrap());
    }
    if let Some(v) = caps.name("s") {
        dur = dur + Duration::seconds(v.as_str().parse().unwrap());
    }

    return Ok(dur);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let caps = parse_duration("1w2d3h4m5s").unwrap();
        let dur = Duration::weeks(1) + Duration::days(2) + Duration::hours(3) + 
            Duration::minutes(4) + Duration::seconds(5);

        assert_eq!(caps, dur);
    }
}
