use std::io;

use config::ConfigError;
use custom_error::custom_error;

use orm::errors::*;

custom_error! {pub CliError
    DB {source: DBError} = "Storage error: {:?}",
    Config {source: ConfigError} = "Config parse error : {:?}",
    Io {source: io::Error} = "IO error: {:?}",
    AppDir {message: String}  = "Application directory locate error: {}",
    Cmd{message: String} = "{}",
    Unexpected{message: String} = "Unexpected behavior: {}"
}
