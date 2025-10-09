# Copilot Instructions – Pull Request Descriptions

## Goal

When generating a Pull Request description, provide a clear, developer-oriented summary with consistent formatting.

## Expected Structure

1. **Title**

   * Always include a `## Title` section at the very top of the PR.
   * Do not just write the first sentence as a title; include the heading and the title text.
   * Example:

     ```markdown
     ## Title
     Fix crash when loading empty configuration files

     ## Summary
     This PR addresses a crash caused by empty configuration files by adding default checks and validations.
     ```

2. **Summary**

   * One or two sentences explaining the purpose of the PR.

3. **Changes**

   * Use `### Title of the change` as a subheading for each change
   * Follow with a brief description of the change
   * Always include minimal examples to illustrate behavior changes or new features
   * Examples must be shown in fenced code blocks with appropriate language tags
   * Use the "Before/After" format to show code changes clearly

   ***Example***
   ````markdown
   ### Title of the change
   Brief description of the change

   **Before:**
   ```rust
   pub struct Pipeline {
       pub notifications: Notification,
       pub jobs: HashMap<String, Job>,
   }
   ```

   **After:**
   ```rust
   pub struct Pipeline {
       pub notifications: Option<Notification>,
       pub jobs: HashMap<String, Job>,
   }
   ```
   ````
4. **Notes**

   * If the change has special implications (migrations, dependencies, compatibility issues), add a *Notes* section.

5. **Style**

   * Be concise and to the point.
   * Avoid long sentences or unnecessary explanations.
   * Always show examples inside code blocks.
   * Never use `@@` diffhunks or omit relevant lines. Show the minimal but complete change.

## Example PR Description Template

````markdown
## Title
<Write a one-line summary of the PR>

## Summary
<Brief explanation of the PR>

## Changes

### <Change title>
<Brief description of the change>

**Before:**
```<language>
<code before change>
```

**After:**
```<language>
<code after change>
```

### <Another change title>
<Brief description of another change>

**Before:**
```<language>
<code before change>
```

**After:**
```<language>
<code after change>
```

## Notes
<Any additional notes about migrations, dependencies, compatibility issues, etc.>
````