# D2V Agent Guidelines

## n8n Workflow Preferences

- Do not suggest Execute Command nodes or Code nodes in n8n workflows. Use built-in nodes only. Expressions inside node fields (e.g. Edit Fields) are fine.
- Do not edit n8n workflow JSON files directly. Describe the changes for the user to make in the n8n UI.
- When inspecting files, read them directly with the Read tool. Do not use commands or scripts (e.g. python3, grep, jq) to parse them.

## Writing style

- No em dashes (—) in user-facing messages (form descriptions, error messages, labels, dropdown option text, etc.). Use a comma, period, colon, or plain hyphen instead, whichever reads most naturally.
