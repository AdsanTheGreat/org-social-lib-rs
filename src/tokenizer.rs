/// Represents a single token in the input text.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Plain text token, fallback for when no implemented syntax is detected
    PlainText(String),
    /// Bold text token
    Bold(String),
    /// Italic text token
    Italic(String),
    /// Bold and italic text token
    BoldItalic(String),
    /// Link token
    Link {
        url: String,
        description: Option<String>,
    },
    /// Org-social mention token
    Mention {
        url: String,
        username: String,
    },
    /// Inline code token
    InlineCode(String),
}

pub struct Tokenizer {
    input: Vec<char>,
    position: usize,
}

impl Tokenizer {
    pub fn new(input: String) -> Self {
        Self { 
            input: input.chars().collect(),
            position: 0 
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        
        while self.position < self.input.len() {
            if let Some(token) = self.next_token() {
                tokens.push(token);
            }
        }
        
        tokens
    }

    fn next_token(&mut self) -> Option<Token> {
        if self.position >= self.input.len() {
            return None;
        }

        // Check for mentions first [[org-social:url][username]]
        if self.peek_chars(2) == "[[" {
            if let Some(token) = self.parse_mention() {
                return Some(token);
            }
            // If it's not a mention, try parsing as a regular link
            return self.parse_link();
        }

        // Check for URLs (protocol://...) before checking for italic text
        if let Some(token) = self.parse_url() {
            return Some(token);
        }

        // Check for bold italic */text/*
        if self.peek_chars(2) == "*/" {
            return self.parse_bold_italic();
        }

        // Check for bold *text*
        if self.peek_char() == '*' && self.position + 1 < self.input.len() {
            if let Some(token) = self.parse_bold() {
                return Some(token);
            }
        }

        // Check for italic /text/
        if self.peek_char() == '/' && self.position + 1 < self.input.len() {
            if let Some(token) = self.parse_italic() {
                return Some(token);
            }
        }

        // Check for inline code ~text~
        if self.peek_char() == '~' {
            if let Some(token) = self.parse_inline_code() {
                return Some(token);
            }
        }

        // Default to plain text
        self.parse_plain_text()
    }

    fn parse_link(&mut self) -> Option<Token> {
        if self.peek_chars(2) != "[[" {
            return None;
        }

        self.advance(2); // Skip [[
        let start = self.position;
        
        // Find the closing ]]
        while self.position < self.input.len() {
            if self.peek_chars(2) == "]]" {
                let link_content: String = self.input[start..self.position].iter().collect();
                self.advance(2); // Skip ]]
                
                // Check if it has description [url][description]
                if let Some(bracket_pos) = link_content.find("][") {
                    let url = link_content[..bracket_pos].to_string();
                    let description = link_content[bracket_pos + 2..].to_string();
                    return Some(Token::Link {
                        url,
                        description: Some(description),
                    });
                } else {
                    return Some(Token::Link {
                        url: link_content,
                        description: None,
                    });
                }
            }
            self.advance(1);
        }
        
        None
    }

    fn parse_mention(&mut self) -> Option<Token> {
        if self.peek_chars(2) != "[[" {
            return None;
        }

        let saved_position = self.position;
        self.advance(2); // Skip [[
        let start = self.position;
        
        // Find the closing ]]
        while self.position < self.input.len() {
            if self.peek_chars(2) == "]]" {
                let content: String = self.input[start..self.position].iter().collect();
                
                // Check if it's a mention: org-social:url][username
                if let Some(bracket_pos) = content.find("][") {
                    let url_part = &content[..bracket_pos];
                    let username = content[bracket_pos + 2..].to_string();
                    
                    // Check if URL part starts with "org-social:"
                    if url_part.starts_with("org-social:") {
                        let url = url_part[11..].to_string(); // Remove "org-social:" prefix
                        self.advance(2); // Skip ]]
                        return Some(Token::Mention { url, username });
                    }
                }
                
                // Not a mention, reset position
                self.position = saved_position;
                return None;
            }
            self.advance(1);
        }
        
        // Reset position if we didn't find a valid mention
        self.position = saved_position;
        None
    }

    fn parse_url(&mut self) -> Option<Token> {
        let start_pos = self.position;
        
        // Only try to parse URL if we're at the start of a word (alphabetic character)
        if !self.peek_char().is_alphabetic() {
            return None;
        }
        
        // Check for http:// or https:// specifically
        let http_prefix = "http://";
        let https_prefix = "https://";
        
        let remaining_chars: String = self.input[self.position..].iter().take(8).collect();
        
        let protocol_len = if remaining_chars.starts_with(https_prefix) {
            8 // "https://" length
        } else if remaining_chars.starts_with(http_prefix) {
            7 // "http://" length
        } else {
            return None; // Not an HTTP/HTTPS URL
        };
        
        // Move past the protocol
        self.position += protocol_len;
        
        // Continue parsing the rest of the URL
        while self.position < self.input.len() {
            let ch = self.peek_char();
            if ch.is_whitespace() || ch == ')' || ch == ']' || ch == '>' 
                || ch == '"' || ch == '\'' || ch == '*' || ch == '~' {
                break;
            }
            self.advance(1);
        }
        
        let url: String = self.input[start_pos..self.position].iter().collect();
        Some(Token::Link {
            url,
            description: None,
        })
    }

    fn parse_bold_italic(&mut self) -> Option<Token> {
        if self.peek_chars(2) != "*/" {
            return None;
        }

        self.advance(2); // Skip */
        let start = self.position;
        
        // Find closing /*
        while self.position < self.input.len() - 1 {
            if self.peek_chars(2) == "/*" {
                let content: String = self.input[start..self.position].iter().collect();
                self.advance(2); // Skip /*
                return Some(Token::BoldItalic(content));
            }
            self.advance(1);
        }
        
        None
    }

    fn parse_bold(&mut self) -> Option<Token> {
        if self.peek_char() != '*' {
            return None;
        }

        self.advance(1); // Skip *
        let start = self.position;
        
        // Find closing *
        while self.position < self.input.len() {
            if self.peek_char() == '*' {
                let content: String = self.input[start..self.position].iter().collect();
                if !content.is_empty() && !content.contains('\n') {
                    self.advance(1); // Skip closing *
                    return Some(Token::Bold(content));
                } else {
                    break;
                }
            }
            self.advance(1);
        }
        
        // Reset position if we didn't find a valid bold
        self.position = start - 1;
        None
    }

    fn parse_italic(&mut self) -> Option<Token> {
        if self.peek_char() != '/' {
            return None;
        }

        self.advance(1); // Skip /
        let start = self.position;
        
        // Find closing /
        while self.position < self.input.len() {
            if self.peek_char() == '/' {
                let content: String = self.input[start..self.position].iter().collect();
                if !content.is_empty() && !content.contains('\n') {
                    self.advance(1); // Skip closing /
                    return Some(Token::Italic(content));
                } else {
                    break;
                }
            }
            self.advance(1);
        }
        
        // Reset position if we didn't find a valid italic
        self.position = start - 1;
        None
    }

    fn parse_inline_code(&mut self) -> Option<Token> {
        if self.peek_char() != '~' {
            return None;
        }

        self.advance(1); // Skip ~
        let start = self.position;
        
        // Find closing ~
        while self.position < self.input.len() {
            if self.peek_char() == '~' {
                let content: String = self.input[start..self.position].iter().collect();
                if !content.is_empty() {
                    self.advance(1); // Skip closing ~
                    return Some(Token::InlineCode(content));
                } else {
                    break;
                }
            }
            self.advance(1);
        }
        
        // Reset position if we didn't find a valid code block
        self.position = start - 1;
        None
    }

    fn parse_plain_text(&mut self) -> Option<Token> {
        let start = self.position;
        
        // Consume characters until we hit a special character or potential URL
        while self.position < self.input.len() {
            let ch = self.peek_char();
            if ch == '*' || ch == '/' || ch == '~' || ch == '[' {
                break;
            }
            
            // If we hit a potential URL, check if it's actually a URL
            if ch.is_alphabetic() {
                // Save current position to check for URL
                let check_position = self.position;
                if self.try_parse_url_from_current_position().is_some() {
                    // We found a URL! First return any plain text we've accumulated
                    if check_position > start {
                        // Reset position to just before the URL
                        self.position = check_position;
                        let content: String = self.input[start..self.position].iter().collect();
                        return Some(Token::PlainText(content));
                    } else {
                        // We're at the start of a URL, parse it
                        return self.parse_url();
                    }
                }
            }
            
            self.advance(1);
        }
        
        if self.position > start {
            let content: String = self.input[start..self.position].iter().collect();
            Some(Token::PlainText(content))
        } else {
            // If we're at a special character but couldn't parse it, 
            // consume it as plain text
            self.advance(1);
            let content: String = self.input[start..self.position].iter().collect();
            Some(Token::PlainText(content))
        }
    }

    fn try_parse_url_from_current_position(&mut self) -> Option<Token> {
        let saved_position = self.position;
        let result = self.parse_url();
        if result.is_none() {
            // Restore position if we didn't find a URL
            self.position = saved_position;
        }
        result
    }

    fn peek_char(&self) -> char {
        self.input.get(self.position).copied().unwrap_or('\0')
    }

    fn peek_chars(&self, count: usize) -> String {
        let end = std::cmp::min(self.position + count, self.input.len());
        self.input[self.position..end].iter().collect()
    }

    fn advance(&mut self, count: usize) {
        self.position = std::cmp::min(self.position + count, self.input.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let mut tokenizer = Tokenizer::new("Hello world".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![Token::PlainText("Hello world".to_string())]);
    }

    #[test]
    fn test_bold_text() {
        let mut tokenizer = Tokenizer::new("This is *bold* text".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("This is ".to_string()),
            Token::Bold("bold".to_string()),
            Token::PlainText(" text".to_string()),
        ]);
    }

    #[test]
    fn test_italic_text() {
        let mut tokenizer = Tokenizer::new("This is /italic/ text".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("This is ".to_string()),
            Token::Italic("italic".to_string()),
            Token::PlainText(" text".to_string()),
        ]);
    }

