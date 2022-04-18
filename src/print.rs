use self::core::*;
use super::*;
use crate::report::*;

const DEFAULT_INTERVAL_INFO_TITLE: &str = "Interval info:";
const DEFAULT_TASK_INFO_TITLE: &str = "Task info:";

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
pub struct NodeData<'a> {
  pub node: &'a [Node],
  pub title: &'a str,
}

impl NodeData<'_> {
  pub fn default_title() -> &'static str {
    DEFAULT_TASK_INFO_TITLE
  }
}

#[derive(Debug, Clone)]
pub struct IntervalCmdData<'a> {
  pub cmd_text: &'a str,
  pub interval: IntervalData<'a>,
}

#[derive(Debug, Clone)]
pub struct NodeCmdData<'a> {
  pub cmd_text: &'a str,
  pub node: NodeData<'a>,
}

#[derive(Debug, Clone)]
pub struct IntervalError<'a> {
  pub err_text: &'a str,
  pub interval: IntervalData<'a>,
}

pub struct ThemeData {
  pub title: String,
  pub theme: Theme,
}

pub trait Printer {
  fn interval_cmd(&self, d: &IntervalCmdData);
  fn node_cmd(&self, d: &NodeCmdData);
  fn error(&self, e: &str);
  fn interval_error(&self, d: &IntervalData, e: &str);
  fn plain(&self, d: &str);
  fn report(&self, r: &Report);
  fn prompt(&self, p: &str);
  fn task_list(&self, tasks: impl Iterator<Item = Vec<Node>>);
  fn interval_list(&self, intervals: impl Iterator<Item = Interval>);
  fn theme_list(&self, list: impl Iterator<Item = ThemeData>);
}

pub trait Markdown {
  fn markdown(&self) -> String;
}

#[derive(Default)]
pub struct TermPrinter {
  style: AppStyle,
}

impl Printer for TermPrinter {
  fn interval_cmd(&self, d: &IntervalCmdData) {
    self.plain(d.cmd_text);
    println!();
    print_interval_info(&d.interval, &self.style);
  }
  fn node_cmd(&self, d: &NodeCmdData) {
    self.plain(d.cmd_text);
    println!();
    print_node_info(&d.node, &self.style)
  }
  fn error(&self, e: &str) {
    println!(
      "{} {}",
      &self.style.plain.apply("Error:"),
      &self.style.error.apply(e)
    );
  }
  fn interval_error(&self, d: &IntervalData, e: &str) {
    self.error(e);
    println!();
    print_interval_info(d, &self.style);
  }
  fn plain(&self, d: &str) {
    println!("{}", &self.style.plain.apply(d));
  }
  fn report(&self, r: &Report) {
    println!(
      "{}",
      self
        .style
        .report
        .text(&r.markdown(), self.style.screen_width)
    );
  }
  fn prompt(&self, p: &str) {
    println!("{}", p);
  }
  fn task_list(&self, tasks: impl Iterator<Item = Vec<Node>>) {
    print_task_list(tasks, &self.style);
  }
  fn interval_list(&self, intervals: impl Iterator<Item = Interval>) {
    print_intervals_list(intervals, &self.style);
  }
  fn theme_list(&self, list: impl Iterator<Item = ThemeData>) {
    print_theme_list(list);
  }
}

impl TermPrinter {
  pub(crate) fn unstyled() -> Self {
    TermPrinter {
      style: AppStyle::empty(),
    }
  }
  pub(crate) fn new(colors: &Theme) -> Self {
    TermPrinter {
      style: AppStyle::new(colors),
    }
  }
}

fn print_task_list(d: impl Iterator<Item = Vec<Node>>, s: &AppStyle) {
  let plain = &s.plain;
  let s = &s.task_list;
  for task in d {
    let last = task.last().unwrap();
    print!(
      "{}{}{} ",
      plain.apply('['),
      s.id.apply(last.id),
      plain.apply(']')
    );
    for (i, t) in task.iter().enumerate() {
      if i > 0 {
        print!(" {} ", plain.apply('>'));
      }
      print!("{}", s.name.apply(&t.label));
    }
    println!(" {} ", plain.apply(format_datetime(&last.created)));
  }
}

