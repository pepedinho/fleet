# Copilot Instructions – Pull Request Descriptions

## Goal

When generating a Pull Request description, provide a clear, developer-oriented summary with consistent formatting.

## Expected Structure

1. **Title**

   * Always include a clear title summarizing the PR in one sentence.

2. **Summary**

   * One or two sentences explaining the purpose of the PR.

3. **Changes**

   * Use bullet points.
   * Each item should have a clear title followed by a short explanation.

4. **Before / After Examples**

   * q or new features.
   * Examples must be shown in fenced code blocks.
   * Two acceptable formats:

   ### A. Full Code Snippets

   ````markdown
   ### Before
   ```json
   {
     "host": "localhost",
     "port": 5432
   }
   ````

   ### After

   ```json
   {
     "host": "localhost",
     "port": 5432,
     "username": "admin"
   }
   ```

   ````

   ### B. Clean Diff
   ```diff
   {
     "host": "localhost",
     "port": 5432
   -}
   +, "username": "admin"}
   ````

   * Allowed: `+` and `-` to indicate additions/removals.
   * Not allowed: diffhunks (`@@ ... @@`) or truncated context.

5. **Notes**

   * If the change has special implications (migrations, dependencies, compatibility issues), add a *Notes* section.

6. **Style**

   * Be concise and to the point.
   * Avoid long sentences or unnecessary explanations.
   * Always show examples inside code blocks.
   * Never use `@@` diffhunks or omit relevant lines. Show the minimal but complete change.
