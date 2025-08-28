//! Demo showcasing post parsing with tokens and blocks.

use org_social_lib_rs::post::Post;

fn main() {
    // Create a post with rich content including formatting and blocks
    let content = r#"This is a multi-line post with *bold* and /italic/ text.

It also includes ~inline code~ and [[https://example.com][a link]].

#+BEGIN_SRC rust
fn hello_world() {
    println!("Hello from a code block!");
}
#+END_SRC

And some more text after the block."#;

    let post = Post::new("20240828T120000".to_string(), content.to_string());
    
    println!("=== Post Information ===");
    println!("ID: {}", post.id());
    println!("Content length: {} characters", post.content().len());
    println!("Lines: {}", post.content().lines().count());
    println!("Tokens parsed: {}", post.tokens().len());
    println!("Blocks parsed: {}", post.blocks().len());
    
    println!("\n=== Parsed Tokens ===");
    for (i, token) in post.tokens().iter().enumerate() {
        println!("{}: {:?}", i + 1, token);
    }
    
    println!("\n=== Parsed Blocks ===");
    for (i, block) in post.blocks().iter().enumerate() {
        println!("{}: {:?}", i + 1, block);
    }
    
    println!("\n=== Full Content ===");
    println!("{}", post.content());
    
    // Demonstrate content modification and re-parsing
    let mut mutable_post = post;
    println!("\n=== Content Modification Demo ===");
    mutable_post.set_content("New content with **different** formatting".to_string());
    println!("After modification - Tokens: {}", mutable_post.tokens().len());
    println!("New content: {}", mutable_post.content());
}
