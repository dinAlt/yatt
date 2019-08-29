use std::error::Error;
use std::io;

use config::ConfigError;
use custom_error::custom_error;

use orm::errors::*;

pub type CliResult<T> = std::result::Result<T, CliError>;

custom_error! {pub CliError
    DB {source: DBError} = "Storage error: {:?}",
    Config {source: ConfigError} = "Config parse error : {:?}",
    Io {source: io::Error} = "IO error: {:?}",
    AppDir {message: String}  = "Application directory locate error: {}",
    Cmd{message: String} = "{}",
    Unexpected{message: String} = "Unexpected behavior: {}"

}

impl CliError {
    pub fn wrap(e: Box<dyn Error>) -> DBError {
        DBError::Wrapped { source: e }
    }
}
