# Installation

macOS (Apple Silicon) and Linux/NixOS. Windows support is planned.

## Prerequisites

On NixOS, `nixos/packages.nix` covers this entire section in one import: Docker, ffmpeg, espeak-ng, the bare-metal n8n package, and also `nix-ld`/`uv`/`cmake`/`gcc`/`gnumake`, which Kokoro-FastAPI (below) needs to build and run on NixOS. It takes the system username as a curried argument:

```nix
imports = [ (import /path/to/Eggocado-D2V/nixos/packages.nix { username = "yourusername"; }) ];
```

The Docker, ffmpeg/espeak-ng, and n8n subsections below are the macOS (Homebrew) equivalent of what that import covers. Ollama, Claude Code, Codex, and Kokoro-FastAPI's own setup are the same steps on both platforms.

### Docker
Required to run the containerized services. See ["Get Docker"](https://docs.docker.com/get-started/get-docker/) for the current setup steps for your platform. Start Docker before proceeding.

### ffmpeg and espeak-ng
Required by the bare-metal n8n instance (video assembly) and Kokoro-FastAPI (fallback pronunciation for unknown words). See ["Download FFmpeg"](https://ffmpeg.org/download.html) and [espeak-ng](https://github.com/espeak-ng/espeak-ng) for the current install steps.

### Ollama
Optional: one of three transcription engines (see "Transcription engines" below). Skip this section entirely if you only plan to use Claude Code or Codex. If you do want a local model, any vision-capable Ollama model works, so pick whichever fits your hardware and documents.

Download from https://ollama.com, then pull a model, for example:

```bash
ollama pull gemma3:27b
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

This service is required to use D2V. `mac/services.nix` has a launchd example if you use nix-darwin, or on NixOS, `nixos/services.nix`'s `n8n-d2v-baremetal` systemd user service, wired into the system configuration and confirmed working end to end on a full document conversion.

### Kokoro-FastAPI (native, for GPU acceleration where available)
Text-to-speech runs as a separate, native service, so it can use Apple Silicon's GPU (MPS) where available. This is a separate project you clone alongside this one; see [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI) for the current setup steps:

```bash
cd /path/to/GitHub  # wherever you keep repos, as a sibling to Eggocado-D2V
git clone https://github.com/remsky/Kokoro-FastAPI.git
cd Kokoro-FastAPI
python docker/scripts/download_model.py --output api/src/models/v1_0
./start-gpu_mac.sh    # macOS (Apple Silicon, MPS)
./start-cpu.sh        # Linux, CPU inference
```

This starts Kokoro on port 8881. Like bare-metal n8n, this service is required to use D2V: `mac/services.nix` has a launchd example if you use nix-darwin (optional), or on NixOS, `nixos/services.nix`'s `kokoro-fastapi` systemd user service, CPU-only (`USE_GPU=false`), confirmed working via a live TTS request returning audio and word-level timestamps.

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
