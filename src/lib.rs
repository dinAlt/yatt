#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::prelude::*;
use clap;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use config::{Config, File};
use crossterm_style::Color::*;
use dirs;
use termimad::*;

mod commands;
mod core;
mod errors;
mod format;
mod history;
mod history_storage;
mod parse;
mod print;
mod report;
mod storage;
mod style;

use errors::*;
pub(crate) use format::*;
use history::DBWatcher;
pub use print::*;
use storage::sqlite::DB;
pub(crate) use style::*;

pub struct CrateInfo<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub authors: &'a str,
    pub description: &'a str,
}

pub struct AppContext<'a> {
    pub args: ArgMatches<'a>,
    pub conf: AppConfig,
    pub root: PathBuf,
    pub printer: Box<dyn Printer>,
    pub db: Box<dyn core::DBRoot>,
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
            message: format!("{} is not a directory", base_path.to_str().unwrap_or("")),
        });
    }

    let mut skin = MadSkin::default();
    skin.set_headers_fg(rgb(255, 187, 0));
    skin.bold.set_fg(Yellow);
    skin.italic.set_fgbg(Magenta, rgb(30, 30, 40));
    skin.bullet = StyledChar::from_fg_char(Yellow, '⟡');
    skin.quote_mark.set_fg(Yellow);
    let mut conf = parse_config(&base_path)?;

    #[cfg(debug_assertions)]
    debug_config(&mut conf);

    let mut db: Box<dyn core::DBRoot> = Box::new({
        match DB::new(base_path.join(&conf.db_path)) {
            Ok(db) => db,
            Err(e) => return Err(CliError::DB { source: e }),
        }
    });

    let history_db_path = base_path.join(&conf.history_db_path);
    if history_db_path.exists() {
        let hs = Rc::new({
            match history_storage::sqlite::DB::new(history_db_path) {
                Ok(db) => db,
                Err(e) => return Err(CliError::DB { source: e }),
            }
        });
        db = Box::new(DBWatcher::new(db, hs));
    }
    let app = AppContext {
        args: make_args(&info),
        conf,
        root: base_path,
        printer: Box::new(TermPrinter::default()),
        db,
    };
    let res = commands::exec(&app);

    if res.is_err() {
        print_error(res.as_ref().unwrap_err(), app.printer);
    }

    res
}

fn print_error(e: &CliError, p: Box<dyn Printer>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        // assert_eq!(test_command(), format!("{}", 12));
    }
}
