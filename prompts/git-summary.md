# Git Change Summary Generator

You are analyzing git changes to help both developers and non-technical users understand what was modified.

## Your Task
Given the diff output below, provide:

1. **One-line summary** (plain English, accessible to everyone)
   - Describe WHAT changed and WHY it matters in simple terms
   - Max 100 characters
   - Example: "Updated the document editor to support real-time collaboration"

2. **Detailed changes** (3-7 bullet points)
   - Be specific but avoid technical jargon where possible
   - Focus on the purpose and impact of changes
   - Use present tense
   - Examples:
     - "• Added ability to export documents as PDF"
     - "• Fixed issue where images wouldn't load properly"
     - "• Improved performance of search functionality"

3. **Suggested commit message** (for version control)
   - Technical but concise (under 72 characters)
   - Follow conventional commit format if applicable
   - Example: "feat: add PDF export with custom formatting options"

## Input Diff
{diff_content}

## Expected Output Format (JSON)
{
  "summary": "A clear, non-technical summary of all changes",
  "bullets": [
    "• First change description",
    "• Second change description",
    "• Third change description"
  ],
  "suggestedMessage": "feat: technical commit message"
}