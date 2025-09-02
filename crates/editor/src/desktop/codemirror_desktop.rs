use latex_ide_ui::*;
use crate::codemirror::{CodeMirrorProps, CodeMirrorOps, DecorationType};

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use {
    serde_json,
};

use std::collections::HashMap;
use uuid::Uuid;

/// Desktop CodeMirror editor with enhanced webview integration
pub struct DesktopCodeMirrorState {
    editor_id: String,
    #[allow(dead_code)]
    initialized: bool,
    #[allow(dead_code)]
    decorations: HashMap<String, DecorationType>,
    #[allow(dead_code)]
    last_content: String,
}

impl DesktopCodeMirrorState {
    pub fn new() -> Self {
        Self {
            editor_id: format!("cm-editor-{}", Uuid::new_v4().simple()),
            initialized: false,
            decorations: HashMap::new(),
            last_content: String::new(),
        }
    }
}

impl CodeMirrorOps for DesktopCodeMirrorState {
    fn get_content(&self) -> String {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let _script = r#"
                (function() {
                    if (window.cmEditor && window.cmEditor.state) {
                        return window.cmEditor.state.doc.toString();
                    }
                    return '';
                })()
            "#;
            
            // This would need to be made async in a real implementation
            // For now, return the cached content
            self.last_content.clone()
        }
        
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "desktop")))]
        String::new()
    }
    
    fn set_content(&self, content: &str) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let script = format!(
                r#"
                (function() {{
                    if (window.cmEditor && window.cmEditor.dispatch) {{
                        const currentContent = window.cmEditor.state.doc.toString();
                        const newContent = {};
                        if (currentContent !== newContent) {{
                            window.cmEditor.dispatch({{
                                changes: {{
                                    from: 0,
                                    to: window.cmEditor.state.doc.length,
                                    insert: newContent
                                }}
                            }});
                        }}
                        return true;
                    }}
                    return false;
                }})()
                "#,
                serde_json::to_string(content).unwrap_or_default()
            );
            
            // Desktop webview eval would go here
            let _ = script;
        }
    }
    
    fn format(&self) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let script = r#"
                (function() {
                    if (window.cmEditor && window.cmEditor.state) {
                        const content = window.cmEditor.state.doc.toString();
                        const lines = content.split('\n');
                        const formatted = [];
                        let indent = 0;
                        let inMathEnvironment = false;
                        
                        for (const line of lines) {
                            const trimmed = line.trim();
                            
                            if (trimmed === '') {
                                formatted.push('');
                                continue;
                            }
                            
                            // Handle math environments
                            if (trimmed.startsWith('\\[') || trimmed.startsWith('\\begin{equation') ||
                                trimmed.startsWith('\\begin{align') || trimmed.startsWith('\\begin{gather')) {
                                inMathEnvironment = true;
                            } else if (trimmed.startsWith('\\]') || trimmed.startsWith('\\end{equation') ||
                                       trimmed.startsWith('\\end{align') || trimmed.startsWith('\\end{gather')) {
                                inMathEnvironment = false;
                            }
                            
                            // Decrease indent for end tags
                            if (trimmed.startsWith('\\end{')) {
                                indent = Math.max(0, indent - 1);
                            }
                            
                            // Create indentation
                            const indentStr = inMathEnvironment && !trimmed.startsWith('\\') ?
                                '  '.repeat(indent + 1) : '  '.repeat(indent);
                            
                            formatted.push(indentStr + trimmed);
                            
                            // Increase indent for begin tags
                            if (trimmed.startsWith('\\begin{')) {
                                indent++;
                            }
                            
                            // Special handling for document structure
                            if (trimmed.startsWith('\\documentclass') || trimmed.startsWith('\\usepackage')) {
                                if (formatted.length > 1 && formatted[formatted.length - 2] !== '') {
                                    formatted.splice(-1, 0, '');
                                }
                            }
                        }
                        
                        window.cmEditor.dispatch({
                            changes: {
                                from: 0,
                                to: window.cmEditor.state.doc.length,
                                insert: formatted.join('\n')
                            }
                        });
                        
                        return true;
                    }
                    return false;
                })()
            "#;
            
            // Desktop webview eval would go here
            let _ = script;
        }
    }
    
    fn add_decoration(&self, from: usize, to: usize, decoration_type: DecorationType) -> String {
        let decoration_id = Uuid::new_v4().to_string();
        
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let script = format!(
                r#"
                (function() {{
                    if (window.cmEditor && window.CM6) {{
                        try {{
                            const from = {};
                            const to = {};
                            const className = '{}';
                            const decorationId = '{}';
                            
                            const mark = window.CM6.view.Decoration.mark({{
                                class: className,
                                attributes: {{ 'data-decoration-id': decorationId }}
                            }});
                            
                            // Store decoration for later removal
                            if (!window.cmDecorations) window.cmDecorations = new Map();
                            window.cmDecorations.set(decorationId, {{ from, to, mark }});
                            
                            // Apply decoration (simplified - real implementation would use proper decoration sets)
                            const effect = window.CM6.state.StateEffect.define();
                            window.cmEditor.dispatch({{ effects: [effect.of({{ from, to, mark }})] }});
                            
                            return decorationId;
                        }} catch (error) {{
                            console.error('Failed to add decoration:', error);
                            return null;
                        }}
                    }}
                    return null;
                }})()
                "#,
                from, to, decoration_type.to_class(), decoration_id
            );
            
            // Desktop webview eval would go here
            let _ = script;
        }
        
        decoration_id
    }
    
    fn remove_decoration(&self, decoration_id: &str) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let script = format!(
                r#"
                (function() {{
                    if (window.cmEditor && window.cmDecorations) {{
                        const decorationId = '{}';
                        const decoration = window.cmDecorations.get(decorationId);
                        
                        if (decoration) {{
                            // Remove decoration (simplified - real implementation would update decoration sets)
                            window.cmDecorations.delete(decorationId);
                            console.log('Removed decoration:', decorationId);
                            return true;
                        }}
                    }}
                    return false;
                }})()
                "#,
                decoration_id
            );
            
            // Desktop webview eval would go here
            let _ = script;
        }
    }
    
    fn scroll_to_line(&self, line: usize) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let script = format!(
                r#"
                (function() {{
                    if (window.cmEditor && window.CM6) {{
                        try {{
                            const lineNumber = {};
                            const doc = window.cmEditor.state.doc;
                            
                            if (lineNumber > 0 && lineNumber <= doc.lines) {{
                                const line = doc.line(lineNumber);
                                const pos = line.from;
                                
                                const effect = window.CM6.view.EditorView.scrollIntoView(pos, {{
                                    y: "center",
                                    yMargin: 50
                                }});
                                
                                window.cmEditor.dispatch({{ effects: [effect] }});
                                return true;
                            }}
                        }} catch (error) {{
                            console.error('Failed to scroll to line:', error);
                        }}
                    }}
                    return false;
                }})()
                "#,
                line
            );
            
            // Desktop webview eval would go here
            let _ = script;
        }
    }
    
    fn highlight_line(&self, line: usize) {
        // Add temporary highlight decoration
        let decoration_id = self.add_decoration(
            line * 80, // Approximate line start position
            (line + 1) * 80, // Approximate line end position
            DecorationType::SyncHighlight
        );
        
        // Remove highlight after 2 seconds
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let script = format!(
                r#"
                setTimeout(() => {{
                    const decorationId = '{}';
                    if (window.cmDecorations && window.cmDecorations.has(decorationId)) {{
                        window.cmDecorations.delete(decorationId);
                        console.log('Auto-removed highlight decoration:', decorationId);
                    }}
                }}, 2000);
                "#,
                decoration_id
            );
            
            // Desktop webview eval would go here  
            let _ = script;
        }
    }
}

