use std::error::Error;
use custom_error::custom_error;

pub type DBResult<T> = Result<T, DBError>;

custom_error! {pub DBError
    Unexpected{message: String} = "Unexpected behavior: {}",
    Wrapped {source: Box<dyn Error>} = "Underlying error: {:?}",
}

impl DBError {
    pub fn wrap(e: Box<dyn Error>) -> DBError {
        DBError::Wrapped{source: e}
    }
}
