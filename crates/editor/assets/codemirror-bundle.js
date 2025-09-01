// CodeMirror 6 Bundle for LaTeX IDE
// This file provides a self-contained CodeMirror 6 setup with LaTeX support

(function(window) {
    'use strict';
    
    // Check if already loaded
    if (window.CodeMirrorLaTeXIDE) {
        console.log('CodeMirror LaTeX IDE bundle already loaded');
        return;
    }
    
    // Module loading state
    let modulesLoaded = false;
    let loadingPromise = null;
    
    // CodeMirror 6 bundle configuration
    const CM6_CONFIG = {
        version: '6.26.0',
        modules: {
            'view': 'https://esm.sh/@codemirror/view@6.26.0',
            'state': 'https://esm.sh/@codemirror/state@6.4.1', 
            'language': 'https://esm.sh/@codemirror/language@6.10.1',
            'autocomplete': 'https://esm.sh/@codemirror/autocomplete@6.15.0',
            'commands': 'https://esm.sh/@codemirror/commands@6.3.3',
            'search': 'https://esm.sh/@codemirror/search@6.5.6',
            'highlight': 'https://esm.sh/@lezer/highlight@1.2.0',
            'basicSetup': 'https://esm.sh/codemirror@6.0.1/dist/index.js',
            'latex': 'https://esm.sh/@codemirror/lang-latex@6.0.0'
        }
    };
    
    // LaTeX-specific configuration
    const LATEX_CONFIG = {
        completions: [
            // Document structure
            {label: '\\section', type: 'keyword', apply: '\\section{${}}', info: 'Document section'},
            {label: '\\subsection', type: 'keyword', apply: '\\subsection{${}}', info: 'Document subsection'},
            {label: '\\subsubsection', type: 'keyword', apply: '\\subsubsection{${}}', info: 'Document subsubsection'},
            
            // Environments
            {label: '\\begin{document}', type: 'keyword', apply: '\\begin{document}\n\t${}\n\\end{document}', info: 'Document environment'},
            {label: '\\begin{equation}', type: 'keyword', apply: '\\begin{equation}\n\t${}\n\\end{equation}', info: 'Equation environment'},
            {label: '\\begin{align}', type: 'keyword', apply: '\\begin{align}\n\t${}\n\\end{align}', info: 'Align environment'},
            {label: '\\begin{gather}', type: 'keyword', apply: '\\begin{gather}\n\t${}\n\\end{gather}', info: 'Gather environment'},
            {label: '\\begin{itemize}', type: 'keyword', apply: '\\begin{itemize}\n\t\\item ${}\n\\end{itemize}', info: 'Itemize list'},
            {label: '\\begin{enumerate}', type: 'keyword', apply: '\\begin{enumerate}\n\t\\item ${}\n\\end{enumerate}', info: 'Enumerate list'},
            {label: '\\begin{figure}', type: 'keyword', apply: '\\begin{figure}[htbp]\n\t\\centering\n\t${}\n\t\\caption{}\n\\end{figure}', info: 'Figure environment'},
            {label: '\\begin{table}', type: 'keyword', apply: '\\begin{table}[htbp]\n\t\\centering\n\t${}\n\t\\caption{}\n\\end{table}', info: 'Table environment'},
            
            // Math functions
            {label: '\\frac', type: 'function', apply: '\\frac{${}}{${}}', info: 'Fraction'},
            {label: '\\sqrt', type: 'function', apply: '\\sqrt{${}}', info: 'Square root'},
            {label: '\\sum', type: 'function', apply: '\\sum_{${}}^{${}}', info: 'Summation'},
            {label: '\\int', type: 'function', apply: '\\int_{${}}^{${}}', info: 'Integral'},
            {label: '\\prod', type: 'function', apply: '\\prod_{${}}^{${}}', info: 'Product'},
            {label: '\\lim', type: 'function', apply: '\\lim_{${} \\to ${}}', info: 'Limit'},
            
            // Greek letters
            {label: '\\alpha', type: 'constant', apply: '\\alpha', info: 'Greek letter alpha'},
            {label: '\\beta', type: 'constant', apply: '\\beta', info: 'Greek letter beta'},
            {label: '\\gamma', type: 'constant', apply: '\\gamma', info: 'Greek letter gamma'},
            {label: '\\delta', type: 'constant', apply: '\\delta', info: 'Greek letter delta'},
            {label: '\\epsilon', type: 'constant', apply: '\\epsilon', info: 'Greek letter epsilon'},
            {label: '\\theta', type: 'constant', apply: '\\theta', info: 'Greek letter theta'},
            {label: '\\lambda', type: 'constant', apply: '\\lambda', info: 'Greek letter lambda'},
            {label: '\\mu', type: 'constant', apply: '\\mu', info: 'Greek letter mu'},
            {label: '\\pi', type: 'constant', apply: '\\pi', info: 'Greek letter pi'},
            {label: '\\sigma', type: 'constant', apply: '\\sigma', info: 'Greek letter sigma'},
            {label: '\\phi', type: 'constant', apply: '\\phi', info: 'Greek letter phi'},
            {label: '\\omega', type: 'constant', apply: '\\omega', info: 'Greek letter omega'},
            
            // Text formatting
            {label: '\\textbf', type: 'function', apply: '\\textbf{${}}', info: 'Bold text'},
            {label: '\\textit', type: 'function', apply: '\\textit{${}}', info: 'Italic text'},
            {label: '\\textsc', type: 'function', apply: '\\textsc{${}}', info: 'Small caps text'},
            {label: '\\emph', type: 'function', apply: '\\emph{${}}', info: 'Emphasized text'},
            {label: '\\underline', type: 'function', apply: '\\underline{${}}', info: 'Underlined text'},
            
            // Common packages
            {label: '\\usepackage{amsmath}', type: 'keyword', apply: '\\usepackage{amsmath}', info: 'AMS Math package'},
            {label: '\\usepackage{graphicx}', type: 'keyword', apply: '\\usepackage{graphicx}', info: 'Graphics package'},
            {label: '\\usepackage{hyperref}', type: 'keyword', apply: '\\usepackage{hyperref}', info: 'Hyperlinks package'},
            {label: '\\usepackage{geometry}', type: 'keyword', apply: '\\usepackage{geometry}', info: 'Page geometry package'}
        ],
        
        themes: {
            light: {
                '&': {
                    color: '#1f2937',
                    backgroundColor: '#ffffff'
                },
                '.cm-content': {
                    padding: '16px',
                    minHeight: '400px'
                },
                '.cm-focused': {
                    outline: '2px solid #3b82f6',
                    outlineOffset: '-2px'
                },
                '.cm-latex-command': {
                    color: '#0969da',
                    fontWeight: '600'
                },
                '.cm-latex-environment': {
                    color: '#8250df'
                },
                '.cm-latex-math': {
                    color: '#0550ae',
                    fontStyle: 'italic'
                },
                '.cm-latex-comment': {
                    color: '#6e7781',
                    fontStyle: 'italic'
                },
                '.cm-addition': {
                    backgroundColor: 'rgba(34, 197, 94, 0.2)',
                    borderBottom: '2px solid rgb(34, 197, 94)'
                },
                '.cm-deletion': {
                    backgroundColor: 'rgba(239, 68, 68, 0.2)',
                    textDecoration: 'line-through'
                },
                '.cm-modification': {
                    backgroundColor: 'rgba(251, 191, 36, 0.2)',
                    borderBottom: '2px solid rgb(251, 191, 36)'
                },
                '.cm-sync-highlight': {
                    backgroundColor: 'rgba(168, 85, 247, 0.3)',
                    borderLeft: '3px solid rgb(168, 85, 247)'
                }
            },
            dark: {
                '&': {
                    color: '#f9fafb',
                    backgroundColor: '#1f2937'
                },
                '.cm-content': {
                    padding: '16px',
                    minHeight: '400px'
                },
                '.cm-focused': {
                    outline: '2px solid #60a5fa',
                    outlineOffset: '-2px'
                },
                '.cm-latex-command': {
                    color: '#60a5fa',
                    fontWeight: '600'
                },
                '.cm-latex-environment': {
                    color: '#a78bfa'
                },
                '.cm-latex-math': {
                    color: '#93c5fd',
                    fontStyle: 'italic'
                },
                '.cm-latex-comment': {
                    color: '#9ca3af',
                    fontStyle: 'italic'
                },
                '.cm-addition': {
                    backgroundColor: 'rgba(34, 197, 94, 0.3)',
                    borderBottom: '2px solid rgb(74, 222, 128)'
                },
                '.cm-deletion': {
                    backgroundColor: 'rgba(239, 68, 68, 0.3)',
                    textDecoration: 'line-through'
                },
                '.cm-modification': {
                    backgroundColor: 'rgba(251, 191, 36, 0.3)',
                    borderBottom: '2px solid rgb(253, 224, 71)'
                },
                '.cm-sync-highlight': {
                    backgroundColor: 'rgba(168, 85, 247, 0.4)',
                    borderLeft: '3px solid rgb(196, 181, 253)'
                }
            }
        }
    };
    
    // Async module loader with caching
    async function loadCodeMirrorModules() {
        if (modulesLoaded) {
            return window.CM6;
        }
        
        if (loadingPromise) {
            return loadingPromise;
        }
        
        console.log('Loading CodeMirror 6 modules...');
        
        loadingPromise = (async () => {
            try {
                const modulePromises = Object.entries(CM6_CONFIG.modules).map(async ([name, url]) => {
                    try {
                        const module = await import(url);
                        return [name, module];
                    } catch (error) {
                        console.warn(`Failed to load CodeMirror module ${name}:`, error);
                        return [name, null];
                    }
                });
                
                const modules = await Promise.all(modulePromises);
                const CM6 = {};
                
                for (const [name, module] of modules) {
                    if (module) {
                        CM6[name] = module;
                    }
                }
                
                // Store globally
                window.CM6 = CM6;
                modulesLoaded = true;
                
                console.log('CodeMirror 6 modules loaded successfully:', Object.keys(CM6));
                return CM6;
                
            } catch (error) {
                console.error('Failed to load CodeMirror modules:', error);
                throw error;
            }
        })();
        
        return loadingPromise;
    }
    
    // Enhanced editor factory
    class CodeMirrorLaTeXFactory {
        constructor() {
            this.editors = new Map();
            this.decorationSets = new Map();
        }
        
        async createEditor(container, config = {}) {
            const {
                content = '',
                theme = 'light',
                enableLatex = true,
                enableAutocompletion = true,
                enableLineNumbers = true,
                enableLineWrapping = true,
                readOnly = false,
                onChange = null,
                onSelectionChange = null
            } = config;
            
            // Ensure modules are loaded
            const CM6 = await loadCodeMirrorModules();
            
            if (!container) {
                throw new Error('Container element is required');
            }
            
            // Build extensions array
            const extensions = [];
            
            // Basic setup
            if (CM6.basicSetup && CM6.basicSetup.basicSetup) {
                extensions.push(CM6.basicSetup.basicSetup);
            } else {
                // Fallback extensions
                const basicExtensions = [
                    enableLineNumbers && CM6.view.lineNumbers(),
                    CM6.view.highlightActiveLineGutter(),
                    CM6.view.highlightSpecialChars(),
                    CM6.view.history(),
                    CM6.view.foldGutter(),
                    CM6.view.drawSelection(),
                    CM6.view.dropCursor(),
                    CM6.state.EditorState.allowMultipleSelections.of(true),
                    CM6.language.indentOnInput(),
                    CM6.language.bracketMatching(),
                    CM6.view.closeBrackets(),
                    enableAutocompletion && CM6.autocomplete.autocompletion(),
                    CM6.view.rectangularSelection(),
                    CM6.view.crosshairCursor(),
                    CM6.view.highlightSelectionMatches(),
                    CM6.view.keymap.of([
                        ...CM6.commands.defaultKeymap,
                        ...CM6.commands.historyKeymap,
                        ...CM6.commands.foldKeymap,
                        ...(enableAutocompletion ? CM6.commands.completionKeymap : [])
                    ])
                ].filter(ext => ext);
                
                extensions.push(...basicExtensions);
            }
            
            // LaTeX language support
            if (enableLatex && CM6.latex && CM6.latex.latex) {
                extensions.push(CM6.latex.latex());
            }
            
            // LaTeX completions
            if (enableLatex && enableAutocompletion && CM6.autocomplete.completeFromList) {
                extensions.push(CM6.autocomplete.completeFromList(LATEX_CONFIG.completions));
            }
            
            // Line wrapping
            if (enableLineWrapping) {
                extensions.push(CM6.view.EditorView.lineWrapping);
            }
            
            // Theme
            if (LATEX_CONFIG.themes[theme] && CM6.view.EditorView.theme) {
                extensions.push(CM6.view.EditorView.theme(LATEX_CONFIG.themes[theme]));
            }
            
            // Read-only mode
            if (readOnly) {
                extensions.push(CM6.state.EditorState.readOnly.of(true));
            }
            
            // Change handler
            if (onChange) {
                extensions.push(CM6.view.EditorView.updateListener.of((update) => {
                    if (update.docChanged) {
                        onChange(update.state.doc.toString());
                    }
                }));
            }
            
            // Selection change handler
            if (onSelectionChange) {
                extensions.push(CM6.view.EditorView.updateListener.of((update) => {
                    if (update.selectionSet) {
                        const selection = update.state.selection.main;
                        onSelectionChange({
                            from: selection.from,
                            to: selection.to,
                            anchor: selection.anchor,
                            head: selection.head,
                            empty: selection.empty
                        });
                    }
                }));
            }
            
            // Create editor state
            const startState = CM6.state.EditorState.create({
                doc: content,
                extensions: extensions
            });
            
            // Create editor view
            const editor = new CM6.view.EditorView({
                state: startState,
                parent: container
            });
            
            // Store editor instance
            const editorId = 'editor-' + Math.random().toString(36).substr(2, 9);
            this.editors.set(editorId, editor);
            this.decorationSets.set(editorId, new Map());
            
            // Add helper methods
            editor._id = editorId;
            editor._factory = this;
            
            // Enhanced API methods
            editor.getContent = () => editor.state.doc.toString();
            editor.setContent = (newContent) => {
                const transaction = editor.state.update({
                    changes: {
                        from: 0,
                        to: editor.state.doc.length,
                        insert: newContent
                    }
                });
                editor.dispatch(transaction);
            };
            
            editor.format = () => {
                const content = editor.getContent();
                const formatted = this.formatLaTeX(content);
                editor.setContent(formatted);
            };
            
            editor.addDecoration = (from, to, className) => {
                const decorationId = 'decoration-' + Math.random().toString(36).substr(2, 9);
                const decorations = this.decorationSets.get(editorId);
                
                const mark = CM6.view.Decoration.mark({
                    class: className,
                    attributes: { 'data-decoration-id': decorationId }
                });
                
                decorations.set(decorationId, { from, to, mark });
                
                // Apply decoration (simplified - real implementation would use proper decoration sets)
                // This would require a more complex decoration state management
                
                return decorationId;
            };
            
            editor.removeDecoration = (decorationId) => {
                const decorations = this.decorationSets.get(editorId);
                if (decorations.has(decorationId)) {
                    decorations.delete(decorationId);
                    // Update decoration sets
                }
            };
            
            editor.scrollToLine = (lineNumber) => {
                const line = editor.state.doc.line(lineNumber);
                const effect = CM6.view.EditorView.scrollIntoView(line.from, { y: "center" });
                editor.dispatch({ effects: [effect] });
            };
            
            editor.highlightLine = (lineNumber, duration = 2000) => {
                const line = editor.state.doc.line(lineNumber);
                const decorationId = editor.addDecoration(line.from, line.to, 'cm-sync-highlight');
                
                setTimeout(() => {
                    editor.removeDecoration(decorationId);
                }, duration);
                
                return decorationId;
            };
            
            console.log(`CodeMirror editor created with ID: ${editorId}`);
            return editor;
        }
        
        formatLaTeX(content) {
            const lines = content.split('\n');
            const formatted = [];
            let indentLevel = 0;
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
                    indentLevel = Math.max(0, indentLevel - 1);
                }
                
                // Create indentation
                const indent = inMathEnvironment && !trimmed.startsWith('\\') ?
                    '  '.repeat(indentLevel + 1) : '  '.repeat(indentLevel);
                
                formatted.push(indent + trimmed);
                
                // Increase indent for begin tags
                if (trimmed.startsWith('\\begin{')) {
                    indentLevel++;
                }
                
                // Special handling for document structure
                if (trimmed.startsWith('\\documentclass') || trimmed.startsWith('\\usepackage')) {
                    if (formatted.length > 1 && formatted[formatted.length - 2] !== '') {
                        formatted.splice(-1, 0, '');
                    }
                }
            }
            
            return formatted.join('\n');
        }
        
        destroyEditor(editorOrId) {
            const editor = typeof editorOrId === 'string' ? 
                this.editors.get(editorOrId) : editorOrId;
                
            if (editor && editor._id) {
                editor.destroy();
                this.editors.delete(editor._id);
                this.decorationSets.delete(editor._id);
                console.log(`CodeMirror editor destroyed: ${editor._id}`);
            }
        }
        
        getAllEditors() {
            return Array.from(this.editors.values());
        }
    }
    
    // Create global factory instance
    const factory = new CodeMirrorLaTeXFactory();
    
    // Global API
    window.CodeMirrorLaTeXIDE = {
        factory,
        loadModules: loadCodeMirrorModules,
        createEditor: (container, config) => factory.createEditor(container, config),
        formatLaTeX: (content) => factory.formatLaTeX(content),
        version: CM6_CONFIG.version,
        config: LATEX_CONFIG
    };
    
    // Backward compatibility
    window.CodeMirrorFactory = {
        createEditor: (container, content = '') => {
            return factory.createEditor(container, { content });
        }
    };
    
    // Auto-load modules on first access
    Object.defineProperty(window, 'CM6', {
        get: () => {
            if (!modulesLoaded) {
                loadCodeMirrorModules();
            }
            return window._CM6_cache;
        },
        set: (value) => {
            window._CM6_cache = value;
        }
    });
    
    console.log('CodeMirror LaTeX IDE bundle initialized');
    
})(window);