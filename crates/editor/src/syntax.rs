#[cfg(feature = "tree-sitter")]
use tree_sitter::{Parser, Tree, Language};
#[cfg(feature = "tree-sitter")]
use tree_sitter_latex;

pub struct SyntaxHighlighter {
    #[cfg(feature = "tree-sitter")]
    parser: Parser,
    #[cfg(feature = "tree-sitter")]
    tree: Option<Tree>,
}

#[cfg(feature = "tree-sitter")]
impl SyntaxHighlighter {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_latex::LANGUAGE.into()).unwrap();
        
        Self {
            parser,
            tree: None,
        }
    }
    
    pub fn parse(&mut self, text: &str) {
        self.tree = self.parser.parse(text, self.tree.as_ref());
    }
    
    pub fn get_highlights(&self, start_line: usize, end_line: usize) -> Vec<Highlight> {
        let mut highlights = Vec::new();
        
        if let Some(tree) = &self.tree {
            let root_node = tree.root_node();
            self.collect_highlights(&root_node, start_line, end_line, &mut highlights);
        }
        
        highlights
    }
    
    fn collect_highlights(&self, node: &tree_sitter::Node, start_line: usize, end_line: usize, highlights: &mut Vec<Highlight>) {
        let start_pos = node.start_position();
        let end_pos = node.end_position();
        
        if start_pos.row >= start_line && end_pos.row <= end_line {
            let kind = node.kind();
            let highlight_type = match kind {
                "command" | "begin" | "end" => HighlightType::Keyword,
                "text" => HighlightType::Text,
                "math" | "inline_math" => HighlightType::Math,
                "comment" => HighlightType::Comment,
                "string" => HighlightType::String,
                _ => HighlightType::Text,
            };
            
            highlights.push(Highlight {
                start_line: start_pos.row,
                start_col: start_pos.column,
                end_line: end_pos.row,
                end_col: end_pos.column,
                highlight_type,
            });
        }
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.collect_highlights(&child, start_line, end_line, highlights);
            }
        }
    }
}

#[cfg(not(feature = "tree-sitter"))]
impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn parse(&mut self, _text: &str) {
        // No-op for web builds
    }
    
    pub fn get_highlights(&self, _start_line: usize, _end_line: usize) -> Vec<Highlight> {
        // Return empty highlights for web builds
        Vec::new()
    }
}

#[derive(Clone, Debug)]
pub struct Highlight {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub highlight_type: HighlightType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HighlightType {
    Keyword,
    Text,
    Math,
    Comment,
    String,
}

impl HighlightType {
    pub fn to_class(&self) -> &'static str {
        match self {
            HighlightType::Keyword => "text-blue-600 font-bold",
            HighlightType::Text => "text-gray-900 dark:text-gray-100",
            HighlightType::Math => "text-green-600 dark:text-green-400",
            HighlightType::Comment => "text-gray-500 italic",
            HighlightType::String => "text-orange-600 dark:text-orange-400",
        }
    }
}