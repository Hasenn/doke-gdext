// src/parsers/doke_parser.rs
use markdown::{mdast::Node, ParseOptions};
use serde_json::{Value, Map};
use std::collections::HashMap;
use yaml_rust2::{YamlLoader, Yaml};
use regex::Regex;
use crate::error::{DokeError, DokeResult};
use crate::parser_api::{DokeUserParser, ParserContext};
use serde_derive::Serialize;
pub struct DokeMarkdownParser;

#[derive(Debug, Clone, Serialize)]
pub struct ResourceLink {
    pub resource_type: Option<String>,
    pub resource_name: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DokeNode {
    pub node_type: String,
    pub markdown_element: String,
    pub content: Option<String>,
    pub raw_content: String,    // Changed from &'input str to String
    pub level: Option<u32>,
    pub line: usize,
    pub column: usize,
    pub children: Vec<DokeNode>,
    pub wiki_links: Vec<ResourceLink>,
    pub ordered: Option<bool>,
    pub resolved: bool,
}

impl DokeUserParser for DokeMarkdownParser {
    fn parse(&self, content: &str, context: &ParserContext) -> DokeResult<HashMap<String, Value>> {
        let (frontmatter, body) = parse_frontmatter(content)?;
        let ast = parse_markdown_body(&body, context)?;
        
        let mut result = HashMap::new();
        result.insert("frontmatter".to_string(), Value::Object(convert_hashmap_to_object(frontmatter)));
        result.insert("body".to_string(), serde_json::to_value(ast)?);
        
        Ok(result)
    }

