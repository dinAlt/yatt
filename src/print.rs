use super::*;
use crate::report::*;
use self::core::*;

const DEFAULT_INTERVAL_INFO_TITLE: &str = "Interval info:";

#[derive(Debug, Clone)]
pub struct IntervalData<'a> {
    pub interval: &'a Interval,
    pub task: &'a [Node],
    pub title: &'a str,
}

impl IntervalData<'_> {
    pub fn default_title() -> &'static str {
        DEFAULT_INTERVAL_INFO_TITLE
    }
}

impl ToString for IntervalData<'_> {
    fn to_string(&self) -> String {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct IntervalCmdData<'a> {
    pub cmd_text: &'a str,
    pub interval: IntervalData<'a>,
}

#[derive(Debug, Clone)]
pub struct IntervalError<'a> {
    pub err_text: &'a str,
    pub interval: IntervalData<'a>,
}

pub trait Printer {
    fn interval_cmd(&self, d: &IntervalCmdData);
    fn error(&self, e: &str);
    fn interval_error(&self, d: &IntervalData, e: &str);
    fn cmd(&self, d: &str);
    fn report(&self, r: &Report);
    fn prompt(&self, p: &str);
}

pub trait Markdown {
    fn markdown(&self) -> String;
}

pub struct TermPrinter {
    style: AppStyle,
}

impl Default for TermPrinter {
    fn default() -> Self {
        TermPrinter {
            style: Default::default(),
        }
    }
}

impl Printer for TermPrinter {
    fn interval_cmd(&self, d: &IntervalCmdData) {
        self.cmd(d.cmd_text);
        println!();
        print_interval_info(&d.interval, &self.style.task);
    }
    fn error(&self, e: &str) {
        println!("Error: {}", &self.style.error.apply_to(e));
    }
    fn interval_error(&self, d: &IntervalData, e: &str) {
        self.error(e);
        println!();
        print_interval_info(d, &self.style.task);
    }
    fn cmd(&self, d: &str) {
        println!("{}", &self.style.cmd.apply_to(d));
    }
    fn report(&self, r: &Report) {
        println!("{}", self.style.report.text(&r.markdown(), self.style.screen_width));
    }
    fn prompt(&self, p: &str) {
        println!("{}", p);
    }
}

fn print_interval_info(d: &IntervalData, s: &TaskStyle) {
    println!("{}", d.title);
    print!("  Task: ");
    for (i, t) in d.task.iter().enumerate() {
        print!("{}", s.name.apply_to(&t.label));
        if i < d.task.len() - 1 {
            print!(" > ");
        }
    }
    println!();
    print!(
        "  Started: {}",
        s.start_time.apply_to(format_datetime(&d.interval.begin))
    );

    let dur = Utc::now() - d.interval.begin;

    if dur.num_seconds() > 2 {
        print!(" ({} ago)", s.time_span.apply_to(format_duration(&dur)));
    }

    if d.interval.end.is_some() {
        let e = d.interval.end.unwrap();
        print!("\n  Stopped: {}", s.end_time.apply_to(format_datetime(&e)));
        let dur = Utc::now() - e;
        if dur.num_seconds() > 2 {
            print!(" ({} ago)", s.time_span.apply_to(format_duration(&dur)));
        }
    }

    println!();
}
