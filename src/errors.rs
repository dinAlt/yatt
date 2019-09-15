use std::error::Error;
use std::io;

use config::ConfigError;
use custom_error::*;

use super::format::*;
use crate::core::*;
use yatt_orm::errors::*;

pub type CliResult<T> = std::result::Result<T, CliError>;

custom_error! {pub CliError
    DB {source: DBError} = "Storage error: {:?}",
    Config {source: ConfigError} = "Config parse error : {:?}",
    Io {source: io::Error} = "IO error: {:?}",
    AppDir {message: String}  = "Application directory locate error: {}",
    Cmd{message: String} = "{}",
    Unexpected{message: String} = "Unexpected behavior: {}",
    Wrapped{source: Box<dyn Error>} = "{:?}",
    Task { source: TaskError } = "Task error: {:?}",
    Parse {message: String} = "Parse error: {}",
}

impl CliError {
    pub fn wrap(e: Box<dyn Error>) -> CliError {
        CliError::Wrapped { source: e }
    }
}

custom_error! {pub TaskError
    CmdTaskInterval{
         message: String,
         interval: Interval,
         task: Vec<Node>} = @{
             format!("Error: {}, task: {}, interval: {}",
             message,
             format_task_name(&task),
             interval.to_string()) },
    Cmd {message: String} = "{}",
}