fn print_intervals_list(
  d: impl Iterator<Item = Interval>,
  s: &AppStyle,
) {
  let plain = &s.plain;
  let s = &s.task_list;
  for i in d {
    if i.end.is_some() {
      println!(
        "{}{}{} {} {} {} {} {}",
        plain.apply('['),
        s.id.apply(i.id),
        plain.apply(']'),
        s.name.apply(format_datetime(&i.begin)),
        plain.apply('-'),
        s.name.apply(format_datetime(&i.end.unwrap())),
        plain.apply("task id:"),
        s.name.apply(i.node_id.unwrap()),
      );
    }
  }
}

fn print_interval_info(d: &IntervalData, s: &AppStyle) {
  let plain = &s.plain;
  let s = &s.task;
  println!("{}", plain.apply(d.title));
  print!("  {} ", plain.apply("Task:"));
  for (i, t) in d.task.iter().enumerate() {
    print!("{}", s.name.apply(&t.label));
    if i < d.task.len() - 1 {
      print!(" {} ", plain.apply('>'));
    }
  }
  println!();
  print!(
    "  {} {}",
    plain.apply("Started:"),
    s.start_time.apply(format_datetime(&d.interval.begin))
  );

  let dur = Utc::now() - d.interval.begin;

  if dur.num_seconds() > 2 {
    print!(
      " {}{} {}{}",
      plain.apply('('),
      s.time_span.apply(format_duration(&dur)),
      plain.apply("ago"),
      plain.apply(')')
    );
  }

  if d.interval.end.is_some() {
    let e = d.interval.end.unwrap();
    print!(
      "\n  {} {}",
      plain.apply("Stopped:"),
      s.end_time.apply(format_datetime(&e))
    );
    let dur = Utc::now() - e;
    if dur.num_seconds() > 2 {
      print!(
        " {}{} {}{}",
        plain.apply('('),
        s.time_span.apply(format_duration(&dur)),
        plain.apply("ago"),
        plain.apply(')')
      );
    }
  }

  println!();
}

fn print_node_info(d: &NodeData, s: &AppStyle) {
  let plain = &s.plain;
  let s = &s.task;
  println!("{}", plain.apply(d.title));
  print!("  {} ", plain.apply("Task:"));
  for (i, t) in d.node.iter().enumerate() {
    print!("{}", s.name.apply(&t.label));
    if i < d.node.len() - 1 {
      print!(" {} ", plain.apply('>'));
    }
  }
  println!();
  let last = d.node.last().unwrap();
  print!(
    "  {} {}",
    plain.apply("Created:"),
    s.created_time.apply(format_datetime(&last.created))
  );
  println!();
  if !last.tags.is_empty() {
    print!(
      "  {} {}",
      plain.apply("Tags:"),
      s.tags.apply(
        last
          .tags
          .trim_matches(',')
          .split(',')
          .collect::<Vec<_>>()
          .join(", ")
      )
    );
  }
  println!();
}

fn print_theme_list(list: impl Iterator<Item = ThemeData>) {
  let mut print_list: Vec<ThemeData> = list.collect();
  if print_list.is_empty() {
    return;
  }

  print_list.sort_by(|a, b| a.title.cmp(&b.title));
  let title_max_len =
    print_list.iter().map(|d| d.title.len()).max().unwrap();

  for item in print_list {
    let t = item.theme;
    let pad = " ".repeat(title_max_len - item.title.len());

    println!(
      "   {}: {}{}{}{}{}{}",
      item.title,
      pad,
      get_colored_style(t.c1).apply("▄▄▄"),
      get_colored_style(t.c2).apply("▄▄▄"),
      get_colored_style(t.c3).apply("▄▄▄"),
      get_colored_style(t.c4).apply("▄▄▄"),
      get_colored_style(t.c5).apply("▄▄▄"),
    )
  }
}
