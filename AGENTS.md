# D2V Agent Guidelines

## Code style

- No code comments. Name functions, variables, and types so the code explains itself; a reader should never need a comment to understand what something does or why.
- A self-documenting name states what it actually checks or does, not a generic verb. `verify_srt` tells a reader unfamiliar with the project nothing; `assert_srt_has_well_formed_cues` does. Prefer the specific claim over the generic action every time.

## n8n Workflow Preferences

- Do not suggest Execute Command nodes or Code nodes in n8n workflows. Use built-in nodes only. Expressions inside node fields (e.g. Edit Fields) are fine.
- Do not edit the exported workflow JSON file directly, and do not make workflow edits by clicking through the n8n UI in the browser (slow and error-prone: CodeMirror's auto-bracket-closing can silently corrupt expression edits). Make all workflow edits using the n8n MCP tool (e.g. `n8n_update_partial_workflow`). Use the browser only for live testing and verification of a running workflow, never for making edits.
- When inspecting files, read them directly with the Read tool. Do not use commands or scripts (e.g. python3, grep, jq) to parse them.

## Writing style

- No em dashes (—) in user-facing messages (form descriptions, error messages, labels, dropdown option text, etc.), or in GitHub issues, pull requests, or comments. Use a comma, period, colon, or plain hyphen instead, whichever reads most naturally.
- In documentation, commit messages, or any other written content, never state what something doesn't do, doesn't contain, or isn't. This includes comparative phrasing that implies the same thing, like "rather than X", "instead of X", "without X", "not just X", or "no X". State only what it actually is or does.
- Never use "real", "actual", "genuine", "tested" or similar meaningless emphasis as a qualifier, in documentation, commit messages, or any other written content. If something needs distinguishing, name what it's being distinguished from; otherwise drop the word entirely.
- Don't frame a requirement as a choice between two options when only one side is real (e.g. "keep this running in the background" falsely implies a meaningful "foreground" alternative exists). State the actual requirement directly instead.
- Don't list self-evident or trivial options as if they were meaningful guidance (e.g. "run it in a terminal tab" as a way to keep a service running). Only state specifics that are actually non-obvious or that this repo itself provides.
- Instructions tell the user what to do, never what not to do or what they don't need to do. If a row/step has no action, state the action that applies instead of noting the absence of one (e.g. "Run Continue mode again", not "Nothing to delete").

## Installation instructions

- For installing anything not part of this repo (not a Nix config, script, or other file checked in here), prefer linking to that thing's own official documentation over spelling out exact install commands, to the greatest extent possible. Official docs stay current as install methods change; hardcoded steps here don't. Don't add the install command alongside the link either, even as a convenience: it's just as likely to go stale, and there's no one keeping this file updated as each dependency's install instructions change.
- Link text should match the linked page's own title or main heading, not a paraphrase of it.

## Dockerfile package pins

- `nemo-normalizer/Dockerfile` pins `gcc` and `python3-dev` to exact apt versions for build reproducibility. When `python:3.10-slim`'s Debian release moves on and the pinned versions stop resolving, run `docker run --rm python:3.10-slim bash -c "apt-get update -qq && apt-cache policy gcc python3-dev"` against the current base image and update the pins to the versions it reports as `Candidate`.

## Commit messages

- Base commit messages and pull request descriptions only on the diff itself. Read the actual code changes and describe those; never factor in anything from the conversation that produced them (what was investigated, which approach was tried first, why a change was made, who asked for it). If the diff doesn't show it, it doesn't belong in the message.
- Only include sections the user actually asked for. A pull request description needs a summary of the change; add a test plan, checklist, or any other section only when the user asks for one, not by default.
- Never truncate or abbreviate a word in the title (e.g. "dedup" for "deduplication", "config" for "configuration"). Spell it out in full.
- Keep the title at or under 50 characters, 72 as a hard ceiling. GitHub and most git tools truncate titles past ~72 characters with an ellipsis, and that cut lands mid-word regardless of what you wrote — a title that's too long causes exactly the mid-word truncation this file already forbids, just via the renderer instead of your own wording. If spelling a word out in full would push the title past the ceiling, cut other content from the title and move it to the body instead; never shorten a word to make it fit.
- Never state the current state of a specific machine or instance (e.g. "still unpublished in n8n") — that's true only right now, on one system, and is wrong or irrelevant the moment anything changes or anyone else reads the history.
- Never describe how the commit was produced (exported via MCP vs. the UI, typed by hand vs. generated) or who asked for it and when. The commit message documents the change, not the circumstances of making it.
- Never add a `Co-Authored-By` trailer or any other AI-attribution line.
