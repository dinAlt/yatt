use std::convert::{TryFrom, TryInto};

use chrono::prelude::*;
use chrono::Duration;
use regex::*;

use super::*;

#[derive(Default, Debug)]
pub struct PeriodOpts {
    pub week_starts_from_sunday: bool,
}

pub fn parse_period(
    s: &str,
    opts: &PeriodOpts,
) -> CliResult<(DateTime<Utc>, DateTime<Utc>)> {
    let parts: Vec<&str> = s.split("::").collect();

    match parts.len() {
        1 => {
            let d = try_parse_date_time(parts[0]);
            if d.is_ok() {
                return Ok((d.unwrap(), Utc::now()));
            }
            try_parse_period(parts[0], opts)
        }
        2 => {
            let mut end = try_parse_date_time(parts[1])?;
            if end.date().and_hms(0, 0, 0) == end {
                end = end + Duration::days(1);
            }
            Ok((try_parse_date_time(parts[0])?, end))
        }
        _ => Err(CliError::Parse {
            message: format!(
                r#"can't parse period from string "{}"#,
                s
            ),
        }),
    }
}

fn try_parse_date_time(s: &str) -> CliResult<DateTime<Utc>> {
    let parts: Vec<&str> = s.split(' ').collect();
    match parts.len() {
        1 => {
            let d = try_parse_date_part(s);
            if d.is_ok() {
                return Ok(d.unwrap().and_hms(0, 0, 0).into());
            };
            try_parse_time_part(s, Local::today())
        }
        2 => {
            let d = try_parse_date_part(parts[0])?;
            try_parse_time_part(parts[1], d)
        }
        _ => Err(CliError::Parse {
            message: format!(
                r#"can't parse date and time from string "{}""#,
                s
            ),
        }),
    }
}

fn try_parse_date_part(s: &str) -> CliResult<Date<Local>> {
    lazy_static! {
        static ref RE_PARSE_DATE_PART: Regex = Regex::new(
            r"^((?P<y>\d{4})-)?(?P<m>\d{1,2})-(?P<d>\d{1,2})|(?P<dr>\d{1,2})\.(?P<mr>\d{1,2})(\.(?P<yr>\d{4}))?$"
        )
        .unwrap();
    }

    let caps = RE_PARSE_DATE_PART.captures(s);
    if caps.is_none() {
        return Err(CliError::Parse {
            message: format!(
                r#"can't parse date from string "{}""#,
                s
            ),
        });
    }

    let caps = caps.unwrap();

    let day: u32 = caps
        .name("d")
        .unwrap_or_else(|| caps.name("dr").unwrap())
        .as_str()
        .parse()
        .map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse date from string "{}""#,
                s
            ),
        })?;
    let month: u32 = caps
        .name("m")
        .unwrap_or_else(|| caps.name("mr").unwrap())
        .as_str()
        .parse()
        .map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse date from string "{}""#,
                s
            ),
        })?;
    let year: i32 = if let Some(y) = caps.name("y") {
        y.as_str().parse().map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse date from string "{}""#,
                s
            ),
        })?
    } else if let Some(y) = caps.name("yr") {
        y.as_str().parse().map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse date from string "{}""#,
                s
            ),
        })?
    } else {
        Local::today().year()
    };
    Ok(Local.ymd(year, month, day))
}

fn try_parse_time_part(
    s: &str,
    d: Date<Local>,
) -> CliResult<DateTime<Utc>> {
    lazy_static! {
        static ref RE_PARSE_TIME_PART: Regex = Regex::new(
            r"^(?P<h>\d{1,2}):(?P<m>\d{1,2})(:(?P<s>\d{1,2}))?$"
        )
        .unwrap();
    }

    let caps = RE_PARSE_TIME_PART.captures(s);

    if caps.is_none() {
        return Err(CliError::Parse {
            message: format!(
                r#"can't parse time from string "{}""#,
                s
            ),
        });
    }

    let caps = caps.unwrap();

    let hour: u32 =
        caps["h"].parse().map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse time from string "{}""#,
                s
            ),
        })?;
    let minute: u32 =
        caps["m"].parse().map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse time from string "{}""#,
                s
            ),
        })?;
    let second: u32 = if let Some(sec) = caps.name("s") {
        sec.as_str().parse().map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse time from string "{}""#,
                s
            ),
        })?
    } else {
        0
    };

    Ok(d.and_hms(hour, minute, second).into())
}

