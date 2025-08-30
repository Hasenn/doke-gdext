//! Doke User Parser API
//!
//! This module defines the interface for creating custom parsers that can be used
//! by the Dokedex system to parse markdown content into structured data.
//!
//! # Overview
//!
//! User parsers are Rust libraries that implement the `DokeUserParser` trait and
//! are compiled to GDExtension libraries. They are automatically discovered and
//! loaded by the Godot-Doke plugin at resource import time.


use std::{collections::HashMap, sync::Arc};
use std::path::PathBuf;

use crate::error::{DokeError, DokeResult};

/// Context provided to parsers during parsing operations.
///
/// Contains information about the current parsing environment including
/// file paths, resource types, and optional parent resource context.
#[derive(Debug, Clone)]
pub struct ParserContext {
    /// Root directory of the Dokedex installation
    pub dokedex_root: PathBuf,
    /// Root directory of the Godot project
    pub project_root: PathBuf,
    /// Type of resource being parsed (e.g., "Item", "Character")
    pub resource_type: String,
    /// Path to the file currently being parsed
    pub current_file: PathBuf,
    /// Name of the parser being used
    pub parser_name: String,
    /// Optional parent resource context for nested parsing
    pub parent_resource: Option<HashMap<String, serde_json::Value>>,
    /// Additional metadata for extended context
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ParserContext {
    /// Creates a new parser context with required fields.
    ///
    /// # Arguments
    ///
    /// * `dokedex_root` - Path to the Dokedex root directory
    /// * `project_root` - Path to the Godot project root
    /// * `resource_type` - Type of resource being parsed
    /// * `current_file` - Path to the file being parsed
    /// * `parser_name` - Name of the parser implementation
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
            metadata: HashMap::new(),
        }
    }

    /// Sets the parent resource context for nested parsing.
    pub fn with_parent_resource(mut self, parent: HashMap<String, serde_json::Value>) -> Self {
        self.parent_resource = Some(parent);
        self
    }

    /// Adds metadata to the context.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Creates a child context for nested parsing.
    pub fn create_child(&self, resource_type: impl Into<String>) -> Self {
        Self::new(
            self.dokedex_root.clone(),
            self.project_root.clone(),
            resource_type,
            self.current_file.clone(),
            self.parser_name.clone(),
        )
        .with_parent_resource(self.get_current_state())
    }

    /// Gets the current parsing state as a serializable map.
    pub fn get_current_state(&self) -> HashMap<String, serde_json::Value> {
        let mut state = HashMap::new();
        state.insert("resource_type".to_string(), serde_json::Value::String(self.resource_type.clone()));
        state.insert("file".to_string(), serde_json::Value::String(self.current_file.display().to_string()));
        state.insert("parser".to_string(), serde_json::Value::String(self.parser_name.clone()));
        state
    }
}

/// Main trait that all user parsers must implement.
///
/// This trait defines the interface between custom parser implementations
/// and the Dokedex system. Parsers are responsible for converting markdown
/// content into structured data that can be used by Godot resources.
pub trait DokeUserParser: Send + Sync {
    /// Parses markdown content into structured data.
    ///
    /// # Arguments
    ///
    /// * `content` - The markdown content to parse
    /// * `context` - Context information about the parsing operation
    ///
    /// # Returns
    ///
    /// A `HashMap` containing the parsed data structure, where keys are
    /// property names and values are JSON-compatible values.
    ///
    /// # Errors
    ///
    /// Returns a `DokeError` if parsing fails due to syntax errors,
    /// validation failures, or other issues.
    fn parse(
        &self,
        content: &str,
        context: &ParserContext,
    ) -> DokeResult<HashMap<String, serde_json::Value>>;

    /// Returns the resource types supported by this parser.
    ///
    /// This method should return a list of resource type names that this
    /// parser can handle. The Dokedex system will use this information
    /// to automatically select the appropriate parser for each resource.
    fn supported_types(&self) -> Vec<String>;

    /// Returns the version of this parser implementation.
    ///
    /// Used for cache invalidation and compatibility checking. Should
    /// change whenever the parser's behavior or output format changes.
    fn version(&self) -> String;

    /// Optional: Provides a default configuration for this parser.
    ///
    /// This configuration will be used when no custom configuration is
    /// provided. Can return `None` if no default configuration is needed.
    fn default_config(&self) -> Option<HashMap<String, serde_json::Value>> {
        None
    }

    /// Optional: Validates parser configuration.
    ///
    /// Called before parsing to validate any provided configuration.
    /// Returns `Ok(())` if the configuration is valid, or an error
    /// describing validation failures.
    fn validate_config(
        &self,
        config: &HashMap<String, serde_json::Value>,
    ) -> DokeResult<()> {
        let _ = config; // Default implementation accepts any config
        Ok(())
    }
}

