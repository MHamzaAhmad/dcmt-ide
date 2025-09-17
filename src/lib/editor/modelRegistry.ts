import type * as Monaco from 'monaco-editor';

/**
 * EditorModelRegistry
 * - Single place to create/reuse Monaco models keyed by file path (URI)
 * - Prevents model recreation on save and avoids cursor/view glitches
 */
export class EditorModelRegistry {
  private monaco: typeof Monaco | null = null;
  private models = new Map<string, Monaco.editor.ITextModel>();

  init(monaco: typeof Monaco) {
    if (!this.monaco) {
      this.monaco = monaco;
    }
  }

  getOrCreate(path: string, content: string, language: string): Monaco.editor.ITextModel {
    if (!this.monaco) throw new Error('EditorModelRegistry not initialized with Monaco instance');
    const uri = this.monaco.Uri.file(path);
    const existing = this.models.get(path) || this.monaco.editor.getModel(uri);
    if (existing) {
      // Ensure language is correct; don't overwrite content here to preserve undo stack
      if (language) {
        this.monaco.editor.setModelLanguage(existing, language);
      }
      this.models.set(path, existing);
      return existing;
    }

    const model = this.monaco.editor.createModel(content ?? '', language || 'plaintext', uri);
    this.models.set(path, model);
    return model;
  }

  setLanguage(path: string, language: string) {
    if (!this.monaco) return;
    const model = this.models.get(path);
    if (model) {
      this.monaco.editor.setModelLanguage(model, language);
    }
  }

  updateContent(path: string, newContent: string) {
    const model = this.models.get(path);
    if (model && model.getValue() !== newContent) {
      model.pushEditOperations(
        [],
        [{ range: model.getFullModelRange(), text: newContent }],
        () => null
      );
    }
  }

  disposeModel(path: string) {
    const model = this.models.get(path);
    if (model) {
      model.dispose();
      this.models.delete(path);
    }
  }

  disposeAll() {
    for (const [path, model] of this.models.entries()) {
      model.dispose();
      this.models.delete(path);
    }
  }
}

export const editorModelRegistry = new EditorModelRegistry();