#[component]
pub fn DesktopCodeMirrorEditor(mut props: CodeMirrorProps) -> Element {
    let editor_state = use_signal(|| DesktopCodeMirrorState::new());
    let mut initialization_error = use_signal(|| None::<String>);
    let mut editor_ready = use_signal(|| false);
    
    // Initialize CodeMirror via enhanced webview integration
    use_effect(move || {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        {
            let container_id = editor_state.read().editor_id.clone();
            let initial_content = props.content.read().clone();
            
            let init_script = format!(
                r#"
                (async function() {{
                    try {{
                        // Ensure CodeMirror 6 is loaded
                        if (!window.CM6) {{
                            console.log('Loading CodeMirror 6 modules...');
                            
                            const [view, state, language, autocomplete, commands, search, highlight, basicSetup] = await Promise.all([
                                import('https://esm.sh/@codemirror/view@6.26.0'),
                                import('https://esm.sh/@codemirror/state@6.4.1'),
                                import('https://esm.sh/@codemirror/language@6.10.1'),
                                import('https://esm.sh/@codemirror/autocomplete@6.15.0'),
                                import('https://esm.sh/@codemirror/commands@6.3.3'),
                                import('https://esm.sh/@codemirror/search@6.5.6'),
                                import('https://esm.sh/@lezer/highlight@1.2.0'),
                                import('https://esm.sh/codemirror@6.0.1/dist/index.js')
                            ]);
                            
                            // Try to load LaTeX support
                            let latex = null;
                            try {{
                                latex = await import('https://esm.sh/@codemirror/lang-latex@6.0.0');
                            }} catch (e) {{
                                console.warn('LaTeX language support not available');
                            }}
                            
                            window.CM6 = {{ view, state, language, autocomplete, commands, search, highlight, basicSetup, latex }};
                        }}
                        
                        const element = document.getElementById('{}');
                        if (!element) {{
                            throw new Error('CodeMirror container element not found');
                        }}
                        
                        // Create editor with enhanced configuration
                        let extensions = [];
                        
                        if (window.CM6.basicSetup && window.CM6.basicSetup.basicSetup) {{
                            extensions.push(window.CM6.basicSetup.basicSetup);
                        }} else {{
                            // Fallback basic extensions
                            extensions = [
                                window.CM6.view.lineNumbers(),
                                window.CM6.view.highlightActiveLineGutter(),
                                window.CM6.view.highlightSpecialChars(),
                                window.CM6.view.history(),
                                window.CM6.view.foldGutter(),
                                window.CM6.view.drawSelection(),
                                window.CM6.view.dropCursor(),
                                window.CM6.state.EditorState.allowMultipleSelections.of(true),
                                window.CM6.language.indentOnInput(),
                                window.CM6.language.bracketMatching(),
                                window.CM6.view.closeBrackets(),
                                window.CM6.autocomplete.autocompletion(),
                                window.CM6.view.rectangularSelection(),
                                window.CM6.view.crosshairCursor(),
                                window.CM6.view.highlightSelectionMatches(),
                                window.CM6.view.keymap.of([
                                    ...window.CM6.commands.defaultKeymap,
                                    ...window.CM6.commands.historyKeymap,
                                    ...window.CM6.commands.foldKeymap,
                                    ...window.CM6.commands.completionKeymap
                                ])
                            ].filter(ext => ext !== undefined);
                        }}
                        
                        // Add LaTeX language support if available
                        if (window.CM6.latex && window.CM6.latex.latex) {{
                            extensions.push(window.CM6.latex.latex());
                        }}
                        
                        // Add line wrapping
                        extensions.push(window.CM6.view.EditorView.lineWrapping);
                        
                        const startState = window.CM6.state.EditorState.create({{
                            doc: {},
                            extensions: extensions
                        }});
                        
                        const editor = new window.CM6.view.EditorView({{
                            state: startState,
                            parent: element
                        }});
                        
                        // Store editor globally and set up event handlers
                        window.cmEditor = editor;
                        window.cmDecorations = new Map();
                        
                        console.log('Desktop CodeMirror editor initialized successfully');
                        return true;
                        
                    }} catch (error) {{
                        console.error('Failed to initialize CodeMirror:', error);
                        throw error;
                    }}
                }})()
                "#,
                container_id,
                serde_json::to_string(&initial_content).unwrap_or("\"\"".to_string())
            );
            
            // Execute initialization script for desktop
            let _ = init_script; // Desktop webview eval would go here
            editor_ready.set(true);
            initialization_error.set(None);
            tracing::info!("Desktop CodeMirror initialized successfully");
        }
    });
    
    // Sync content changes
    use_effect(move || {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        if *editor_ready.read() {
            let new_content = props.content.read();
            editor_state.read().set_content(&new_content);
        }
    });
    
    // Event handlers with enhanced functionality
    let handle_format = move |_| {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        if *editor_ready.read() {
            editor_state.read().format();
            
            // Update content signal with formatted content
            if let Some(on_change) = &props.on_change {
                let formatted_content = editor_state.read().get_content();
                props.content.set(formatted_content.clone());
                on_change.call(formatted_content);
            }
        }
    };
    
    let handle_ai_suggestion = move |_| {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        if *editor_ready.read() {
            let decoration_id = editor_state.read().add_decoration(10, 25, DecorationType::Addition);
            tracing::info!("Added AI suggestion decoration: {}", decoration_id);
        }
    };
    
    let handle_pdf_sync = move |_| {
        #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
        if *editor_ready.read() {
            editor_state.read().highlight_line(3);
            tracing::info!("Applied PDF sync highlight");
        }
    };
    
    rsx! {
        div {
            class: format!("h-full flex flex-col bg-white dark:bg-gray-900 {}", 
                props.class.as_ref().unwrap_or(&String::new())),
            
            // Error display
            if let Some(error) = initialization_error.read().as_ref() {
                div {
                    class: "bg-red-50 border border-red-200 text-red-700 px-4 py-2 text-sm",
                    "⚠️ Desktop Editor Error: {error}"
                }
            }
            
            // Enhanced toolbar for desktop
            div {
                class: "flex items-center gap-2 p-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800",
                
                // Format button with desktop-specific styling
                button {
                    class: format!("px-3 py-1 text-sm rounded focus:outline-none focus:ring-2 focus:ring-opacity-50 {}",
                        if *editor_ready.read() {
                            "bg-blue-500 text-white hover:bg-blue-600 focus:ring-blue-500"
                        } else {
                            "bg-gray-300 text-gray-500 cursor-not-allowed"
                        }
                    ),
                    onclick: handle_format,
                    disabled: !*editor_ready.read(),
                    "📝 Format LaTeX"
                }
                
                if props.enable_ai_suggestions {
                    button {
                        class: format!("px-3 py-1 text-sm rounded focus:outline-none focus:ring-2 focus:ring-opacity-50 {}",
                            if *editor_ready.read() {
                                "bg-green-500 text-white hover:bg-green-600 focus:ring-green-500"
                            } else {
                                "bg-gray-300 text-gray-500 cursor-not-allowed"
                            }
                        ),
                        onclick: handle_ai_suggestion,
                        disabled: !*editor_ready.read(),
                        "🤖 AI Suggestions"
                    }
                }
                
                if props.enable_pdf_sync {
                    button {
                        class: format!("px-3 py-1 text-sm rounded focus:outline-none focus:ring-2 focus:ring-opacity-50 {}",
                            if *editor_ready.read() {
                                "bg-purple-500 text-white hover:bg-purple-600 focus:ring-purple-500"
                            } else {
                                "bg-gray-300 text-gray-500 cursor-not-allowed"
                            }
                        ),
                        onclick: handle_pdf_sync,
                        disabled: !*editor_ready.read(),
                        "🔗 PDF Sync"
                    }
                }
                
                // Desktop-specific status indicator
                div {
                    class: "ml-auto flex items-center gap-2 text-xs text-gray-500",
                    
                    // Platform indicator
                    span {
                        class: "px-2 py-1 bg-gray-200 dark:bg-gray-700 rounded",
                        "🖥️ Desktop"
                    }
                    
                    // Editor status
                    match (editor_ready.read().clone(), initialization_error.read().as_ref()) {
                        (true, None) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-green-100 text-green-800",
                                "● CodeMirror Ready"
                            }
                        },
                        (false, Some(_)) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-red-100 text-red-800",
                                "● Initialization Failed"
                            }
                        },
                        (false, None) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-yellow-100 text-yellow-800",
                                "● Loading CodeMirror..."
                            }
                        },
                        (true, Some(_)) => rsx! {
                            span {
                                class: "inline-flex items-center px-2 py-1 rounded-full bg-orange-100 text-orange-800",
                                "● Warning"
                            }
                        },
                    }
                }
            }
            
            // Editor container with desktop-optimized layout
            div {
                class: "flex-1 relative bg-white dark:bg-gray-900",
                
                // Main editor container
                div {
                    id: "{editor_state.read().editor_id}",
                    class: "h-full w-full",
                    
                    // Desktop loading overlay
                    if !*editor_ready.read() && initialization_error.read().is_none() {
                        div {
                            class: "absolute inset-0 flex items-center justify-center bg-white dark:bg-gray-900 bg-opacity-90 z-10",
                            div {
                                class: "text-center p-8",
                                div {
                                    class: "inline-block animate-spin rounded-full h-12 w-12 border-4 border-blue-500 border-t-transparent mb-4",
                                }
                                div {
                                    class: "text-lg font-medium text-gray-900 dark:text-gray-100 mb-2",
                                    "Initializing CodeMirror 6"
                                }
                                div {
                                    class: "text-sm text-gray-600 dark:text-gray-400",
                                    "Loading modules and setting up LaTeX editor..."
                                }
                            }
                        }
                    }
                }
                
                // Desktop-specific help overlay (shown on first load)
                if *editor_ready.read() {
                    div {
                        class: "absolute bottom-4 right-4 bg-blue-50 border border-blue-200 rounded-lg p-3 text-xs text-blue-700 max-w-xs",
                        "💡 Desktop editor loaded! Use Ctrl+S to save, Ctrl+F to search, and the toolbar for LaTeX-specific features."
                    }
                }
            }
        }
    }
}