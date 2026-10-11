# Installation

macOS (Apple Silicon) only, for now.

## Prerequisites

### Docker
Required to run the containerized services.

```bash
brew install --cask docker
```

Start Docker Desktop before proceeding.

### ffmpeg and espeak-ng
Required by the bare-metal n8n instance (video assembly) and Kokoro-FastAPI (fallback pronunciation for unknown words).

```bash
brew install ffmpeg espeak-ng
```

### Ollama
Runs the vision-language model used for page transcription. Any vision-capable Ollama model works — pick whichever fits your hardware and documents; the default config uses Google's Gemma.

Download from https://ollama.com, then pull a model, for example:

```bash
ollama pull gemma3:27b
```

If you use a different model, update the model name in the `Populate Variables` node after importing the main workflow (see below).

Ollama must be running and listening on `http://127.0.0.1:11434` when the workflow executes.

### n8n (bare-metal)
A second n8n instance runs directly on the host at port 5679, separate from the Docker one — this is what actually runs ffmpeg and talks to Ollama, so it needs host access rather than Docker's.

```bash
npm install -g n8n
```

Start it from the project directory so that relative paths (`temp/`, `output/`) resolve correctly:

```bash
cd /path/to/Eggocado-D2V
N8N_PORT=5679 n8n start
```

Keep this running in the background (a terminal tab, `screen`/`tmux`, or a launchd service — see `mac/services.nix` for an example if you use nix-darwin) while you use D2V.

### Kokoro-FastAPI (native, for GPU acceleration)
Text-to-speech runs as a separate service, native rather than Dockerized, so it can use Apple Silicon's GPU (MPS). This is a separate project you clone alongside this one:

```bash
cd /path/to/GitHub  # wherever you keep repos, as a sibling to Eggocado-D2V
git clone https://github.com/remsky/Kokoro-FastAPI.git
cd Kokoro-FastAPI
python docker/scripts/download_model.py --output api/src/models/v1_0
./start-gpu_mac.sh
```

This starts Kokoro on port 8881. Like bare-metal n8n, keep it running in the background (again, `mac/services.nix` has a launchd example if you use nix-darwin — it's optional, not required).

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

The bare-metal instance runs arbitrary shell commands sent to it by the main workflow, so both sides need a shared secret before anything will work — without it, every call from the main workflow to the bare-metal one is rejected with a 403.

1. In the **bare-metal n8n** (`localhost:5679`), go to **Credentials → Create credential → Header Auth**. Set:
   - **Name** (the header name): `X-Webhook-Secret`
   - **Value**: any long random string — generate one with `openssl rand -hex 24`
   - Save it (call it e.g. "D2V Webhook Shared Secret")
2. Open each of the three webhook trigger nodes in the bare-metal workflow (`Get Request for Assembling Clip`, `Receive Prompt`, `Receive TTS Request`) and set **Authentication** to **Header Auth**, selecting the credential you just created.
3. In the **main n8n** (`localhost:5678`), create a **matching** Header Auth credential with the **same header name and value** as step 1.
4. Attach that credential to every node that calls the bare-metal instance: `Request Image Transcription`, `Request Clip Assembly`, `Request Final Video Assembly`, `Request TTS with Timestamps`, `Knit Images`, `Knit Transcriptions`, `Ensure Document Folder Exists`, `Ensure Output Folder Exists`, `Ensure Sample Folder Exists`. For each, set the node's **Authentication** to **Generic Credential Type → Header Auth** and pick the credential.
5. Save both workflows.

Verify it's working:

```bash
# Without the secret — should return 403
curl -X GET http://localhost:5679/webhook/clip -d '{"command":"echo test"}'

# With the secret — should return 200
curl -X GET http://localhost:5679/webhook/clip \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: <your secret value>" \
  -d '{"command":"echo test"}'
```

## Verification

Check that all services are reachable before running:

```bash
curl http://localhost:11434/api/tags          # Ollama
curl http://localhost:8080                    # Stirling PDF
curl http://localhost:8881/v1/audio/voices    # Kokoro-FastAPI
curl http://localhost:8765/health             # NeMo normalizer
curl http://localhost:5679/healthz            # Bare-metal n8n
```
