//! Advanced template processing module with comprehensive Jinja2 compatibility

pub mod handlebars_helpers;
pub mod jinja_parser;
pub mod template_processor;

pub use handlebars_helpers::*;
pub use jinja_parser::{ConversionResult, Jinja2Parser, ParseError};
pub use template_processor::{AdvancedTemplateProcessor, TemplateError};
