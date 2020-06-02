#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

use std::fs;
use std::path::PathBuf;

use crate::core::DBRoot;
use chrono::prelude::*;
use clap;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use config::{Config, File};
use dirs;

mod commands;
mod core;
mod errors;
mod format;
mod history;
mod history_storage;
mod parse;
mod print;
mod report;
mod style;

use errors::*;
pub(crate) use format::*;
use history::DBWatcher;
pub use print::*;
pub(crate) use style::*;
use yatt_orm::sqlite::DB;

pub struct CrateInfo<'a> {
  pub name: &'a str,
  pub version: &'a str,
  pub authors: &'a str,
  pub description: &'a str,
}

pub struct AppContext<'a, T, P>
where
  T: DBRoot,
  P: Printer,
{
  pub args: ArgMatches<'a>,
  pub conf: AppConfig,
  pub root: PathBuf,
  pub printer: P,
  pub db: T,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
  pub db_path: String,
  pub history_db_path: String,
}
impl Default for AppConfig {
  fn default() -> Self {
    let db_path = String::from("yatt.db");
    let history_db_path = String::from("yatt_history.db");
    AppConfig {
      db_path,
      history_db_path,
    }
  }
}

fn parse_config(base_path: &PathBuf) -> CliResult<AppConfig> {
  let mut s = Config::new();
  let path = base_path.join("config");
  if s.merge(File::with_name(path.to_str().unwrap())).is_err() {
    return Ok(AppConfig::default());
  }
  match s.try_into() {
    Ok(res) => Ok(res),
    Err(e) => Err(CliError::Config { source: e }),
  }
}

fn make_args<'a>(info: &CrateInfo<'a>) -> ArgMatches<'a> {
  let app = App::new(info.name)
    .version(info.version)
    .author(info.authors)
    .about(info.description)
    .arg(
      Arg::with_name("no-color")
        .help("Unstyled output")
        .short("c"),
    )
    .setting(AppSettings::ArgRequiredElseHelp);

  commands::register(app).get_matches()
}

fn app_dir(name: &str) -> CliResult<PathBuf> {
  if let Some(p) = dirs::config_dir() {
    return Ok(p.join(name));
  }
  Err(CliError::AppDir {
    message: "Unable to resolve os config directory path".to_string(),
  })
}

pub fn run(info: CrateInfo) -> CliResult<()> {
  let base_path = app_dir(info.name)?;
  if !base_path.exists() {
    if let Err(e) = fs::create_dir_all(&base_path) {
      return Err(CliError::Io { source: e });
    }
  } else if !base_path.is_dir() {
    return Err(CliError::AppDir {
      message: format!(
        "{} is not a directory",
        base_path.to_str().unwrap_or("")
      ),
    });
  }

  let mut conf = parse_config(&base_path)?;

  #[cfg(debug_assertions)]
  debug_config(&mut conf);

  let db = match DB::new(base_path.join(&conf.db_path)) {
    Ok(db) => db,
    Err(e) => return Err(CliError::DB { source: e }),
  };

  let history_db_path = base_path.join(&conf.history_db_path);
  if history_db_path.exists() {
    let hs = {
      match history_storage::sqlite::DB::new(history_db_path) {
        Ok(db) => db,
        Err(e) => return Err(CliError::DB { source: e }),
      }
    };
    let db = DBWatcher::new(db, hs);
    run_app(db, base_path, &info, conf)
  } else {
    run_app(db, base_path, &info, conf)
  }
}

fn run_app<T: DBRoot>(
  db: T,
  base_path: PathBuf,
  info: &CrateInfo,
  conf: AppConfig,
) -> CliResult<()> {
  let args = make_args(info);
  let printer = if args.is_present("no-color") {
    TermPrinter::unstyled()
  } else {
    TermPrinter::default()
  };
  let app = AppContext {
    args,
    conf,
    root: base_path,
    printer,
    db,
  };
  let res = commands::exec(&app);
  if res.is_err() {
    print_error(res.as_ref().unwrap_err(), &app.printer);
  }

  res
}

fn print_error<T: Printer>(e: &CliError, p: &T) {
  if let CliError::Task { source } = e {
    match source {
      TaskError::Cmd { message } => p.error(message),
      TaskError::CmdTaskInterval {
        message,
        interval,
        task,
      } => p.interval_error(
        &IntervalData {
          interval,
          task,
          title: IntervalData::default_title(),
        },
        message,
      ),
    }
    return;
  }

  p.error(&e.to_string());
}

fn debug_config(conf: &mut AppConfig) {
  conf.db_path = "yatt_debug.db".to_string();
}
