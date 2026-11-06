# Updating

How to pull in later changes to an existing install. See [INSTALL.md](INSTALL.md) for setting up a new one.

## Pull the Repo

```bash
git pull
```

## Nix-Managed Dependencies (NixOS, macOS, WSL)

If your system config imports this repo's flake modules (`nixosModules.packages`, `nixosModules.services`, `darwinModules.homebrew`/`darwinModules.services`), a plain system rebuild (`sudo nixos-rebuild switch`, `darwin-rebuild switch`, or your home-manager equivalent) picks up any change to this repo automatically — these modules are consumed as a `path:` flake input, so a rebuild always reads the repo's current contents, with no separate `flake.lock` update needed for this input specifically.

If your system's own flake pins other inputs this repo depends on (nixpkgs, etc.) and those need bumping too, update them the normal way for your system's flake before rebuilding.

If you're not using the Nix modules, re-check [Prerequisites](INSTALL.md#prerequisites) for anything that changed: Docker, ffmpeg/espeak-ng, the transcription engine CLIs, Kokoro-FastAPI's own dependencies.

## Docker Services

```bash
docker compose pull
docker compose up -d --build
```

`--build` picks up changes to `nemo-normalizer`, which builds from source rather than a published image. If `docker-compose.yml` itself changed (a new image version, a new service), `git pull` already brought that in — the commands above just apply it.

## Re-Importing the n8n Workflows

Only needed when `d2v-workflow-main.json` or `d2v-workflow-bare-metal-services.json` changed (check `git log` for the files, or your diff against the previous version).

**Do not import the new file into the existing workflow's canvas.** n8n's Import merges into whatever's already open, which duplicates every node with matching names rather than replacing them. Instead, for each workflow that changed:

1. Open the existing workflow, and from its **⋯** menu, **Archive** it (reversible — this just hides it from the workflow list, it doesn't delete anything yet)
2. Click **Create workflow** to get a genuinely blank canvas, then use its own **⋯ → Import → From file** to import the updated JSON
3. Save it. Node-level credentials carry over automatically as long as the credential IDs referenced in the JSON still exist in this n8n instance (true for a same-instance update; re-attach manually if you're moving to a fresh instance)
4. Publish it

If publishing fails with a webhook path conflict even though the old workflow is archived and inactive: archiving and deactivating aren't enough to clear n8n's webhook-path registration for the old workflow. Go back to the archived workflow's **⋯** menu and **Delete** it (now a permanent action, unlike archiving) — that clears the stale registration, and the new workflow will publish.

Re-check [Securing the Bare-Metal Webhook](INSTALL.md#securing-the-bare-metal-webhook) if the new JSON added nodes that call across instances; newly added nodes need their Header Auth credential attached same as during initial install.