    #[test]
    fn test_bold_italic_text() {
        let mut tokenizer = Tokenizer::new("This is */bold italic/* text".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("This is ".to_string()),
            Token::BoldItalic("bold italic".to_string()),
            Token::PlainText(" text".to_string()),
        ]);
    }

    #[test]
    fn test_link_without_description() {
        let mut tokenizer = Tokenizer::new("Visit [[https://example.com]] for more".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Visit ".to_string()),
            Token::Link {
                url: "https://example.com".to_string(),
                description: None,
            },
            Token::PlainText(" for more".to_string()),
        ]);
    }

    #[test]
    fn test_link_with_description() {
        let mut tokenizer = Tokenizer::new("Visit [[https://example.com][Example Site]] for more".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Visit ".to_string()),
            Token::Link {
                url: "https://example.com".to_string(),
                description: Some("Example Site".to_string()),
            },
            Token::PlainText(" for more".to_string()),
        ]);
    }

    #[test]
    fn test_inline_code() {
        let mut tokenizer = Tokenizer::new("Use ~println!~ to print".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Use ".to_string()),
            Token::InlineCode("println!".to_string()),
            Token::PlainText(" to print".to_string()),
        ]);
    }

    #[test]
    fn test_mixed_formatting() {
        let mut tokenizer = Tokenizer::new("*Bold* and /italic/ with [[https://example.com][link]]".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::Bold("Bold".to_string()),
            Token::PlainText(" and ".to_string()),
            Token::Italic("italic".to_string()),
            Token::PlainText(" with ".to_string()),
            Token::Link {
                url: "https://example.com".to_string(),
                description: Some("link".to_string()),
            },
        ]);
    }

    #[test]
    fn test_utf8_text() {
        let mut tokenizer = Tokenizer::new("Hello 世界 *bold 中文* text".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Hello 世界 ".to_string()),
            Token::Bold("bold 中文".to_string()),
            Token::PlainText(" text".to_string()),
        ]);
    }

    #[test]
    fn test_url_http() {
        let mut tokenizer = Tokenizer::new("Visit http://example.com for more info".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Visit ".to_string()),
            Token::Link {
                url: "http://example.com".to_string(),
                description: None,
            },
            Token::PlainText(" for more info".to_string()),
        ]);
    }

    #[test]
    fn test_url_https() {
        let mut tokenizer = Tokenizer::new("Check https://secure.example.com/path?query=value".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Check ".to_string()),
            Token::Link {
                url: "https://secure.example.com/path?query=value".to_string(),
                description: None,
            },
        ]);
    }

    #[test]
    fn test_url_mixed_protocols() {
        let mut tokenizer = Tokenizer::new("Check https://secure.example.com and http://example.com".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Check ".to_string()),
            Token::Link {
                url: "https://secure.example.com".to_string(),
                description: None,
            },
            Token::PlainText(" and ".to_string()),
            Token::Link {
                url: "http://example.com".to_string(),
                description: None,
            },
        ]);
    }

    #[test]
    fn test_non_http_protocols_not_parsed() {
        let mut tokenizer = Tokenizer::new("Connect via ftp://files.example.com or matrix://matrix.org".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Connect via ftp:".to_string()),
            Token::PlainText("/".to_string()),
            Token::Italic("files.example.com or matrix:".to_string()),
            Token::PlainText("/".to_string()),
            Token::PlainText("matrix.org".to_string()),
        ]);
    }

    #[test]
    fn test_italic_vs_url() {
        let mut tokenizer = Tokenizer::new("This is /italic/ but this is https://example.com/path not italic".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("This is ".to_string()),
            Token::Italic("italic".to_string()),
            Token::PlainText(" but this is ".to_string()),
            Token::Link {
                url: "https://example.com/path".to_string(),
                description: None,
            },
            Token::PlainText(" not italic".to_string()),
        ]);
    }

    #[test]
    fn test_mention_basic() {
        let mut tokenizer = Tokenizer::new("Contact [[org-social:http://example.org/social.org][username]]".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Contact ".to_string()),
            Token::Mention {
                url: "http://example.org/social.org".to_string(),
                username: "username".to_string(),
            },
        ]);
    }

    #[test]
    fn test_mention_with_https() {
        let mut tokenizer = Tokenizer::new("Hello [[org-social:https://social.example.com/user.org][alice]]!".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Hello ".to_string()),
            Token::Mention {
                url: "https://social.example.com/user.org".to_string(),
                username: "alice".to_string(),
            },
            Token::PlainText("!".to_string()),
        ]);
    }

    #[test]
    fn test_mention_mixed_with_links() {
        let mut tokenizer = Tokenizer::new("Visit [[https://example.com][site]] and talk to [[org-social:http://social.org/bob.org][bob]]".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Visit ".to_string()),
            Token::Link {
                url: "https://example.com".to_string(),
                description: Some("site".to_string()),
            },
            Token::PlainText(" and talk to ".to_string()),
            Token::Mention {
                url: "http://social.org/bob.org".to_string(),
                username: "bob".to_string(),
            },
        ]);
    }

    #[test]
    fn test_mention_without_org_social_prefix_fallback_to_link() {
        let mut tokenizer = Tokenizer::new("This is [[http://example.com][regular link]]".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("This is ".to_string()),
            Token::Link {
                url: "http://example.com".to_string(),
                description: Some("regular link".to_string()),
            },
        ]);
    }

    #[test]
    fn test_mention_complex_username() {
        let mut tokenizer = Tokenizer::new("Message [[org-social:https://myorg.example.com/profiles/alice.org][alice_123@domain]]".to_string());
        let tokens = tokenizer.tokenize();
        assert_eq!(tokens, vec![
            Token::PlainText("Message ".to_string()),
            Token::Mention {
                url: "https://myorg.example.com/profiles/alice.org".to_string(),
                username: "alice_123@domain".to_string(),
            },
        ]);
    }
}