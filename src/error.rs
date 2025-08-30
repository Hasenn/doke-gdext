// src/error.rs
use std::path::{Path, PathBuf};
use thiserror::Error;
use serde_json;
use yaml_rust2::Yaml;

/// Position information for error reporting
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

impl Default for SourcePosition {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            byte_offset: 0,
        }
    }
}

/// Span information for error reporting (start and end positions)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

/// Main error type for the Dokedex system
#[derive(Error, Debug)]
pub enum DokeError {
    #[error("Invalid frontmatter, YAML parsing error : {message}")]
    InvalidFrontmatter {
        message : String,
        file : PathBuf,
        line : usize
    },
    // Parser Errors
    #[error("Syntax error in {file} at line {line}, column {col}: {message} (parser: {parser})")]
    SyntaxError {
        message: String,
        line: usize,
        col: usize,
        file: PathBuf,
        parser: String,
        #[source]
        source: Option<Box<DokeError>>,
    },

    #[error("Validation error in {file}: {message} (parser: {parser})")]
    ValidationError {
        message: String,
        file: PathBuf,
        parser: String,
        span: Option<SourceSpan>,
    },

    #[error("Type mismatch in {file}: expected {expected}, found {found} (parser: {parser})")]
    TypeMismatch {
        expected: String,
        found: String,
        file: PathBuf,
        parser: String,
    },

    #[error("Parser '{parser}' not found for type: {target_type}")]
    ParserNotFound {
        parser: String,
        target_type: String,
    },

    // File System Errors - Using #[from] for automatic conversion
    #[error("I/O error for file {file}: {source}")]
    IoError {
        #[source]
        source: std::io::Error,
        file: PathBuf,
    },

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    // Configuration Errors - Using #[from] for serialization errors
    #[error("Configuration error in {file}: {message}")]
    ConfigError {
        message: String,
        file: PathBuf,
        #[source]
        source: Option<Box<DokeError>>,
    },

    #[error("JSON error in {file}: {source}")]
    JsonError {
        #[source]
        
        source: serde_json::Error,
        file: PathBuf,
    },

    #[error("YAML error in {file}: {source}")]
    YamlError {
        #[source]
        source: yaml_rust2::ScanError,
        file: PathBuf,
    },

    // Export/Import Errors
    #[error("Export error for {file}: {message}")]
    ExportError {
        message: String,
        file: PathBuf,
        #[source]
        source: Option<Box<DokeError>>,
    },

    #[error("Import error for {file}: {message}")]
    ImportError {
        message: String,
        file: PathBuf,
    },

    // Grammar Errors
    #[error("Grammar error: {message}")]
    GrammarError {
        message: String,
        #[source]
        source: Option<Box<DokeError>>,
    },

    // Generic Errors
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

// Manual From implementations for errors that need additional context
impl From<std::io::Error> for DokeError {
    fn from(source: std::io::Error) -> Self {
        DokeError::IoError {
            source,
            file: PathBuf::from("unknown"),
        }
    }
}

// Manual From implementations for errors that need additional context
impl From<serde_json::Error> for DokeError {
    fn from(source: serde_json::Error) -> Self {
        DokeError::JsonError{
            source,
            file: PathBuf::from("unknown"),
        }
    }
}

// Utility methods for error creation
impl DokeError {
    /// Create a syntax error with position information
    pub fn syntax_error(
        message: impl Into<String>,
        line: usize,
        col: usize,
        file: impl Into<PathBuf>,
        parser: impl Into<String>,
    ) -> Self {
        DokeError::SyntaxError {
            message: message.into(),
            line,
            col,
            file: file.into(),
            parser: parser.into(),
            source: None,
        }
    }

    /// Create a validation error
    pub fn validation_error(
        message: impl Into<String>,
        file: impl Into<PathBuf>,
        parser: impl Into<String>,
    ) -> Self {
        DokeError::ValidationError {
            message: message.into(),
            file: file.into(),
            parser: parser.into(),
            span: None,
        }
    }

    /// Create an I/O error with file context
    pub fn io_error(source: std::io::Error, file: impl Into<PathBuf>) -> Self {
        DokeError::IoError {
            source,
            file: file.into(),
        }
    }

    /// Create a config error
    pub fn config_error(message: impl Into<String>, file: impl Into<PathBuf>) -> Self {
        DokeError::ConfigError {
            message: message.into(),
            file: file.into(),
            source: None,
        }
    }

    /// Add source error to this error
    pub fn with_source(self, source: DokeError) -> Self {
        match self {
            DokeError::SyntaxError { message, line, col, file, parser, .. } => {
                DokeError::SyntaxError {
                    message,
                    line,
                    col,
                    file,
                    parser,
                    source: Some(Box::new(source)),
                }
            }
            DokeError::ConfigError { message, file, .. } => {
                DokeError::ConfigError {
                    message,
                    file,
                    source: Some(Box::new(source)),
                }
            }
            DokeError::ExportError { message, file, .. } => {
                DokeError::ExportError {
                    message,
                    file,
                    source: Some(Box::new(source)),
                }
            }
            DokeError::GrammarError { message, .. } => {
                DokeError::GrammarError {
                    message,
                    source: Some(Box::new(source)),
                }
            }
            _ => self, // Other variants don't support source
        }
    }

    /// Get the file path associated with this error
    pub fn file_path(&self) -> Option<&Path> {
        match self {
            DokeError::SyntaxError { file, .. } => Some(file),
            DokeError::ValidationError { file, .. } => Some(file),
            DokeError::TypeMismatch { file, .. } => Some(file),
            DokeError::IoError { file, .. } => Some(file),
            DokeError::FileNotFound(path) => Some(path),
            DokeError::ConfigError { file, .. } => Some(file),
            DokeError::JsonError { file, .. } => Some(file),
            DokeError::YamlError { file, .. } => Some(file),
            DokeError::ExportError { file, .. } => Some(file),
            DokeError::ImportError { file, .. } => Some(file),
            _ => None,
        }
    }
}

// Result type alias for convenience
pub type DokeResult<T> = Result<T, DokeError>;