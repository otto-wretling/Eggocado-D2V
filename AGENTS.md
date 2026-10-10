# D2V Agent Guidelines

## n8n Workflow Preferences

- Do not suggest Execute Command nodes or Code nodes in n8n workflows. Use built-in nodes only. Expressions inside node fields (e.g. Edit Fields) are fine.
- Do not edit the exported workflow JSON file directly, and do not make workflow edits by clicking through the n8n UI in the browser (slow and error-prone: CodeMirror's auto-bracket-closing can silently corrupt expression edits). Make all workflow edits using the n8n MCP tool (e.g. `n8n_update_partial_workflow`). Use the browser only for live testing and verification of a running workflow, never for making edits.
- When inspecting files, read them directly with the Read tool. Do not use commands or scripts (e.g. python3, grep, jq) to parse them.

## Writing style

- No em dashes (—) in user-facing messages (form descriptions, error messages, labels, dropdown option text, etc.). Use a comma, period, colon, or plain hyphen instead, whichever reads most naturally.
