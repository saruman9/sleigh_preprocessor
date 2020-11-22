use std::{fmt, io, path::PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Preprocessor(PreprocessorError),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<PreprocessorError> for Error {
    fn from(err: PreprocessorError) -> Self {
        Self::Preprocessor(err)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::Io(ref e) => Some(e),
            Error::Preprocessor(ref e) => Some(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref e) => write!(f, "IO error: {}", e),
            Error::Preprocessor(ref e) => write!(f, "Preprocessor error: {}", e),
        }
    }
}

#[derive(Debug)]
pub struct PreprocessorError {
    message: String,
    path: PathBuf,
    line_no: usize,
    overall_line_no: usize,
    line: String,
}

impl PreprocessorError {
    pub(crate) fn new<S, P>(
        message: S,
        path: P,
        line_no: usize,
        overall_line_no: usize,
        line: S,
    ) -> Self
    where
        S: Into<String>,
        P: Into<PathBuf>,
    {
        let message = message.into();
        let path = path.into();
        let line = line.into();
        Self {
            message,
            path,
            line_no,
            overall_line_no,
            line,
        }
    }
}

impl std::error::Error for PreprocessorError {}

impl fmt::Display for PreprocessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}({}): {}",
            self.message,
            self.path.display(),
            self.line_no,
            self.overall_line_no,
            self.line
        )
    }
}
