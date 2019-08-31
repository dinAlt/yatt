#[macro_use]
extern crate serde_derive;

use std::fs;
use std::path::PathBuf;

use chrono::prelude::*;
use clap;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use config::{Config, File};
use crossterm_style::Color::*;
use dirs;
use termimad::*;

mod commands;
mod errors;
mod format;
mod print;
mod style;

use errors::*;
pub(crate) use format::*;
use sqlite::DB;
pub(crate) use style::*;
pub(crate) use print::*;

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
    pub skin: &'a MadSkin,
    pub style: AppStyle,
    pub db: Box<dyn core::DBRoot>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub db_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let db_path = String::from("yatt.db");
        AppConfig { db_path }
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

fn make_args<'a>(info: &CrateInfo) -> ArgMatches<'a> {
    App::new(info.name)
        .version(info.version)
        .author(info.authors)
        .about(info.description)
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("start")
                .about("starts new task, or continues existing")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("task")
                        .help("task name with nested tasks, delimited by \"::\"")
                        .required(true)
                        .multiple(true),
                ),
        )
        .subcommand(SubCommand::with_name("stop").about("stops running task"))
        .subcommand(SubCommand::with_name("restart").about("restart last task"))
        .subcommand(SubCommand::with_name("state").about("show running state"))
        .get_matches()
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

    let db = {
        match DB::new(base_path.join(&conf.db_path)) {
            Ok(db) => db,
            Err(e) => return Err(CliError::DB { source: e }),
        }
    };

    commands::exec(&AppContext {
        args: make_args(&info),
        conf,
        root: base_path,
        skin: &skin,
        style: AppStyle::default(),
        db: Box::new(db),
    })
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
