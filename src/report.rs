use crate::format::*;
use crate::print::Markdown;
use chrono::prelude::*;
use chrono::Duration;

#[derive(Default)]
pub struct Report {
    rows: Vec<Row>,
}

impl Report {
    pub fn new() -> Self {
        Report { rows: vec![] }
    }
    pub fn push(&mut self, r: impl Into<Row>) {
        self.rows.push(r.into());
    }
    pub fn rows(&self) -> &Vec<Row> {
        &self.rows
    }
}

pub enum Row {
    Header(String),
    Interval(DateTime<Utc>, DateTime<Utc>),
    Table(Vec<Cell>),
    TableHeader(Vec<String>),
    SubTotal(Vec<Cell>),
    Total(Vec<Cell>),
    Nested(Vec<Cell>),
    Span,
}

impl From<String> for Row {
    fn from(v: String) -> Self {
        Row::Header(v)
    }
}
impl From<&str> for Row {
    fn from(v: &str) -> Self {
        Row::Header(v.into())
    }
}
impl From<(DateTime<Utc>, DateTime<Utc>)> for Row {
    fn from(v: (DateTime<Utc>, DateTime<Utc>)) -> Self {
        Row::Interval(v.0, v.1)
    }
}
impl From<Vec<Cell>> for Row {
    fn from(v: Vec<Cell>) -> Self {
        Row::Table(v)
    }
}
impl From<Vec<String>> for Row {
    fn from(v: Vec<String>) -> Self {
        Row::TableHeader(v)
    }
}

pub enum Cell {
    Usize(usize),
    Isize(isize),
    String(String),
    DateTime(DateTime<Utc>),
    Duration(Duration),
    Nested(Box<Cell>, usize),
    Span,
}

impl From<usize> for Cell {
    fn from(v: usize) -> Self {
        Cell::Usize(v)
    }
}
impl From<isize> for Cell {
    fn from(v: isize) -> Self {
        Cell::Isize(v)
    }
}
impl From<String> for Cell {
    fn from(v: String) -> Self {
        Cell::String(v)
    }
}
impl From<&str> for Cell {
    fn from(v: &str) -> Self {
        Cell::String(v.into())
    }
}
impl From<DateTime<Utc>> for Cell {
    fn from(v: DateTime<Utc>) -> Self {
        Cell::DateTime(v)
    }
}
impl From<Duration> for Cell {
    fn from(v: Duration) -> Self {
        Cell::Duration(v)
    }
}

impl Markdown for Report {
    fn markdown(&self) -> String {
        self.rows()
            .iter()
            .map(|r| r.markdown())
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl Markdown for Row {
    fn markdown(&self) -> String {
        match self {
            Row::Header(v) => format!("Report: **{}**", v),
            Row::Interval(b, e) => {
                let dtopts = DateTimeOpts {
                    olways_long: true,
                    no_string_now: true,
                };
                format!(
                    "Period: *{} - {}*",
                    format_datetime_opts(b, &dtopts),
                    format_datetime_opts(e, &dtopts)
                )
            }
            Row::Table(v) => format_cells(&v),
            Row::TableHeader(v) => format_header(&v),
            Row::SubTotal(v) => format_subtotal(&v),
            Row::Total(v) => format_total(&v),
            Row::Nested(v) => v
                .iter()
                .map(|c| format!("|{}", c.markdown()))
                .collect::<Vec<String>>()
                .join(""),
            Row::Span => "|-".to_string(),
        }
    }
}

fn format_subtotal(cells: &[Cell]) -> String {
    let mut aligns = "|".to_string();
    let mut cols = "|total:".to_string();
    for c in cells {
        aligns += "|";
        cols += &format!("|*{}*", c.markdown());
    }
    format!("{}\n{}", aligns, cols)
}

fn format_total(cells: &[Cell]) -> String {
    let mut aligns = "|-:".to_string();
    let mut cols = "|Total".to_string();
    for c in cells {
        aligns += match c {
            Cell::String(_) | Cell::Duration(_) | Cell::DateTime(_) | Cell::Nested(_, _) => "|-",
            _ => "|-:",
        };
        cols += &format!("|**{}**\n|-", c.markdown());
    }
    format!("{}\n{}", aligns, cols)
}

fn format_header(cells: &[String]) -> String {
    let mut aligns = String::new();
    let mut cols = String::new();
    for c in cells {
        aligns += "|:-:";
        cols += &format!("|{}", c);
    }
    format!("{}\n{}", aligns, cols)
}

fn format_cells(cells: &[Cell]) -> String {
    let mut aligns = String::new();
    let mut cols = String::new();
    for c in cells {
        aligns += match c {
            Cell::String(_) | Cell::Duration(_) | Cell::DateTime(_) | Cell::Nested(_, _) => "|-",
            _ => "|-:",
        };
        cols += &format!("|{}", c.markdown());
    }
    format!("{}\n{}", aligns, cols)
}

impl Markdown for Cell {
    fn markdown(&self) -> String {
        match self {
            Cell::Usize(v) => v.to_string(),
            Cell::Isize(v) => v.to_string(),
            Cell::String(v) => v.to_owned(),
            Cell::DateTime(v) => format_datetime(&v),
            Cell::Duration(v) => format_duration(&v),
            Cell::Nested(v, p) => {
                let mut pad = "".to_string();
                let mark = match p {
                    0 => "",
                    1 => "\u{2023}",
                    _ => "\u{2219}",
                };
                for _ in 0..*p {
                    pad += "  ";
                }
                format!("`{}{} {}`", pad, mark, v.markdown())
            }
            Cell::Span => "".to_string(),
        }
    }
}