    fn supported_types(&self) -> Vec<String> {
        vec!["Markdown".to_string(), "Doke".to_string(), "Generic".to_string()]
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }
}

fn convert_hashmap_to_object(map: HashMap<String, Value>) -> Map<String, Value> {
    map.into_iter().collect()
}

fn parse_frontmatter(content: &str) -> DokeResult<(HashMap<String, Value>, String)> {
    let frontmatter_regex = Regex::new(r"^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)").unwrap();
    
    if let Some(caps) = frontmatter_regex.captures(content) {
        let yaml_content = caps.get(1).unwrap().as_str();
        let body_content = caps.get(2).unwrap().as_str();
        
        let docs = YamlLoader::load_from_str(yaml_content)
            .map_err(|e| DokeError::InvalidFrontmatter {
                message: format!("YAML parsing error: {}", e),
                file: "unknown".into(),
                line: 0,
            })?;
            
        if docs.is_empty() {
            return Err(DokeError::InvalidFrontmatter {
                message: "Empty YAML frontmatter".to_string(),
                file: "unknown".into(),
                line: 0,
            });
        }
        
        let yaml_doc = &docs[0];
        let mut frontmatter = HashMap::new();
        parse_yaml_to_value(yaml_doc, &mut frontmatter, "");
        
        Ok((frontmatter, body_content.to_string()))
    } else {
        Ok((HashMap::new(), content.to_string()))
    }
}

fn parse_markdown_body(content: &str, context: &ParserContext) -> DokeResult<Vec<DokeNode>> {
    let options = ParseOptions::gfm();
    let ast = markdown::to_mdast(content, &options)
        .map_err(|e| DokeError::ValidationError {
            message: format!("Markdown parsing error: {}", e),
            file: context.current_file.display().to_string().into(),
            parser: context.parser_name.clone(),
            span: None,
        })?;
    
    let mut nodes = Vec::new();
    if let Node::Root(root) = ast {
        for child in root.children {
            nodes.push(convert_mdast_node(child, content, 1, 1));
        }
    }
    
    Ok(nodes)
}
fn convert_mdast_node(node: Node, original_content: &str, line: usize, column: usize) -> DokeNode {
    match node {
        Node::Heading(heading) => {
            let children = (&heading).clone().children.into_iter()
                .map(|child| convert_mdast_node(child, original_content, line, column))
                .collect();
                
            DokeNode {
                node_type: "DokeNode".to_string(),
                markdown_element: "heading".to_string(),
                content: extract_text_content_from_node(&Node::Heading((&heading).clone())),
                raw_content: extract_raw_content(&Node::Heading((&heading).clone()), original_content),
                level: Some((&heading).depth as u32),
                line,
                column,
                children,
                wiki_links: extract_wiki_links(&Node::Heading((&heading).clone())),
                ordered: None,
                resolved: false,
            }
        },
        Node::Paragraph(paragraph) => {
            let children = (&paragraph).clone().children.into_iter()
                .map(|child| convert_mdast_node(child, original_content, line, column))
                .collect();
                
            DokeNode {
                node_type: "DokeNode".to_string(),
                markdown_element: "paragraph".to_string(),
                content: extract_text_content_from_node(&Node::Paragraph((&paragraph).clone())),
                raw_content: extract_raw_content(&Node::Paragraph((&paragraph).clone()), original_content),
                level: None,
                line,
                column,
                children,
                wiki_links: extract_wiki_links(&Node::Paragraph((&paragraph).clone())),
                ordered: None,
                resolved: false,
            }
        },
        Node::List(list) => {
            let children = (&list).clone().children.into_iter()
                .map(|child| convert_mdast_node(child, original_content, line, column))
                .collect();
                
            DokeNode {
                node_type: "DokeNode".to_string(),
                markdown_element: "list".to_string(),
                content: None,
                raw_content: extract_raw_content(&Node::List((&list).clone()), original_content),
                level: None,
                line,
                column,
                children,
                wiki_links: extract_wiki_links(&Node::List((&list).clone())),
                ordered: Some((&list).ordered),
                resolved: false,
            }
        },
        Node::ListItem(item) => {
            let children = (&item).clone().children.into_iter()
                .map(|child| convert_mdast_node(child, original_content, line, column))
                .collect();
                
            DokeNode {
                node_type: "DokeNode".to_string(),
                markdown_element: "list_item".to_string(),
                content: extract_text_content_from_node(&Node::ListItem((&item).clone())),
                raw_content: extract_raw_content(&Node::ListItem((&item).clone()), original_content),
                level: None,
                line,
                column,
                children,
                wiki_links: extract_wiki_links(&Node::ListItem((&item).clone())),
                ordered: None,
                resolved: false,
            }
        },
        Node::Text(text) => {
            DokeNode {
                node_type: "DokeNode".to_string(),
                markdown_element: "text".to_string(),
                content: extract_text_content_from_node(&Node::Text((&text).clone())),
                raw_content: (&text).value.clone(),
                level: None,
                line,
                column,
                children: Vec::new(),
                wiki_links: extract_wiki_links_from_text(&(&text).value),
                ordered: None,
                resolved: false,
            }
        },
        _ => {
            DokeNode {
                node_type: "DokeNode".to_string(),
                markdown_element: "unknown".to_string(),
                content: None,
                raw_content: String::new(),
                level: None,
                line,
                column,
                children: Vec::new(),
                wiki_links: Vec::new(),
                ordered: None,
                resolved: false,
            }
        }
    }
}
// Remove the old extract_text_content function and replace it with the new implementation
fn extract_text_content_from_heading(heading: &markdown::mdast::Heading) -> Option<String> {
    let mut content = String::new();
    for child in &heading.children {
        if let Some(text) = extract_text_content_from_node(child) {
            content.push_str(&text);
        }
    }
    Some(content)
}

fn extract_text_content_from_paragraph(paragraph: &markdown::mdast::Paragraph) -> Option<String> {
    let mut content = String::new();
    for child in &paragraph.children {
        if let Some(text) = extract_text_content_from_node(child) {
            content.push_str(&text);
        }
    }
    Some(content)
}

fn extract_text_content_from_list_item(item: &markdown::mdast::ListItem) -> Option<String> {
    let mut content = String::new();
    for child in &item.children {
        if let Some(text) = extract_text_content_from_node(child) {
            content.push_str(&text);
        }
    }
    Some(content)
}

fn extract_text_content_from_node(node: &markdown::mdast::Node) -> Option<String> {
    match node {
        markdown::mdast::Node::Text(text) => Some(text.value.clone()),
        markdown::mdast::Node::Heading(heading) => extract_text_content_from_heading(heading),
        markdown::mdast::Node::Paragraph(paragraph) => extract_text_content_from_paragraph(paragraph),
        markdown::mdast::Node::ListItem(item) => extract_text_content_from_list_item(item),
        markdown::mdast::Node::Strong(strong) => {
            let mut content = String::new();
            for child in &strong.children {
                if let Some(text) = extract_text_content_from_node(child) {
                    content.push_str(&text);
                }
            }
            Some(content)
        },
        markdown::mdast::Node::Emphasis(emphasis) => {
            let mut content = String::new();
            for child in &emphasis.children {
                if let Some(text) = extract_text_content_from_node(child) {
                    content.push_str(&text);
                }
            }
            Some(content)
        },
        markdown::mdast::Node::InlineCode(inline_code) => Some(inline_code.value.clone()),
        markdown::mdast::Node::Link(link) => {
            let mut content = String::new();
            for child in &link.children {
                if let Some(text) = extract_text_content_from_node(child) {
                    content.push_str(&text);
                }
            }
            Some(content)
        },
        markdown::mdast::Node::Image(image) => Some(image.alt.clone()),
        markdown::mdast::Node::Delete(delete) => {
            let mut content = String::new();
            for child in &delete.children {
                if let Some(text) = extract_text_content_from_node(child) {
                    content.push_str(&text);
                }
            }
            Some(content)
        },
        _ => None,
    }
}

fn extract_raw_content(node: &Node, original_content: &str) -> String {
    // For now, return a simplified version - in a real implementation,
    // you'd use position information to extract the exact content
    match node {
        Node::Heading(heading) => {
            format!("Heading level {}", heading.depth)
        },
        Node::Paragraph(paragraph) => {
            "Paragraph content".to_string()
        },
        Node::List(list) => {
            if list.ordered {
                "Ordered list".to_string()
            } else {
                "Unordered list".to_string()
            }
        },
        Node::ListItem(item) => {
            "List item".to_string()
        },
        Node::Text(text) => {
            text.value.clone()
        },
        _ => String::new(),
    }
}

fn extract_text_content(node: &Node) -> Option<String> {
    match node {
        Node::Text(text) => Some(text.value.clone()),
        Node::Heading(heading) => {
            let mut content = String::new();
            for child in &heading.children {
                if let Some(text) = extract_text_content(child) {
                    content.push_str(&text);
                }
            }
            Some(content)
        },
        Node::Paragraph(paragraph) => {
            let mut content = String::new();
            for child in &paragraph.children {
                if let Some(text) = extract_text_content(child) {
                    content.push_str(&text);
                }
            }
            Some(content)
        },
        Node::ListItem(item) => {
            let mut content = String::new();
            for child in &item.children {
                if let Some(text) = extract_text_content(child) {
                    content.push_str(&text);
                }
            }
            Some(content)
        },
        _ => None,
    }
}

fn extract_wiki_links(node: &Node) -> Vec<ResourceLink> {
    let mut links = Vec::new();
    match node {
        Node::Text(text) => {
            links.extend(extract_wiki_links_from_text(&text.value));
        },
        Node::Heading(heading) => {
            for child in &heading.children {
                links.extend(extract_wiki_links(child));
            }
        },
        Node::Paragraph(paragraph) => {
            for child in &paragraph.children {
                links.extend(extract_wiki_links(child));
            }
        },
        Node::List(list) => {
            for child in &list.children {
                links.extend(extract_wiki_links(child));
            }
        },
        Node::ListItem(item) => {
            for child in &item.children {
                links.extend(extract_wiki_links(child));
            }
        },
        _ => {}
    }
    links
}

fn extract_wiki_links_from_text(text: &str) -> Vec<ResourceLink> {
    let wiki_link_regex = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let mut links = Vec::new();
    
    for cap in wiki_link_regex.captures_iter(text) {
        if let Some(link_text) = cap.get(1) {
            links.push(ResourceLink {
                resource_type: None,
                resource_name: link_text.as_str().to_string(),
                resolved: false,
            });
        }
    }
    
    links
}

fn parse_yaml_to_value(yaml: &Yaml, result: &mut HashMap<String, Value>, current_path: &str) {
    match yaml {
        Yaml::Hash(hash) => {
            for (key, value) in hash {
                if let Yaml::String(key_str) = key {
                    let new_path = if current_path.is_empty() {
                        key_str.clone()
                    } else {
                        format!("{}.{}", current_path, key_str)
                    };
                    parse_yaml_to_value(value, result, &new_path);
                }
            }
        },
        Yaml::String(s) => {
            result.insert(current_path.to_string(), Value::String(s.clone()));
        },
        Yaml::Integer(i) => {
            result.insert(current_path.to_string(), Value::Number((*i).into()));
        },
        Yaml::Real(f) => {
            if let Ok(num) = f.parse::<f64>() {
                result.insert(current_path.to_string(), Value::Number(serde_json::Number::from_f64(num).unwrap()));
            }
        },
        Yaml::Boolean(b) => {
            result.insert(current_path.to_string(), Value::Bool(*b));
        },
        Yaml::Array(arr) => {
            let values: Vec<Value> = arr.iter().map(|item| {
                let mut temp_map = HashMap::new();
                parse_yaml_to_value(item, &mut temp_map, "");
                Value::Object(temp_map.into_iter().collect())
            }).collect();
            result.insert(current_path.to_string(), Value::Array(values));
        },
        Yaml::Null => {
            result.insert(current_path.to_string(), Value::Null);
        },
        _ => {}
    }

    fn extract_text_content_from_heading(heading: &markdown::mdast::Heading) -> Option<String> {
        let mut content = String::new();
        for child in &heading.children {
            if let Some(text) = extract_text_content_from_node(child) {
                content.push_str(&text);
            }
        }
        Some(content)
    }

    fn extract_text_content_from_paragraph(paragraph: &markdown::mdast::Paragraph) -> Option<String> {
        let mut content = String::new();
        for child in &paragraph.children {
            if let Some(text) = extract_text_content_from_node(child) {
                content.push_str(&text);
            }
        }
        Some(content)
    }

    fn extract_text_content_from_list_item(item: &markdown::mdast::ListItem) -> Option<String> {
        let mut content = String::new();
        for child in &item.children {
            if let Some(text) = extract_text_content_from_node(child) {
                content.push_str(&text);
            }
        }
        Some(content)
    }

    fn extract_text_content_from_node(node: &markdown::mdast::Node) -> Option<String> {
        match node {
            markdown::mdast::Node::Text(text) => Some(text.value.clone()),
            markdown::mdast::Node::Heading(heading) => extract_text_content_from_heading(heading),
            markdown::mdast::Node::Paragraph(paragraph) => extract_text_content_from_paragraph(paragraph),
            markdown::mdast::Node::ListItem(item) => extract_text_content_from_list_item(item),
            markdown::mdast::Node::Strong(strong) => {
                let mut content = String::new();
                for child in &strong.children {
                    if let Some(text) = extract_text_content_from_node(child) {
                        content.push_str(&text);
                    }
                }
                Some(content)
            },
            markdown::mdast::Node::Emphasis(emphasis) => {
                let mut content = String::new();
                for child in &emphasis.children {
                    if let Some(text) = extract_text_content_from_node(child) {
                        content.push_str(&text);
                    }
                }
                Some(content)
            },
            markdown::mdast::Node::InlineCode(inline_code) => Some(inline_code.value.clone()),
            markdown::mdast::Node::Link(link) => {
                let mut content = String::new();
                for child in &link.children {
                    if let Some(text) = extract_text_content_from_node(child) {
                        content.push_str(&text);
                    }
                }
                Some(content)
            },
            markdown::mdast::Node::Image(image) => Some(image.alt.clone()),
            markdown::mdast::Node::Delete(delete) => {
                let mut content = String::new();
                for child in &delete.children {
                    if let Some(text) = extract_text_content_from_node(child) {
                        content.push_str(&text);
                    }
                }
                Some(content)
            },
            _ => None,
        }
    }



}





















#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_context() -> ParserContext {
        ParserContext::new(
            "/dokedex",
            "/project",
            "Test",
            "test.md",
            "DokeMarkdownParser"
        )
    }

    #[test]
    fn test_frontmatter_parsing() -> DokeResult<()> {
        let content = r#"---
id: test_001
name: "Test Item"
price: 100
tags: [common, test]
---
This is the body content"#;

        let (frontmatter, body) = parse_frontmatter(content)?;
        
        assert_eq!(frontmatter.get("id"), Some(&Value::String("test_001".to_string())));
        assert_eq!(frontmatter.get("name"), Some(&Value::String("Test Item".to_string())));
        assert_eq!(frontmatter.get("price"), Some(&Value::Number(100.into())));
        assert_eq!(body, "This is the body content");
        
        Ok(())
    }

    #[test]
    fn test_no_frontmatter() -> DokeResult<()> {
        let content = "This is content without frontmatter";
        let (frontmatter, body) = parse_frontmatter(content)?;
        
        assert!(frontmatter.is_empty());
        assert_eq!(body, "This is content without frontmatter");
        
        Ok(())
    }

    #[test]
    fn test_wiki_link_extraction() {
        let text = "This has [[WikiLink]] and [[Another Resource]] with some text";
        let links = extract_wiki_links_from_text(text);
        
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].resource_name, "WikiLink");
        assert_eq!(links[1].resource_name, "Another Resource");
        assert!(!links[0].resolved);
        assert!(!links[1].resolved);
    }

    #[test]
    fn test_markdown_body_parsing() -> DokeResult<()> {
        let content = r#"# Heading 1
This is a paragraph with [[WikiLink]].

## Heading 2
- List item 1
- List item 2 with [[AnotherLink]]
- List item 3"#;

        let context = create_test_context();
        let nodes = parse_markdown_body(content, &context)?;
        
        // Should have 3 top-level nodes: heading, paragraph, heading
        assert_eq!(nodes.len(), 3);
        
        // First node should be heading
        assert_eq!(nodes[0].markdown_element, "heading");
        assert_eq!(nodes[0].level, Some(1));
        
        // Second node should be paragraph with wiki link
        assert_eq!(nodes[1].markdown_element, "paragraph");
        assert_eq!(nodes[1].wiki_links.len(), 1);
        assert_eq!(nodes[1].wiki_links[0].resource_name, "WikiLink");
        
        // Third node should be heading with list children
        assert_eq!(nodes[2].markdown_element, "heading");
        assert_eq!(nodes[2].level, Some(2));
        assert!(!nodes[2].children.is_empty());
        assert_eq!(nodes[2].children[0].markdown_element, "list");
        
        Ok(())
    }

    #[test]
    fn test_ordered_list_parsing() -> DokeResult<()> {
        let content = r#"1. First item
2. Second item with [[Resource]]
3. Third item"#;

        let context = create_test_context();
        let nodes = parse_markdown_body(content, &context)?;
        
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].markdown_element, "list");
        assert_eq!(nodes[0].ordered, Some(true));
        assert_eq!(nodes[0].children.len(), 3);
        assert_eq!(nodes[0].children[1].wiki_links.len(), 1);
        assert_eq!(nodes[0].children[1].wiki_links[0].resource_name, "Resource");
        
        Ok(())
    }

    #[test]
    fn test_full_doke_parser() -> DokeResult<()> {
        let content = r#"---
id: test_full
name: "Full Test"
---
# Test Document

This is a test paragraph with [[TestResource]].

## Features
- Feature 1
- Feature 2 with [[FeatureResource]]
- [[StandaloneResource]]

## Steps
1. Step one
2. Step two with [[StepResource]]"#;

        let parser = DokeMarkdownParser;
        let context = create_test_context();
        let result = parser.parse(content, &context)?;
        
        // Check frontmatter
        let frontmatter = result.get("frontmatter").unwrap().as_object().unwrap();
        assert_eq!(frontmatter.get("id"), Some(&Value::String("test_full".to_string())));
        assert_eq!(frontmatter.get("name"), Some(&Value::String("Full Test".to_string())));
        
        // Check body structure
        let body = result.get("body").unwrap().as_array().unwrap();
        assert!(body.len() >= 3); // heading, paragraph, heading
        
        // Check resolved flags are all false
        for node in body {
            assert_eq!(node.get("resolved").unwrap().as_bool().unwrap(), false);
        }
        
        Ok(())
    }

    #[test]
    fn test_complex_frontmatter() -> DokeResult<()> {
        let content = r#"---
name: "Complex Item"
stats:
  health: 100
  damage: 25
  defense: 10
tags: [weapon, melee, rare]
---
Body content"#;

        let (frontmatter, _) = parse_frontmatter(content)?;
        
        // Test flat properties
        assert_eq!(frontmatter.get("name"), Some(&Value::String("Complex Item".to_string())));
        
        // Test nested properties
        assert_eq!(frontmatter.get("stats.health"), Some(&Value::Number(100.into())));
        assert_eq!(frontmatter.get("stats.damage"), Some(&Value::Number(25.into())));
        assert_eq!(frontmatter.get("stats.defense"), Some(&Value::Number(10.into())));
        
        // Test array
        let tags = frontmatter.get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 3);
        
        Ok(())
    }

    #[test]
    fn test_malformed_frontmatter() {
        let content = r#"---
name: "Test
unclosed: string
---
Body"#;

        let result = parse_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_content() -> DokeResult<()> {
        let content = "";
        let (frontmatter, body) = parse_frontmatter(content)?;
        
        assert!(frontmatter.is_empty());
        assert_eq!(body, "");
        
        Ok(())
    }

    #[test]
    fn test_text_content_extraction() -> DokeResult<()> {
        let content = r#"# Heading with **bold** and _italic_
        
Paragraph with [[Link]] and `code`."#;

        let context = create_test_context();
        let nodes = parse_markdown_body(content, &context)?;
        
        // Should extract text content properly
        let heading = &nodes[0];
        assert!(heading.content.as_ref().unwrap().contains("Heading with bold and italic"));
        
        let paragraph = &nodes[1];
        assert!(paragraph.content.as_ref().unwrap().contains("Paragraph with Link and code"));
        
        Ok(())
    }

    #[test]
    fn test_nested_structure() -> DokeResult<()> {
        let content = r#"# Main Heading

## Subheading
- Item 1
- Item 2
  - Nested item
  - Another nested with [[NestedResource]]
- Item 3"#;

        let context = create_test_context();
        let nodes = parse_markdown_body(content, &context)?;
        
        // Check nested structure
        let subheading = &nodes[1];
        assert_eq!(subheading.markdown_element, "heading");
        assert_eq!(subheading.level, Some(2));
        
        let list = &subheading.children[0];
        assert_eq!(list.markdown_element, "list");
        assert_eq!(list.children.len(), 3);
        
        let nested_list = &list.children[1].children[0];
        assert_eq!(nested_list.markdown_element, "list");
        assert_eq!(nested_list.children[1].wiki_links.len(), 1);
        assert_eq!(nested_list.children[1].wiki_links[0].resource_name, "NestedResource");
        
        Ok(())
    }

    #[test]
    fn test_parser_supported_types() {
        let parser = DokeMarkdownParser;
        let types = parser.supported_types();
        
        assert_eq!(types.len(), 3);
        assert!(types.contains(&"Markdown".to_string()));
        assert!(types.contains(&"Doke".to_string()));
        assert!(types.contains(&"Generic".to_string()));
    }

    #[test]
    fn test_parser_version() {
        let parser = DokeMarkdownParser;
        assert_eq!(parser.version(), "1.0.0");
    }

    #[test]
    fn test_wiki_links_in_different_contexts() -> DokeResult<()> {
        let content = r#"# Heading with [[HeadingLink]]

Paragraph with [[ParagraphLink]].

- List item with [[ListItemLink]]
- [[StandaloneListLink]]

> Blockquote with [[BlockquoteLink]]"#;

        let context = create_test_context();
        let nodes = parse_markdown_body(content, &context)?;
        
        // Collect all wiki links
        let mut all_links = Vec::new();
        fn collect_links(node: &DokeNode, links: &mut Vec<String>) {
            for wiki_link in &node.wiki_links {
                links.push(wiki_link.resource_name.clone());
            }
            for child in &node.children {
                collect_links(child, links);
            }
        }
        
        for node in &nodes {
            collect_links(node, &mut all_links);
        }
        
        // Should find all wiki links
        assert!(all_links.contains(&"HeadingLink".to_string()));
        assert!(all_links.contains(&"ParagraphLink".to_string()));
        assert!(all_links.contains(&"ListItemLink".to_string()));
        assert!(all_links.contains(&"StandaloneListLink".to_string()));
        assert!(all_links.contains(&"BlockquoteLink".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_mixed_markdown_elements() -> DokeResult<()> {
        let content = r#"# Mixed Elements

**Bold text** with [[BoldLink]].

*Italic text* with [[ItalicLink]].

`Code with [[CodeLink]]` but wiki link shouldn't parse here.

[Regular link](http://example.com) with [[RegularLink]]."#;

        let context = create_test_context();
        let nodes = parse_markdown_body(content, &context)?;
        
        // Should parse wiki links in appropriate contexts
        let mut found_links = Vec::new();
        fn find_links(node: &DokeNode, links: &mut Vec<String>) {
            for wiki_link in &node.wiki_links {
                links.push(wiki_link.resource_name.clone());
            }
            for child in &node.children {
                find_links(child, links);
            }
        }
        
        for node in &nodes {
            find_links(node, &mut found_links);
        }
        
        // Should find wiki links in text but not in code
        assert!(found_links.contains(&"BoldLink".to_string()));
        assert!(found_links.contains(&"ItalicLink".to_string()));
        assert!(found_links.contains(&"RegularLink".to_string()));
        assert!(!found_links.contains(&"CodeLink".to_string()));
        
        Ok(())
    }

    #[test]
    fn test_error_handling() -> DokeResult<()> {
        // Test with invalid markdown that should still parse gracefully
        let content = r#"---
valid: frontmatter
---
# Valid content

[Unclosed link

- List with [[ValidLink]]"#;

        let parser = DokeMarkdownParser;
        let context = create_test_context();
        
        // Should still parse despite invalid markdown
        let result = parser.parse(content, &context);
        assert!(result.is_ok());
        
        let parsed = result.unwrap();
        assert!(parsed.contains_key("frontmatter"));
        assert!(parsed.contains_key("body"));
        
        Ok(())
    }
}