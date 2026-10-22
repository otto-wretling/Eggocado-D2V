# D2V Agent Guidelines

## n8n Workflow Preferences

- Do not suggest Execute Command nodes or Code nodes in n8n workflows. Use built-in nodes only. Expressions inside node fields (e.g. Edit Fields) are fine.
- Do not edit the exported workflow JSON file directly, and do not make workflow edits by clicking through the n8n UI in the browser (slow and error-prone: CodeMirror's auto-bracket-closing can silently corrupt expression edits). Make all workflow edits using the n8n MCP tool (e.g. `n8n_update_partial_workflow`). Use the browser only for live testing and verification of a running workflow, never for making edits.
- When inspecting files, read them directly with the Read tool. Do not use commands or scripts (e.g. python3, grep, jq) to parse them.

## Writing style

- No em dashes (—) in user-facing messages (form descriptions, error messages, labels, dropdown option text, etc.). Use a comma, period, colon, or plain hyphen instead, whichever reads most naturally.

## Commit messages

- Never use the word "real" (or similar meaningless emphasis like "actual", "genuine") as a qualifier. If something needs distinguishing, name what it's being distinguished from; otherwise drop the word entirely.
- Never truncate or abbreviate a word in the title (e.g. "dedup" for "deduplication", "config" for "configuration"). Spell it out in full.
- Keep the title at or under 50 characters, 72 as a hard ceiling. GitHub and most git tools truncate titles past ~72 characters with an ellipsis, and that cut lands mid-word regardless of what you wrote — a title that's too long causes exactly the mid-word truncation this file already forbids, just via the renderer instead of your own wording. If spelling a word out in full would push the title past the ceiling, cut other content from the title and move it to the body instead; never shorten a word to make it fit.
- Never state the current state of a specific machine or instance (e.g. "still unpublished in n8n") — that's true only right now, on one system, and is wrong or irrelevant the moment anything changes or anyone else reads the history.
- Never describe how the commit was produced (exported via MCP vs. the UI, typed by hand vs. generated) or who asked for it and when. The commit message documents the change, not the circumstances of making it.
