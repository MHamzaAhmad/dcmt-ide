use crate::*;
use crate::commands::{CommandExecutor, EditorCommand};

#[component]
pub fn TextEditor(
    #[props(default)] initial_content: Option<String>,
    #[props(default)] onchange: Option<EventHandler<String>>,
    #[props(default = true)] show_line_numbers: bool,
    #[props(default = true)] syntax_highlighting: bool,
) -> Element {
    let mut buffer = use_signal(|| TextBuffer::new());
    
    // Update buffer when initial_content changes
    use_effect(move || {
        if let Some(content) = &initial_content {
            let current_content = buffer.read().get_text();
            if current_content != *content {
                buffer.set(TextBuffer::from_str(content));
            }
        }
    });
    
    let mut cursor = use_signal(|| Cursor::new());
    let mut highlighter = use_signal(|| SyntaxHighlighter::new());
    let mut executor = use_signal(|| CommandExecutor::new());
    let _theme = use_theme();
    
    // Parse content for syntax highlighting
    use_effect(move || {
        if syntax_highlighting {
            highlighter.write().parse(&buffer.read().get_text());
        }
    });
    
    let handle_keydown = move |evt: KeyboardEvent| {
        let key = evt.key();
        let ctrl = evt.modifiers().ctrl();
        let _shift = evt.modifiers().shift();
        
        let command = match key {
            Key::Character(ch) if !ctrl => {
                Some(EditorCommand::InsertChar(ch.chars().next().unwrap_or(' ')))
            }
            Key::Backspace => Some(EditorCommand::Backspace),
            Key::Delete => Some(EditorCommand::DeleteChar),
            Key::ArrowLeft => Some(EditorCommand::MoveCursorLeft),
            Key::ArrowRight => Some(EditorCommand::MoveCursorRight),
            Key::ArrowUp => Some(EditorCommand::MoveCursorUp),
            Key::ArrowDown => Some(EditorCommand::MoveCursorDown),
            Key::Home => Some(EditorCommand::MoveToLineStart),
            Key::End => Some(EditorCommand::MoveToLineEnd),
            Key::Character(ch) if ctrl && ch == "z" => Some(EditorCommand::Undo),
            Key::Character(ch) if ctrl && ch == "y" => Some(EditorCommand::Redo),
            _ => None,
        };
        
        if let Some(cmd) = command {
            executor.write().execute(cmd, &mut buffer.write(), &mut cursor.write());
            
            // Notify parent of content change
            if let Some(handler) = &onchange {
                handler.call(buffer.read().get_text());
            }
            
            // Re-parse for syntax highlighting
            if syntax_highlighting {
                highlighter.write().parse(&buffer.read().get_text());
            }
        }
    };
    
    rsx! {
        div {
            class: "flex h-full w-full bg-white dark:bg-gray-900 font-mono text-sm",
            
            // Line numbers
            if show_line_numbers {
                div {
                    class: "select-none px-4 py-2 text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700",
                    for line_num in 1..=buffer.read().len_lines() {
                        div {
                            class: "text-right",
                            "{line_num}"
                        }
                    }
                }
            }
            
            // Editor content
            div {
                class: "flex-1 relative",
                tabindex: 0,
                onkeydown: handle_keydown,
                
                // Text content with syntax highlighting
                div {
                    class: "p-2",
                    for (line_idx, line) in buffer.read().get_text().lines().enumerate() {
                        div {
                            class: "min-h-[1.5rem]",
                            if syntax_highlighting {
                                RenderHighlightedLine {
                                    line: line.to_string(),
                                    line_idx: line_idx,
                                    highlighter: highlighter,
                                }
                            } else {
                                span {
                                    class: "text-gray-900 dark:text-gray-100",
                                    "{line}"
                                }
                            }
                        }
                    }
                }
                
                // Cursor
                RenderCursor {
                    cursor: cursor,
                    buffer: buffer,
                }
            }
        }
    }
}

#[component]
fn RenderHighlightedLine(
    line: String,
    line_idx: usize,
    highlighter: Signal<SyntaxHighlighter>,
) -> Element {
    let highlights = highlighter.read().get_highlights(line_idx, line_idx);
    
    rsx! {
        span {
            if highlights.is_empty() {
                span {
                    class: "text-gray-900 dark:text-gray-100",
                    "{line}"
                }
            } else {
                for highlight in highlights {
                    span {
                        class: "{highlight.highlight_type.to_class()}",
                        "{&line[highlight.start_col..highlight.end_col.min(line.len())]}"
                    }
                }
            }
        }
    }
}

#[component]
fn RenderCursor(
    cursor: Signal<Cursor>,
    buffer: Signal<TextBuffer>,
) -> Element {
    let cursor_pos = cursor.read().position;
    let line_height = 1.5; // rem
    let char_width = 0.6; // rem
    
    let top = cursor_pos.line as f32 * line_height;
    let left = cursor_pos.column as f32 * char_width;
    
    rsx! {
        div {
            class: "absolute w-0.5 h-6 bg-blue-600 animate-pulse",
            style: "top: {top}rem; left: {left}rem;",
        }
    }
}