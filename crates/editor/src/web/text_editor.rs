use dioxus::prelude::*;
use dioxus_signals::{Signal, Readable, Writable, GlobalSignal};
use dioxus_hooks::{use_signal, use_effect};
use crate::{TextBuffer, Cursor, SyntaxHighlighter, commands::CommandExecutor};

#[component]
pub fn WebTextEditor(content: Signal<String>) -> Element {
    let mut buffer = use_signal(|| TextBuffer::from_str(&content.read()));
    let mut cursor = use_signal(|| Cursor::new());
    let mut highlighter = use_signal(|| SyntaxHighlighter::new());
    let mut executor = use_signal(|| CommandExecutor::new());
    
    // Update buffer when external content changes
    use_effect(move || {
        let current_content = buffer.read().get_text();
        let new_content = content.read();
        if current_content != *new_content {
            buffer.set(TextBuffer::from_str(&new_content));
            highlighter.write().parse(&new_content);
        }
    });
    
    // Parse content for syntax highlighting
    use_effect(move || {
        highlighter.write().parse(&buffer.read().get_text());
    });
    
    let handle_keydown = move |evt: KeyboardEvent| {
        // Prevent default behavior for handled keys
        let key = evt.key();
        let ctrl = evt.modifiers().ctrl();
        
        tracing::info!("Key pressed: {:?}, ctrl: {}", key, ctrl);
        
        let command = match key {
            Key::Character(ch) if !ctrl => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::InsertChar(ch.chars().next().unwrap_or(' ')))
            }
            Key::Backspace => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::Backspace)
            }
            Key::Delete => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::DeleteChar)
            }
            Key::ArrowLeft => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::MoveCursorLeft)
            }
            Key::ArrowRight => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::MoveCursorRight)
            }
            Key::ArrowUp => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::MoveCursorUp)
            }
            Key::ArrowDown => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::MoveCursorDown)
            }
            Key::Home => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::MoveToLineStart)
            }
            Key::End => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::MoveToLineEnd)
            }
            Key::Character(ch) if ctrl && ch == "z" => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::Undo)
            }
            Key::Character(ch) if ctrl && ch == "y" => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::Redo)
            }
            Key::Enter => {
                evt.prevent_default();
                Some(crate::commands::EditorCommand::InsertChar('\n'))
            }
            _ => None,
        };
        
        if let Some(cmd) = command {
            let cursor_before = cursor.read().position;
            tracing::info!("Executing command: {:?}, cursor before: {:?}", cmd, cursor_before);
            
            executor.write().execute(cmd, &mut buffer.write(), &mut cursor.write());
            
            let cursor_after = cursor.read().position;
            tracing::info!("Cursor after: {:?}", cursor_after);
            
            // Update parent content
            let new_content = buffer.read().get_text();
            content.set(new_content.clone());
            
            // Re-parse for syntax highlighting
            highlighter.write().parse(&new_content);
            
            tracing::info!("Buffer updated, new content length: {}, lines: {}", 
                new_content.len(), 
                new_content.lines().count()
            );
        }
    };
    
    rsx! {
        div {
            class: "flex h-full w-full bg-white dark:bg-gray-900 font-mono text-sm",
            
            // Line numbers
            div {
                class: "select-none px-4 py-2 text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700",
                for line_num in 1..=buffer.read().len_lines() {
                    div {
                        class: "text-right leading-6",
                        "{line_num}"
                    }
                }
            }
            
            // Editor content
            div {
                class: "flex-1 relative focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50",
                tabindex: 0,
                autofocus: true,
                onkeydown: handle_keydown,
                onkeypress: |evt: KeyboardEvent| {
                    // Also handle keypress to ensure we catch all input
                    tracing::info!("Key press event: {:?}", evt.key());
                },
                onclick: move |_| {
                    tracing::info!("Editor clicked - attempting to focus");
                    // Focus will happen automatically due to tabindex
                },
                onfocusin: |_| {
                    tracing::info!("Editor focused");
                },
                onfocusout: |_| {
                    tracing::info!("Editor lost focus");
                },
                
                // Text content with syntax highlighting
                div {
                    class: "p-2 cursor-text",
                    onclick: move |evt| {
                        // Position cursor based on click location
                        evt.stop_propagation();
                    },
                    {
                        let text = buffer.read().get_text();
                        let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
                        lines.into_iter().enumerate().map(|(line_idx, line)| {
                            rsx! {
                                div {
                                    key: "{line_idx}",
                                    class: "min-h-[24px] leading-6",
                                    style: "height: 24px; line-height: 24px;",
                                    onclick: move |evt| {
                                        // Calculate cursor position based on click coordinates
                                        let click_x = evt.client_coordinates().x;
                                        let click_y = evt.client_coordinates().y;
                                        
                                        // Get element position to calculate relative coordinates
                                        let relative_x = click_x - 8.0; // Account for editor padding
                                        let char_width = 9.6; // Same as cursor positioning
                                        
                                        // Calculate column position, ensure it's within line bounds
                                        let clicked_column = if relative_x < 0.0 {
                                            0
                                        } else {
                                            ((relative_x / char_width).round() as usize).min(line.len())
                                        };
                                        
                                        tracing::info!("Click at ({}, {}), relative_x: {}, line length: {}, calculated column: {}", 
                                            click_x, click_y, relative_x, line.len(), clicked_column);
                                        
                                        cursor.write().position.line = line_idx;
                                        cursor.write().position.column = clicked_column;
                                    },
                                    RenderHighlightedLine {
                                        line: line.clone(),
                                        line_idx: line_idx,
                                        highlighter: highlighter,
                                    }
                                }
                            }
                        })
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
    // For web builds, use the regex-based line highlighting (only available without tree-sitter)
    #[cfg(not(feature = "tree-sitter"))]
    let highlights = highlighter.with(|h| h.highlight_line(&line, line_idx));
    #[cfg(feature = "tree-sitter")]
    let highlights: Vec<crate::syntax::Highlight> = Vec::new(); // Fallback for desktop builds
    
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
    
    // Use pixel-based positioning like engrave
    let line_height = 24.0; // pixels, matches text line height
    let char_width = 9.6; // pixels, approximate for monospace font
    
    let top = cursor_pos.line as f32 * line_height + 8.0; // Add padding offset
    let left = cursor_pos.column as f32 * char_width + 8.0; // Add padding offset
    
    rsx! {
        div {
            class: "absolute bg-blue-600",
            style: "top: {top}px; left: {left}px; width: 2px; height: 20px; z-index: 9; display: block;",
        }
    }
}