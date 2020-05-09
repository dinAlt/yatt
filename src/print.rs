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

pub trait Printer {
  fn interval_cmd(&self, d: &IntervalCmdData);
  fn node_cmd(&self, d: &NodeCmdData);
  fn error(&self, e: &str);
  fn interval_error(&self, d: &IntervalData, e: &str);
  fn cmd(&self, d: &str);
  fn report(&self, r: &Report);
  fn prompt(&self, p: &str);
  fn task_list(&self, tasks: impl Iterator<Item = Vec<Node>>);
  fn interval_list(&self, intervals: impl Iterator<Item = Interval>);
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
  fn node_cmd(&self, d: &NodeCmdData) {
    self.cmd(d.cmd_text);
    println!();
    print_node_info(&d.node, &self.style.task)
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
    print_task_list(tasks, &self.style.task_list);
  }
  fn interval_list(&self, intervals: impl Iterator<Item = Interval>) {
    print_intervals_list(intervals, &self.style.task_list);
  }
}

fn print_task_list<'a>(
  d: impl Iterator<Item = Vec<Node>>,
  s: &TaskListStyle,
) {
  for task in d {
    let last = task.last().unwrap();
    print!("[{}] ", s.id.apply_to(last.id));
    for (i, t) in task.iter().enumerate() {
      if i > 0 {
        print!(" > ");
      }
      print!("{}", s.name.apply_to(&t.label));
    }
    print!(" {} \n", format_datetime(&last.created));
  }
}

fn print_intervals_list<'a>(
  d: impl Iterator<Item = Interval>,
  s: &TaskListStyle,
) {
  for i in d {
    if i.end.is_some() {
      print!(
        "[{}] {} - {} [{}]\n",
        s.id.apply_to(i.id),
        s.name.apply_to(format_datetime(&i.begin)),
        s.name.apply_to(format_datetime(&i.end.unwrap())),
        s.name.apply_to(i.node_id.unwrap()),
      );
    }
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
    print!(
      "\n  Stopped: {}",
      s.end_time.apply_to(format_datetime(&e))
    );
    let dur = Utc::now() - e;
    if dur.num_seconds() > 2 {
      print!(
        " ({} ago)",
        s.time_span.apply_to(format_duration(&dur))
      );
    }
  }

  println!();
}

fn print_node_info(d: &NodeData, s: &TaskStyle) {
  println!("{}", d.title);
  print!("  Task: ");
  for (i, t) in d.node.iter().enumerate() {
    print!("{}", s.name.apply_to(&t.label));
    if i < d.node.len() - 1 {
      print!(" > ");
    }
  }
  println!();
  print!(
    "  Created: {}",
    s.created_time
      .apply_to(format_datetime(&d.node.last().unwrap().created))
  );
  println!();
}
