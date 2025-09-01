# Copilot Instructions – Pull Request Descriptions

## Goal
When generating a Pull Request description, provide a clear, developer-oriented summary with consistent formatting.

## Expected Structure

1. **Summary**
   - One or two sentences explaining the purpose of the PR.

2. **Changes**
   - Use bullet points.
   - Each item should have a clear title followed by a short explanation.

3. **Before / After Examples**
   - Always include minimal examples to illustrate behavior changes or new features.
   - Examples must be shown in fenced code blocks.
   - Two acceptable formats:

   ### A. Full Code Snippets
   ```markdown
   ### Before
   ```json
   {
     "host": "localhost",
     "port": 5432
   }
   ```

   ### After
   ```json
   {
     "host": "localhost",
     "port": 5432,
     "username": "admin"
   }
   ```
   ```

   ### B. Clean Diff
   ```diff
   {
     "host": "localhost",
     "port": 5432
   -}
   +, "username": "admin"}
   ```
   - Allowed: `+` and `-` to indicate additions/removals.  
   - Not allowed: diffhunks (`@@ ... @@`) or truncated context.

4. **Notes**
   - If the change has special implications (migrations, dependencies, compatibility issues), add a *Notes* section.

## Style
- Be concise and to the point.
- Avoid long sentences or unnecessary explanations.
- Always show examples inside code blocks.
- **Never use `@@` diffhunks or omit relevant lines. Show the minimal but complete change.**
