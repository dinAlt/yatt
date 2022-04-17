#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::DBRoot;
use chrono::prelude::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use config::{Config, File};
use semver::Version;

mod commands;
mod core;
pub mod errors;
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
  pub db: &'a T,
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

fn parse_config(base_path: &Path) -> CliResult<AppConfig> {
  let mut s = Config::new();
  let path = base_path.join("config");
  if s.merge(File::with_name(path.to_str().unwrap())).is_err() {
    return Ok(AppConfig::default());
  }
  s.try_into().map_err(|e| CliError::Config { source: e })
}

fn make_args<'a>(info: &CrateInfo<'a>) -> ArgMatches<'a> {
  let app = App::new(info.name)
    .version(info.version)
    .author(info.authors)
    .about(info.description)
    .arg(
      Arg::with_name("no-color")
        .long("no-color")
        .help("Unstyled output")
        .short("c"),
    )
    .arg(
      Arg::with_name("theme")
        .help("theme name or inline color list")
        .long_help(
          r#"This parameter takes one of installed theme name,
or inline color list (up to 5 colors) to override defaults.
Inline value should starts with "inline:", followed by
colon separated colors in hex, ansi (in rgb or 256 colors 
format) format. This color aliases also allowed: black,
dark_grey, red dark_red, green, dark_green, yellow, dark_yellow,
blue, dark_blue, magenta, dark_magenta, cyan, dark_cyan, white,
grey.

Examples:
  use gruvbox-dark-hard theme (should be installed):
    yatt --theme gruvbox-dark-hard state
  inline colors with hex values:
    yatt --theme 'inline:#ff0000:#00ff00:#0000ff' state
  inline colors with ansi values:
    yatt --theme 'inline:5;40:5;30:5;70:2;255;0;0:2;255;0;0' state
  inline colors with aliases:
    yatt --theme 'inline:red:green:blue:yellow:white' state
    "#,
        )
        .takes_value(true)
        .long("theme"),
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

  let mut db = DB::new(base_path.join(&conf.db_path), |con| {
    let db_ver =
      con.query_row("select version from version", [], |r| r.get(0));
    let db_ver = match db_ver {
      Ok(ver) => Ok(ver),
      Err(e) => {
        if !e.to_string().contains("no such table: version") {
          Err(e)
        } else {
          Ok(String::from(""))
        }
      }
    }?;
    let db_ver = if db_ver.is_empty() {
      con.execute(
        "create table version (version TEXT NOT NULL)",
        [],
      )?;
      con.execute("insert into version values ('0')", [])?;
      if con
        .query_row("select id from nodes limit 1", [], |_| Ok(()))
        .is_err()
      {
        con.execute(
          "create table nodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            label TEXT NOT NULL,
            parent_id INTEGER,
            created INTEGER NOT NULL,
            closed INTEGER DEFAULT 0,
            deleted integer default 0
            )",
          [],
        )?;
        con.execute(
          "create table intervals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id integer,
             begin integer NOT NULL,
             end integer,
             deleted integer default 0,
             closed integer default 0
             )",
          [],
        )?;
      };
      String::from("0.0.0")
    } else {
      db_ver
    };

    if db_ver == "0.0.0" {
      con.execute(
        "alter table nodes add column tags TEXT NOT NULL DEFAULT ''",
        [],
      )?;
    }
    let db_sem_ver = Version::parse(&db_ver).unwrap();
    let crate_ver = clap::crate_version!();
    let crate_sem_ver = Version::parse(crate_ver).unwrap();

    if db_sem_ver < crate_sem_ver {
      con.execute("update version set version = ?", &[crate_ver])?;
    }

    Ok(())
  })
  .map_err(|e| CliError::Wrapped { source: e.into() })?;

  let db = db.transaction()?;

  let history_db_path = base_path.join(&conf.history_db_path);
  let res = if history_db_path.exists() {
    let hs = {
      match history_storage::sqlite::DB::new(history_db_path) {
        Ok(db) => db,
        Err(e) => return Err(CliError::DB { source: e }),
      }
    };
    let db = DBWatcher::new(&db, hs);
    run_app(&db, base_path, &info, conf)
  } else {
    run_app(&db, base_path, &info, conf)
  };

  if res.is_ok() {
    db.commit()?;
  }

  res
}

fn run_app<T: DBRoot>(
  db: &T,
  base_path: PathBuf,
  info: &CrateInfo,
  conf: AppConfig,
) -> CliResult<()> {
  let args = make_args(info);
  let printer = if args.is_present("no-color") {
    TermPrinter::unstyled()
  } else if let Some(theme) = args.value_of("theme") {
    match load_theme(theme, &base_path.join("themes")) {
      Err(e) => {
        println!("Error: {}", e);
        return Err(e);
      }
      Ok(c) => TermPrinter::new(&c),
    }
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

fn load_theme(theme: &str, themes_dir: &Path) -> CliResult<Theme> {
  if theme.starts_with("inline:") {
    Theme::try_from(theme.trim_start_matches("inline:"))
  } else {
    let pth = themes_dir.join(theme);
    if !pth.exists() || !pth.is_file() {
      return Err(CliError::Cmd {
        message: format!("theme not found: \"{}\"", theme),
      });
    }

    let theme = fs::read_to_string(pth)
      .map_err(|source| CliError::wrap(Box::new(source)))?;

    Theme::try_from(theme.as_str().trim())
  }
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
