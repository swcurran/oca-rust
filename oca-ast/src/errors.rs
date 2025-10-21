use thiserror::Error;

#[derive(Error, Debug)]

pub enum Error {
    #[error("{0}")]
    InvalidVersion(String),

    #[error("{0}")]
    InvalidOperation(String),

    #[error("{0}")]
    Unknown(String),

    #[error("")]
    MissingVersion(),

    #[error("Validation errors: {0:?}")]
    Validation(Vec<Error>),

    #[error("Overlay definition not found for {0}")]
    MissingOverlayDef(String),

    #[error("Attribute not allowed by overlay definition: {0}")]
    InvalidAttribute(String),

    #[error("Duplicate attribute in CaptureBase: {0}")]
    DuplicateAttribute(String),

    #[error("Missing required attribute in Overlay: {0}")]
    MissingRequiredAttribute(String),

    #[error("Invalid Overlay Name: {0}")]
    InvalidOverlayName(String),

    #[error("Invalid Property Value: {0}")]
    InvalidPropertyValue(String),

    #[error("Invalid Property: {0}")]
    InvalidProperty(String),
}

#[allow(dead_code)]
struct Errors(Vec<Error>);

impl std::fmt::Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0
            .iter()
            .try_fold((), |_result, error| writeln!(f, "{}", error))
    }
}
