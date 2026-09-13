# Installation

## Prerequisites

### Docker
Required to run the containerized services.

```bash
# macOS
brew install --cask docker
```

Start Docker Desktop before proceeding.

### ffmpeg
Required by OS n8n to assemble video clips.

```bash
brew install ffmpeg
```

### Ollama
Required to run the Gemma vision model locally.

Download from https://ollama.com, then pull the model:

```bash
ollama pull gemma4:31b
```

Ollama must be running and listening on `http://127.0.0.1:11434` when the workflow executes.

### n8n (bare-metal)
A second n8n instance runs directly on the host at port 5679, separate from the Docker one. Install globally:

```bash
npm install -g n8n
```

Start it from the project directory so that relative paths (`temp/`, `output/`) resolve correctly:

```bash
cd /path/to/D2V
N8N_PORT=5679 n8n start
```

## Docker services

All remaining services are defined in `docker-compose.yml` and start with:

```bash
docker compose up -d
```

This starts:

| Service | Port | Purpose |
|---|---|---|
| n8n | 5678 | Main orchestration workflow |
| stirling-pdf | 8080 | Converts PDF pages to PNG images |
| kokoro | 8880 | Text-to-speech (Kokoro FastAPI) |
| nemo-normalizer | 8765 | Text normalization for TTS (NeMo, built locally) |

The nemo-normalizer image is built from `./nemo-normalizer/` on first run. This takes several minutes as it installs NeMo text processing dependencies inside the container.

## Importing the workflows

1. Open Docker n8n at `http://localhost:5678`
2. Import `d2v-workflow-main.json`
3. Open OS n8n at `http://localhost:5679`
4. Import `d2v-workflow-bare-metal-services.json`
5. Activate both workflows

## Verification

Check that all services are reachable before running:

```bash
curl http://localhost:11434/api/tags          # Ollama
curl http://localhost:8080                    # Stirling PDF
curl http://localhost:8880/v1/models          # Kokoro
curl http://localhost:8765/health             # NeMo normalizer
curl http://localhost:5679/webhook/clip       # OS n8n clip endpoint
curl http://localhost:5679/webhook/prompt     # OS n8n prompt endpoint
```
