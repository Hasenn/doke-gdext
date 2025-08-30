// Create examples/simple_parser.rs
use dokedex::parser_api::{DokeUserParser, ParserContext, DokeResult};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct SimpleTestParser;

impl DokeUserParser for SimpleTestParser {
    fn parse(&self, content: &str, context: &ParserContext) -> DokeResult<HashMap<String, serde_json::Value>> {
        let mut result = HashMap::new();
        result.insert("content".to_string(), serde_json::Value::String(content.to_string()));
        result.insert("length".to_string(), serde_json::Value::Number(content.len().into()));
        result.insert("type".to_string(), serde_json::Value::String("simple_test".to_string()));
        
        Ok(result)
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["Test".to_string(), "Simple".to_string()]
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }
}

// Test the parser
fn main() -> DokeResult<()> {
    let parser = SimpleTestParser;
    let context = ParserContext::new(
        "/dokedex",
        "/project",
        "Test",
        "test.txt",
        "SimpleTestParser"
    );
    
    let result = parser.parse("Hello, world!", &context)?;
    println!("Parsing result: {:?}", result);
    
    Ok(())
}