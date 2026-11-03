# Installation

macOS (Apple Silicon) and Linux/NixOS. Windows support is planned.

## Prerequisites

On NixOS, this repo's `nixosModules.packages` flake output covers this entire section in one import: Docker, ffmpeg, espeak-ng, the bare-metal n8n package, `cargo`/`rustc` (for the Rust test suite, see [Testing](README.md#testing)), and the `nix-ld`/`uv`/`cmake`/`gcc`/`gnumake` build tools. The module pulls these packages from its own pinned nixpkgs (tracked in this repo's `flake.lock`), independent of your system's nixpkgs version, so a Node/Java security fix here doesn't wait on a full system nixpkgs bump. Add it as a flake input and import the module, passing your system username as a curried argument:

```nix
# flake.nix
inputs.eggocado-d2v.url = "path:/path/to/Eggocado-D2V";

# configuration.nix
imports = [ (eggocado-d2v.nixosModules.packages { username = "yourusername"; }) ];
```

The Docker, ffmpeg/espeak-ng, and n8n subsections below are the macOS (Homebrew) equivalent of what that import covers. Ollama, Claude Code, and Codex are the same steps on both platforms. On NixOS, Kokoro-FastAPI comes from this flake as well (see [Kokoro-FastAPI](#kokoro-fastapi-native-for-gpu-acceleration-where-available) below).

### Docker
Required to run the containerized services. See ["Get Docker"](https://docs.docker.com/get-started/get-docker/) for the current setup steps for your platform. Start Docker before proceeding.

### ffmpeg and espeak-ng
Required by the bare-metal n8n instance (video assembly) and Kokoro-FastAPI (fallback pronunciation for unknown words). See ["Download FFmpeg"](https://ffmpeg.org/download.html) and [espeak-ng](https://github.com/espeak-ng/espeak-ng) for the current install steps.

### Ollama
Optional: one of three transcription engines (see "Transcription engines" below). Skip this section entirely if you only plan to use Claude Code or Codex. If you do want a local model, any vision-capable Ollama model works, so pick whichever fits your hardware and documents.

On NixOS, `nixosModules.packages` (see [Prerequisites](#prerequisites) above) already runs Ollama as a service, listening on `http://127.0.0.1:11434`. Elsewhere, download from https://ollama.com and run it yourself. Either way, pull a model, for example:

```bash
ollama pull gemma4:31b
```

Whichever models you've pulled are fetched live and offered in the engine/model dropdown at the start of each conversion.

If you install Ollama, it must be running and listening on `http://127.0.0.1:11434` when the workflow executes.

### Transcription engines
Page transcription can use any of three engines, selected at the start of each conversion (the first form field in the main workflow). Ollama (above) always works out of the box; the other two need their CLI installed and logged in once before they'll work:

- **Local Ollama model**: covered by the Ollama section above.
- **Claude Code**: see Claude Code's ["Quickstart"](https://code.claude.com/docs/en/quickstart) for the current install steps, then run `claude` once and follow the interactive login prompt. Needs a Claude subscription or API access.
- **OpenAI Codex**: see ["Codex CLI"](https://learn.chatgpt.com/docs/codex/cli) for the current install steps, then run `codex login` once. Needs a ChatGPT Plus/Pro/Team/Enterprise plan or API access.

Both CLIs cache their login after the first run, so this is a one-time step per machine. Available models for each engine are fetched live (same pattern as the Kokoro voice picker), so whatever your account has access to shows up in the model dropdown.

### n8n (bare-metal)
A second n8n instance runs directly on the host at port 5679, separate from the Docker one. This is what actually runs ffmpeg, calls Ollama's API, and shells out to the Claude Code/Codex CLIs, so it needs host access. See ["Host n8n"](https://docs.n8n.io/hosting/installation/) for the current method.

Start it from the project directory so that relative paths (`temp/`, `output/`) resolve correctly:

```bash
cd /path/to/Eggocado-D2V
N8N_PORT=5679 n8n start
```

This service is required to use D2V. `nix/mac.nix` has a launchd example if you use nix-darwin, or on NixOS, `nix/nixos.nix`'s `n8n-d2v-baremetal` systemd user service, wired into the system configuration and confirmed working end to end on a full document conversion.

### Kokoro-FastAPI (native, for GPU acceleration where available)
Text-to-speech runs as a separate, native service on port 8881, so it can use the GPU where available, plus a CPU-only instance on port 8882 that the bare-metal n8n starts on demand through its `D2V_START_CPU_TTS_COMMAND` environment variable. Like bare-metal n8n, this service is required to use D2V. Kokoro selects MPS, CUDA, or CPU in that order of availability.

On NixOS, `nix/nixos.nix`'s `kokoro-fastapi` and `kokoro-fastapi-cpu` systemd user services run [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI) entirely from this flake: its source is the pinned `kokoro-fastapi` flake input, Nix builds its Python environment from Kokoro-FastAPI's own `uv.lock`, and Nix downloads the model weights with fixed hashes. The environment uses CPU PyTorch by default. For an NVIDIA GPU, set the CUDA 12.6 build in your configuration:

```nix
eggocado-d2v.kokoro.torchVariant = "gpu";
```

On macOS, clone Kokoro-FastAPI as a sibling of this repo and set it up following [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI)'s Apple Silicon (MPS) instructions. `nix/mac.nix` has a launchd example if you use nix-darwin (optional), which runs the checkout at `eggocado-d2v.kokoroRoot` (by default `~/Documents/GitHub/Kokoro-FastAPI`).

## Docker services

The remaining services are defined in `docker-compose.yml` and start with:

```bash
docker compose up -d
```

This starts:

| Service | Port | Purpose |
|---|---|---|
| n8n | 5678 | Main orchestration workflow |
| stirling-pdf | 8080 | Converts PDF pages to PNG images |
| nemo-normalizer | 8765 | Text normalization for TTS (NeMo, built locally) |

The nemo-normalizer image is built from `./nemo-normalizer/` on first run. This takes several minutes as it installs NeMo text processing dependencies inside the container.

## Importing the workflows

1. Open Docker n8n at `http://localhost:5678`, complete the one-time owner account setup if prompted
2. Import `d2v-workflow-main.json`
3. Open bare-metal n8n at `http://localhost:5679`, complete its own one-time owner account setup
4. Import `d2v-workflow-bare-metal-services.json`
5. Activate both workflows

## Securing the bare-metal webhook

The bare-metal instance runs arbitrary shell commands sent to it by the main workflow, so both sides need a shared secret before anything will work. Without it, every call from the main workflow to the bare-metal one is rejected with a 403.

1. In the **bare-metal n8n** (`localhost:5679`), go to **Credentials → Create credential → Header Auth**. Set:
   - **Name** (the header name): `X-Webhook-Secret`
   - **Value**: any long random string (generate one with `openssl rand -hex 24`)
   - Save it (call it e.g. "D2V Webhook Shared Secret")
2. Open each webhook trigger node in the bare-metal workflow (`Get Request for Assembling Clip`, `Receive Prompt`, `Receive TTS Request`, `Receive Claude Prompt`, `Receive Codex Prompt`, `Receive Ollama Models Request`, `Receive Codex Models Request`) and set **Authentication** to **Header Auth**, selecting the credential you just created.
3. In the **main n8n** (`localhost:5678`), create a **matching** Header Auth credential with the **same header name and value** as step 1.
4. Attach that credential to every node that calls the bare-metal instance: `Request Image Transcription`, `Request Clip Assembly`, `Request Final Video Assembly`, `Request TTS with Timestamps`, `Knit Images`, `Knit Transcriptions`, `Ensure Base Folders Exist`, `Ensure Document Folder Exists`, `Ensure Output Folder Exists`, `Ensure Sample Folder Exists`, `Fetch Ollama Models`, `Fetch Codex Models`. For each, set the node's **Authentication** to **Generic Credential Type → Header Auth** and pick the credential. (`Fetch Claude Models` calls `endoflife.date`'s public API directly.)
5. Save both workflows.

Verify it's working:

```bash
# Without the secret, should return 403
curl -X GET http://localhost:5679/webhook/clip -d '{"command":"echo test"}'

# With the secret, should return 200
curl -X GET http://localhost:5679/webhook/clip \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: <your secret value>" \
  -d '{"command":"echo test"}'
```

## Verification

Check that all services are reachable before running:

```bash
curl http://localhost:11434/api/tags          # Ollama (only if you installed it)
curl http://localhost:8080                    # Stirling PDF
curl http://localhost:8881/v1/audio/voices    # Kokoro-FastAPI
curl http://localhost:8765/health             # NeMo normalizer
curl http://localhost:5679/healthz            # Bare-metal n8n
```

## Troubleshooting

### Docker Desktop: "WSL integration with distro unexpectedly stopped"

On Windows, Docker Desktop can report that WSL integration with your NixOS-WSL distribution unexpectedly stopped, typically after Docker Desktop restarts or updates while WSL keeps running. The `docker` command is then missing inside the distribution, and Docker Desktop's log repeats `waiting 1m0s for the backend socket`. This is a known Docker Desktop issue, [docker/desktop-feedback#582](https://github.com/docker/desktop-feedback/issues/582): a Docker Desktop backend that starts while the WSL VM is still running skips publishing the sockets its WSL integration connects to.

To recover:

1. Quit Docker Desktop.
2. Run `wsl --shutdown` in PowerShell.
3. Start Docker Desktop.
