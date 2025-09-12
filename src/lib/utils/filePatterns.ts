/**
 * Shared file pattern utilities for consistent filtering across the application
 * Prevents infinite loops by filtering out generated/temporary files from various processes
 */

/**
 * Pattern to match generated/temporary files that should be excluded from:
 * - Git auto-refresh triggers  
 * - Query invalidation processes
 * - File system event processing
 * - Compilation triggers
 */
export const GENERATED_FILE_PATTERN = /\.(synctex\.gz|fls|fdb_latexmk|aux|log|out|toc|nav|snm|vrb|bbl|blg|idx|ind|ilg|glo|gls|glg|acn|acr|alg|lof|lot|dvi|ps|bcf|run\.xml|xdv)$|\.git\/|node_modules\/|\.DS_Store$|Thumbs\.db$|~$|\.tmp$|\.temp$|\.backup$|\.bak$|\.orig$|\.rej$/i;

/**
 * LaTeX-specific file extensions that are build outputs/intermediates
 */
export const LATEX_OUTPUT_EXTENSIONS = [
    '.aux', '.log', '.out', '.fdb_latexmk', '.fls', '.synctex.gz',
    '.toc', '.lof', '.lot', '.bbl', '.blg', '.idx', '.ind', '.ilg',
    '.nav', '.snm', '.vrb', '.figlist', '.makefile', '.figdir',
    '.figdist', '.figpdf', '.run.xml', '.bcf', '.glg', '.glo', '.gls',
    '.ist', '.xdy', '.acn', '.acr', '.alg', '.glsdefs', '.lol',
    '.auxlock', '.dpth', '.md5', '.auxdata', '.xdv', '.dvi', '.ps'
];

/**
 * Common temporary/system file patterns
 */
export const TEMP_FILE_PATTERNS = [
    '.tmp', '.temp', '.backup', '.bak', '.orig', '.rej',
    '.DS_Store', 'Thumbs.db', '~$'
];

/**
 * Directory patterns to exclude
 */
export const EXCLUDED_DIRECTORIES = [
    '.git/', 'node_modules/', 'target/', 'dist/', 'build/', '.cache/'
];

/**
 * Check if a file path should be excluded from git refresh triggers
 */
export function shouldSkipGitRefresh(filePath: string): boolean {
    return GENERATED_FILE_PATTERN.test(filePath);
}

/**
 * Check if a file path should be excluded from query invalidation
 */
export function shouldSkipQueryInvalidation(filePath: string): boolean {
    return GENERATED_FILE_PATTERN.test(filePath);
}

/**
 * Check if a file should trigger LaTeX compilation
 */
export function shouldTriggerLatexCompilation(filePath: string): boolean {
    const path = filePath.toLowerCase();
    
    // Include LaTeX ecosystem files
    const includeExtensions = ['.tex', '.bib', '.sty', '.cls', '.def', '.cfg', '.clo'];
    const shouldInclude = includeExtensions.some(ext => path.endsWith(ext));
    
    if (!shouldInclude) {
        return false;
    }
    
    // Exclude generated/temporary files
    if (shouldSkipGitRefresh(filePath)) {
        return false;
    }
    
    // Don't include PDF files (outputs, not inputs)
    if (path.endsWith('.pdf')) {
        return false;
    }
    
    return true;
}

/**
 * Get a human-readable description of why a file was filtered
 */
export function getFilterReason(filePath: string): string | null {
    if (LATEX_OUTPUT_EXTENSIONS.some(ext => filePath.toLowerCase().endsWith(ext))) {
        return 'LaTeX build output/intermediate file';
    }
    
    if (TEMP_FILE_PATTERNS.some(pattern => filePath.toLowerCase().includes(pattern))) {
        return 'Temporary/system file';
    }
    
    if (EXCLUDED_DIRECTORIES.some(dir => filePath.includes(dir))) {
        return 'Excluded directory';
    }
    
    if (filePath.includes('.git/')) {
        return 'Git internal file';
    }
    
    return null;
}