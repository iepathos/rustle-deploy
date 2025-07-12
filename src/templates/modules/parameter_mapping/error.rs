use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParameterError {
    #[error("Missing required parameter: {param}")]
    MissingRequired { param: String },

    #[error("Invalid parameter value for {param}: {reason}")]
    InvalidValue { param: String, reason: String },

    #[error("Conflicting parameters: {params:?}")]
    ConflictingParameters { params: Vec<String> },

    #[error("Unknown parameter: {param}")]
    UnknownParameter { param: String },
}
