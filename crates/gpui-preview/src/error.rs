use std::fmt;

/// Errors that can occur during preview or capture operations.
#[derive(Debug)]
pub enum PreviewError {
    /// Filesystem I/O failure (e.g. writing a PNG output file).
    Io(std::io::Error),
    /// Image encoding failure during frame capture.
    #[cfg(feature = "capture")]
    ImageEncode(image::ImageError),
}

impl fmt::Display for PreviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            #[cfg(feature = "capture")]
            Self::ImageEncode(e) => write!(f, "image encoding error: {e}"),
        }
    }
}

impl std::error::Error for PreviewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            #[cfg(feature = "capture")]
            Self::ImageEncode(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for PreviewError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(feature = "capture")]
impl From<image::ImageError> for PreviewError {
    fn from(e: image::ImageError) -> Self {
        Self::ImageEncode(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn preview_error_implements_error_trait() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: PreviewError = PreviewError::from(io_err);
        // Must compile: Error + Display are implemented
        let _: &dyn Error = &err;
        let display = err.to_string();
        assert!(display.contains("I/O error"), "display was: {display}");
    }

    #[test]
    fn preview_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = PreviewError::Io(io_err);
        assert!(err.to_string().starts_with("I/O error:"));
    }

    #[test]
    fn preview_error_source_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        let err = PreviewError::Io(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn from_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let err: PreviewError = io_err.into();
        assert!(matches!(err, PreviewError::Io(_)));
    }
}
