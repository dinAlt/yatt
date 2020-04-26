use custom_error::custom_error;
use std::error::Error;

pub type DBResult<T> = Result<T, DBError>;

custom_error! {pub DBError
    Unexpected{message: String} = "Unexpected behavior: {}",
    Wrapped {source: Box<dyn Error>} = "Underlying error: {:?}",
    IsEmpty {message: String} = "Empty result: {}",
    Convert {message: String} = "Conversion error: {}",
}

impl DBError {
  pub fn wrap(e: Box<dyn Error>) -> DBError {
    DBError::Wrapped { source: e }
  }
}
