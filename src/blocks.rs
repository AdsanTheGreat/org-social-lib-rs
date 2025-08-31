//! Org-mode block parsing and handling.
//!
//! This module provides functionality to parse and manage org-mode blocks
//! such as code blocks, quotes, examples, etc.
use std::collections::HashMap;

/// Represents a collapsible org-mode block
#[derive(Debug, Clone, PartialEq)]
pub struct OrgBlock {
    pub block_type: String,
    pub attributes: Option<String>,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub is_collapsed: bool,
}

/// Represents the position and type of an activatable element
#[derive(Debug, Clone, PartialEq)]
pub enum ActivatableElement {
    Block(OrgBlock),
}

impl ActivatableElement {
    pub fn start_line(&self) -> usize {
        match self {
            ActivatableElement::Block(block) => block.start_line,
        }
    }

    pub fn end_line(&self) -> usize {
        match self {
            ActivatableElement::Block(block) => block.end_line,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        match self {
            ActivatableElement::Block(block) => block.is_collapsed,
        }
    }

    pub fn toggle_collapsed(&mut self) {
        match self {
            ActivatableElement::Block(block) => block.is_collapsed = !block.is_collapsed,
        }
    }

    pub fn get_summary(&self) -> String {
        match self {
            ActivatableElement::Block(block) => {
                let type_display = match block.block_type.to_lowercase().as_str() {
                    "src" => "Code block",
                    "quote" => "Quote block",
                    "example" => "Example block",
                    "verse" => "Verse block",
                    _ => "Block",
                };
                
                if let Some(attrs) = &block.attributes {
                    format!("{type_display} ({attrs})")
                } else {
                    type_display.to_string()
                }
            }
        }
    }

    pub fn get_content(&self) -> &str {
        match self {
            ActivatableElement::Block(block) => &block.content,
        }
    }
}

/// Parse org-mode blocks from content and return activatable elements
pub fn parse_blocks(content: &str) -> Vec<ActivatableElement> {
    let lines: Vec<&str> = content.lines().collect();
    let mut elements = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        
        // Check for block start
        if line.starts_with("#+begin_") || line.starts_with("#+BEGIN_") {
            if let Some(element) = parse_block_from_line(i, &lines) {
                i = element.end_line() + 1;
                elements.push(element);
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    elements
}

/// Parse a single block starting from the given line index
fn parse_block_from_line(start_line: usize, lines: &[&str]) -> Option<ActivatableElement> {
    let start_line_content = lines[start_line].trim();
    
    // Extract block type and attributes
    let begin_prefix = if start_line_content.starts_with("#+begin_") {
        "#+begin_"
    } else if start_line_content.starts_with("#+BEGIN_") {
        "#+BEGIN_"
    } else {
        return None;
    };

    let after_begin = &start_line_content[begin_prefix.len()..];
    let parts: Vec<&str> = after_begin.splitn(2, ' ').collect();
    let block_type = parts[0].to_lowercase();
    let attributes = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };

    // Find matching end
    let end_pattern_lower = format!("#+end_{block_type}");
    let end_pattern_upper = format!("#+END_{}", block_type.to_uppercase());
    
    let mut content_lines = Vec::new();
    let mut end_line = None;

    for (idx, &line) in lines.iter().enumerate().skip(start_line + 1) {
        let trimmed = line.trim();
        if trimmed == end_pattern_lower || trimmed == end_pattern_upper {
            end_line = Some(idx);
            break;
        }
        content_lines.push(line);
    }

    if let Some(end_line) = end_line {
        let content = content_lines.join("\n");
        let block = OrgBlock {
            block_type,
            attributes,
            content,
            start_line,
            end_line,
            is_collapsed: false, // Default to expanded
        };
        Some(ActivatableElement::Block(block))
    } else {
        None
    }
}

/// Process content with collapsed blocks, returning modified content and block positions
pub fn process_content_with_blocks(content: &str, collapsed_blocks: &HashMap<usize, bool>) -> (String, Vec<ActivatableElement>) {
    let mut elements = parse_blocks(content);
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines = Vec::new();
    let mut line_idx = 0;

    // Update collapse state from provided map
    for element in &mut elements {
        if let Some(&is_collapsed) = collapsed_blocks.get(&element.start_line()) {
            match element {
                ActivatableElement::Block(block) => block.is_collapsed = is_collapsed,
            }
        }
    }

    while line_idx < lines.len() {
        // Check if we're at the start of a block
        if let Some(element) = elements.iter().find(|e| e.start_line() == line_idx) {
            if element.is_collapsed() {
                // Add collapsed representation
                let summary = element.get_summary();
                result_lines.push(format!("[+] {summary} [...]"));
                line_idx = element.end_line() + 1;
            } else {
                // Add the full block content
                for i in element.start_line()..=element.end_line() {
                    if i < lines.len() {
                        result_lines.push(lines[i].to_string());
                    }
                }
                line_idx = element.end_line() + 1;
            }
        } else {
            // Regular line
            result_lines.push(lines[line_idx].to_string());
            line_idx += 1;
        }
    }

    (result_lines.join("\n"), elements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_code_block() {
        let content = r#"Some text before
#+begin_src rust
fn hello() {
    println!("Hello, world!");
}
#+end_src
Some text after"#;

        let elements = parse_blocks(content);
        assert_eq!(elements.len(), 1);
        
        let ActivatableElement::Block(block) = &elements[0];
        assert_eq!(block.block_type, "src");
        assert_eq!(block.attributes, Some("rust".to_string()));
        assert_eq!(block.start_line, 1);
        assert_eq!(block.end_line, 5);
        assert!(block.content.contains("fn hello()"));
    }

    #[test]
    fn test_parse_quote_block() {
        let content = r#"Text before
#+begin_quote
This is a quote
with multiple lines
#+end_quote
Text after"#;

        let elements = parse_blocks(content);
        assert_eq!(elements.len(), 1);
        
        let ActivatableElement::Block(block) = &elements[0];
        assert_eq!(block.block_type, "quote");
        assert_eq!(block.attributes, None);
        assert_eq!(block.start_line, 1);
        assert_eq!(block.end_line, 4);
        assert!(block.content.contains("This is a quote"));
    }

    #[test]
    fn test_process_collapsed_blocks() {
        let content = r#"Text before
#+begin_src rust
fn test() {}
#+end_src
Text after"#;

        let mut collapsed_blocks = HashMap::new();
        collapsed_blocks.insert(1, true); // Collapse the block starting at line 1

        let (processed_content, elements) = process_content_with_blocks(content, &collapsed_blocks);
        
        assert!(processed_content.contains("[+] Code block (rust) [...]"));
        assert!(!processed_content.contains("fn test()"));
        assert_eq!(elements.len(), 1);
    }

    #[test]
    fn test_multiple_blocks() {
        let content = r#"Text before
#+begin_src python
print("hello")
#+end_src
Middle text
#+begin_quote
A quote here
#+end_quote
Text after"#;

        let elements = parse_blocks(content);
        assert_eq!(elements.len(), 2);
        
        let ActivatableElement::Block(block1) = &elements[0];
        assert_eq!(block1.block_type, "src");
        assert_eq!(block1.start_line, 1);
        
        let ActivatableElement::Block(block2) = &elements[1];
        assert_eq!(block2.block_type, "quote");
        assert_eq!(block2.start_line, 5);
    }
}