/// Registry for managing available parsers.
///
/// This struct provides methods for registering, discovering, and
/// retrieving parsers. It's used by both the CLI and Godot plugin.
pub struct ParserRegistry {
    parsers: HashMap<String, Arc<dyn DokeUserParser>>,
}

impl ParserRegistry {
    /// Creates a new empty parser registry.
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }

    /// Registers a parser with the registry.
    pub fn register(&mut self, parser: Arc<dyn DokeUserParser>) {
        for type_name in parser.supported_types() {
            self.parsers.insert(type_name.to_lowercase(), parser.clone());
        }
    }

    /// Finds a parser for the given resource type.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - The type of resource to find a parser for
    ///
    /// # Returns
    ///
    /// A reference to the parser if found, or `None` if no matching
    /// parser is registered.
    pub fn get_parser(&self, resource_type: &str) -> Option<&dyn DokeUserParser> {
        self.parsers.get(&resource_type.to_lowercase()).map(|p| p.as_ref())
    }

    /// Returns all registered parsers.
    pub fn get_all_parsers(&self) -> Vec<&dyn DokeUserParser> {
        self.parsers.values().map(|p| p.as_ref()).collect()
    }

    /// Returns all supported resource types.
    pub fn get_supported_types(&self) -> Vec<String> {
        self.parsers.keys().cloned().collect()
    }
}

/// Macro for easily registering parsers.
///
/// This macro simplifies the process of registering parsers by
/// automatically handling Arc wrapping and registration.
/// register_parser!(MyParser, "Item", "Boot")
#[macro_export]
macro_rules! register_parser {
    ($parser_type:ty, $($type_name:expr),+) => {
        {
            let parser = Arc::new(<$parser_type>::new());
            let mut registry = $crate::parser_api::ParserRegistry::new();
            registry.register(parser);
            registry
        }
    };
}

/// Default markdown parser implementation.
///
/// Provides basic markdown parsing functionality that can be used
/// as a fallback or for simple content types.
pub struct DefaultMarkdownParser;

impl DokeUserParser for DefaultMarkdownParser {
    fn parse(
        &self,
        content: &str,
        context: &ParserContext,
    ) -> DokeResult<HashMap<String, serde_json::Value>> {
        let mut result = HashMap::new();

        // Basic markdown parsing
        result.insert("raw_content".to_string(), serde_json::Value::String(content.to_string()));
        result.insert("type".to_string(), serde_json::Value::String("markdown".to_string()));

        // Simple section detection
        let lines: Vec<&str> = content.lines().collect();
        let mut sections = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            if line.starts_with('#') {
                let level = line.chars().take_while(|c| *c == '#').count();
                sections.push(serde_json::json!({
                    "type": "heading",
                    "level": level,
                    "content": line.trim_start_matches('#').trim(),
                    "line": line_num + 1
                }));
            } else if !line.trim().is_empty() {
                sections.push(serde_json::json!({
                    "type": "paragraph",
                    "content": line.trim(),
                    "line": line_num + 1
                }));
            }
        }

        result.insert("sections".to_string(), serde_json::Value::Array(sections));
        Ok(result)
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["Markdown".to_string(), "Text".to_string(), "Note".to_string()]
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }
}

// Unit tests for the parser API
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_context_creation() {
        let context = ParserContext::new(
            "/dokedex",
            "/project",
            "Item",
            "/dokedex/Items/sword.md",
            "ItemParser",
        );

        assert_eq!(context.resource_type, "Item");
        assert_eq!(context.parser_name, "ItemParser");
    }

    #[test]
    fn test_default_markdown_parser() {
        let parser = DefaultMarkdownParser;
        let context = ParserContext::new(
            "/dokedex",
            "/project",
            "Markdown",
            "test.md",
            "DefaultMarkdownParser",
        );

        let content = "# Heading\nSome content";
        let result = parser.parse(content, &context).unwrap();

        assert!(result.contains_key("sections"));
        let sections = result["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn test_parser_registry() {
        let mut registry = ParserRegistry::new();
        let parser = Arc::new(DefaultMarkdownParser);
        
        registry.register(parser);
        
        // Test that parser is registered for all supported types
        assert!(registry.get_parser("markdown").is_some());
        assert!(registry.get_parser("text").is_some());
        assert!(registry.get_parser("note").is_some());
        
        // Test that unknown types return None
        assert!(registry.get_parser("unknown").is_none());
    }
}