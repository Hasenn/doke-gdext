// src/parser_api.rs
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

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

impl SourcePosition {
    pub fn new(line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            line,
            column,
            byte_offset,
        }
    }
    
    pub fn from_byte_offset(content: &str, byte_offset: usize) -> Self {
        let mut line = 1;
        let mut column = 1;
        let mut current_offset = 0;
        
        for c in content.chars() {
            if current_offset >= byte_offset {
                break;
            }
            
            if c == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            
            current_offset += c.len_utf8();
        }
        
        Self {
            line,
            column,
            byte_offset,
        }
    }
}

/// Span information for error reporting (start and end positions)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    pub fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }
    
    pub fn single_position(pos: SourcePosition) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
}

/// Error types for parser operations with detailed context
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Syntax error in {file} at {span:?}: {message}")]
    SyntaxError {
        message: String,
        span: SourceSpan,
        file: PathBuf,
        parser: String,
    },
    
    #[error("Validation error in {file} (parser: {parser}): {message}")]
    ValidationError {
        message: String,
        file: PathBuf,
        parser: String,
        span: Option<SourceSpan>,
    },
    
    #[error("Type mismatch in {file} at {span:?} (parser: {parser}): {message}")]
    TypeMismatch {
        message: String,
        span: SourceSpan,
        file: PathBuf,
        parser: String,
    },
    
    #[error("I/O error for file {file}: {source}")]
    IoError {
        #[source]
        source: std::io::Error,
        file: PathBuf,
    },
    
    #[error("Parser '{parser}' not found for type: {target_type}")]
    ParserNotFound {
        parser: String,
        target_type: String,
        file: Option<PathBuf>,
    },
    
    #[error("Invalid frontmatter in {file}: {message}")]
    InvalidFrontmatter {
        message: String,
        file: PathBuf,
        span: Option<SourceSpan>,
    },
    
    #[error("AST conversion error in {file} (parser: {parser}): {message}")]
    AstConversionError {
        message: String,
        file: PathBuf,
        parser: String,
        span: Option<SourceSpan>,
    },
    
    #[error("Unsupported operation by parser '{parser}': {message}")]
    UnsupportedOperation {
        message: String,
        parser: String,
        file: Option<PathBuf>,
    },
    
    #[error("Parser '{parser}' failed for file {file}: {message}")]
    ParserFailure {
        message: String,
        parser: String,
        file: PathBuf,
        span: Option<SourceSpan>,
    },
}

// Implement conversion from std::io::Error with file context
impl ParserError {
    pub fn io_error(source: std::io::Error, file: impl AsRef<Path>) -> Self {
        ParserError::IoError {
            source,
            file: file.as_ref().to_path_buf(),
        }
    }
    
    pub fn syntax_error(message: impl Into<String>, span: SourceSpan, file: impl AsRef<Path>, parser: impl Into<String>) -> Self {
        ParserError::SyntaxError {
            message: message.into(),
            span,
            file: file.as_ref().to_path_buf(),
            parser: parser.into(),
        }
    }
    
    pub fn validation_error(message: impl Into<String>, file: impl AsRef<Path>, parser: impl Into<String>, span: Option<SourceSpan>) -> Self {
        ParserError::ValidationError {
            message: message.into(),
            file: file.as_ref().to_path_buf(),
            parser: parser.into(),
            span,
        }
    }
}

// Implement conversion from std::io::Error
impl From<std::io::Error> for ParserError {
    fn from(error: std::io::Error) -> Self {
        ParserError::IoError {
            source: error,
            file: PathBuf::from("unknown"),
        }
    }
}

// Implement conversion from serde_json::Error with context
impl From<serde_json::Error> for ParserError {
    fn from(error: serde_json::Error) -> Self {
        ParserError::AstConversionError {
            message: format!("JSON error: {}", error),
            file: PathBuf::from("unknown"),
            parser: "serde_json".to_string(),
            span: None,
        }
    }
}

/// Enhanced context with file information
#[derive(Clone, Debug)]
pub struct ParserContext {
    pub dokedex_root: PathBuf,
    pub project_root: PathBuf,
    pub resource_type: String,
    pub parent_resource: Option<HashMap<String, serde_json::Value>>,
    pub current_file: PathBuf,
    pub parser_name: String,
}

impl ParserContext {
    pub fn new(
        dokedex_root: impl Into<PathBuf>,
        project_root: impl Into<PathBuf>,
        resource_type: impl Into<String>,
        current_file: impl Into<PathBuf>,
        parser_name: impl Into<String>,
    ) -> Self {
        Self {
            dokedex_root: dokedex_root.into(),
            project_root: project_root.into(),
            resource_type: resource_type.into(),
            current_file: current_file.into(),
            parser_name: parser_name.into(),
            parent_resource: None,
        }
    }
    
    pub fn with_parent_resource(mut self, parent: HashMap<String, serde_json::Value>) -> Self {
        self.parent_resource = Some(parent);
        self
    }
    
    pub fn error(&self, kind: ParserErrorKind, message: impl Into<String>, span: Option<SourceSpan>) -> ParserError {
        match kind {
            ParserErrorKind::Syntax => ParserError::syntax_error(
                message.into(),
                span.unwrap_or_else(|| SourceSpan::single_position(SourcePosition::default())),
                &self.current_file,
                &self.parser_name,
            ),
            ParserErrorKind::Validation => ParserError::validation_error(
                message.into(),
                &self.current_file,
                &self.parser_name,
                span,
            ),
            ParserErrorKind::TypeMismatch => ParserError::TypeMismatch {
                message: message.into(),
                span: span.unwrap_or_else(|| SourceSpan::single_position(SourcePosition::default())),
                file: self.current_file.clone(),
                parser: self.parser_name.clone(),
            },
        }
    }
}

/// Convenience enum for common error types
pub enum ParserErrorKind {
    Syntax,
    Validation,
    TypeMismatch,
}

/// Result type for parser operations
pub type ParserResult<T> = Result<T, ParserError>;