fn try_parse_period(
    s: &str,
    opts: &PeriodOpts,
) -> CliResult<(DateTime<Utc>, DateTime<Utc>)> {
    if s.starts_with('l') && s.len() < 2 {
        return Err(CliError::Parse {
            message: format!(
                r#"can't parse last from string "{}""#,
                s
            ),
        });
    }

    lazy_static! {
        static ref RE_PARSE_DURATION: Regex =
            Regex::new(r"^(?P<o>[lp])?(?P<n>\d+)?(?P<p>[ymwdh])$")
                .unwrap();
    }

    let caps = RE_PARSE_DURATION.captures(s);
    if caps.is_none() {
        return Err(CliError::Parse {
            message: format!(
                r#"can't parse last from string "{}""#,
                s
            ),
        });
    }
    let caps = caps.unwrap();
    let o: u32 = if let Some(o) = caps.name("o") {
        if o.as_str() == "p" {
            1
        } else {
            0
        }
    } else {
        0
    };
    let p = &caps["p"];
    let n: u32 = if let Some(n) = caps.name("n") {
        n.as_str().parse().map_err(|_| CliError::Parse {
            message: format!(
                r#"can't parse last from string "{}""#,
                s
            ),
        })?
    } else {
        1
    };
    let now = Local::now();
    let (begin, end) = match p {
    "y" => (
      Local
        .ymd(
          now.year()
            - i32::try_from(o + n - 1).map_err(|_| {
              CliError::Parse {
                message: format!(
                  r#"can't parse last from string "{}""#,
                  s
                ),
              }
            })?,
          1,
          1,
        )
        .and_hms(0, 0, 0),
      if o == 1 {
        Local.ymd(now.year(), 1, 1).and_hms(0, 0, 0)
      } else {
        now
      },
    ),
    "m" => {
      let mo = o + n - 1;
      let mut yo = mo / 12;
      let mut mo = mo - yo * 12;
      let mut month = if mo > now.month() {
        yo += 1;
        mo -= now.month();
        12 - mo
      } else {
        now.month() - mo
      };
      let mut year = now.year()
        - i32::try_from(yo).map_err(|_| CliError::Parse {
          message: format!(r#"can't parse last from string "{}""#, s),
        })?;
      if month == 0 {
        month = 12;
        year -= 1;
      };
      (
        Local.ymd(year, month, 1).and_hms(0, 0, 0),
        if o == 1 {
          Local.ymd(now.year(), now.month(), 1).and_hms(0, 0, 0)
        } else {
          now
        },
      )
    }
    "w" => {
      let first_dow = (if opts.week_starts_from_sunday {
        now
          - Duration::days(
            now.weekday().number_from_sunday().try_into().unwrap(),
          )
      } else {
        now
          - Duration::days(
            now.weekday().number_from_monday().try_into().unwrap(),
          )
      })
      .date()
        + Duration::days(1);
      let wo = o + n - 1;
      (
        (first_dow - Duration::weeks(wo.try_into().unwrap()))
          .and_hms(0, 0, 0),
        if o == 1 {
          first_dow.and_hms(0, 0, 0)
        } else {
          now
        },
      )
    }
    "d" => {
      let today = Local::today();
      let dayo = o + n - 1;
      (
        (today - Duration::days(dayo.try_into().unwrap()))
          .and_hms(0, 0, 0),
        if o == 1 { today.and_hms(0, 0, 0) } else { now },
      )
    }
    "h" => {
      let hour = Local::today().and_hms(now.hour(), 0, 0);
      let ho = o + n - 1;
      (
        (hour - Duration::hours(ho.try_into().unwrap())),
        if o == 1 { hour } else { now },
      )
    }
    _ => unreachable!(),
  };

    Ok((begin.into(), end.into()))
}
