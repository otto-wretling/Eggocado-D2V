# Installation

macOS (Apple Silicon), Linux/NixOS, and Windows through NixOS-WSL. On Windows, start with [Windows](#windows).

## Windows

On Windows, D2V runs in a [NixOS-WSL](https://github.com/nix-community/NixOS-WSL) distribution. Its NixOS configuration provides bare-metal n8n, Kokoro-FastAPI, ffmpeg, and espeak-ng from [Prerequisites](#prerequisites), and step 7 adds the transcription engine CLIs. Docker Desktop on Windows runs the [Docker services](#docker-services). Ollama runs natively on Windows with the full system memory and GPU, and the distribution reaches it at `http://127.0.0.1:11434`. Kokoro-FastAPI uses an NVIDIA GPU through the Windows driver when one is present.

1. Download [`.config/configuration.winget`](.config/configuration.winget), review it, and apply it in PowerShell from the folder you saved it to, accepting the UAC prompts:

   ```powershell
   winget configure -f configuration.winget --accept-configuration-agreements
   ```

   It installs WSL, Docker Desktop, and Ollama, and sets `networkingMode=mirrored` and `hostAddressLoopback=true` in `%USERPROFILE%\.wslconfig` so the containers and the services in the distribution reach each other. Restart Windows if the WSL installation asks for it. If WSL was already running, run `wsl --shutdown` to apply the new `.wslconfig`.

2. Install NixOS-WSL following its [Installation](https://nix-community.github.io/NixOS-WSL/install.html) guide.

3. Inside the distribution, replace `/etc/nixos` with this repo's NixOS-WSL template and build it:

   ```bash
   sudo mv /etc/nixos /etc/nixos.original
   sudo mkdir /etc/nixos
   cd /etc/nixos
   sudo nix --extra-experimental-features 'nix-command flakes' flake init -t github:otto-wretling/Eggocado-D2V#wsl
   sudo nixos-rebuild switch --flake /etc/nixos#eggocado-d2v
   ```

   The template's `configuration.nix` sets `wsl.defaultUser = "nixos"`, NixOS-WSL's default user. If your distribution uses a different user name, set it there before running `nixos-rebuild`. On a machine with a CPU only, also set `eggocado-d2v.kokoro.torchVariant = "cpu";` there for a smaller Kokoro-FastAPI environment.

4. In Docker Desktop, open **Settings > Resources > WSL Integration**, turn on the NixOS-WSL distribution, and select **Apply**. See [Docker Desktop WSL 2 backend on Windows](https://docs.docker.com/desktop/features/wsl/) for details. Once integration is on, `/usr/bin/docker` exists inside the distribution. If it is still missing a minute later, follow [Docker Desktop: "WSL integration with distro unexpectedly stopped"](#docker-desktop-wsl-integration-with-distro-unexpectedly-stopped).

5. Inside the distribution, clone this repo to `~/Documents/GitHub/Eggocado-D2V`, where the services run from:

   ```bash
   git clone https://github.com/otto-wretling/Eggocado-D2V.git ~/Documents/GitHub/Eggocado-D2V
   ```

   The `d2v-docker-services` user service then starts the [Docker services](#docker-services) from that checkout as soon as Docker Desktop's integration answers, and recreates them each time the distribution starts so the `n8n` container mounts the checkout after the distribution's files are available. The first start builds the nemo-normalizer image, which takes several minutes.

6. On Windows itself, clone this repo again, this time to the same relative path under your Windows user profile: `C:\Users\<you>\Documents\GitHub\Eggocado-D2V` (GitHub Desktop or a plain `git clone` both work). This is a separate, ordinary Windows-side checkout for browsing and editing the code; the distribution's own checkout from step 5 is what the services actually run against. Its `temp` and `output` folders start out empty: the next time the form or the headless API runs, it checks whether `temp.lnk`/`output.lnk` shortcuts already exist there and, if not, creates them pointing at the distribution's real `temp`/`output` folders, so double-clicking either one opens the real, live folder, with files appearing there natively as soon as they're produced. No admin rights or Developer Mode needed: shortcuts are plain files, not privileged reparse points. Deleting a specific file, or a whole `temp/{document}/` subfolder, from inside one of these opened folders genuinely deletes it on the distribution side too, matching [Continuing a Conversion](README.md#continuing-a-conversion)'s recreate instructions. Deleting the shortcut itself just removes the shortcut; the next run recreates it automatically.

7. Install the CLIs for the engines you plan to use from [Transcription engines](#transcription-engines) inside the distribution, and pull an Ollama model in PowerShell as shown in [Ollama](#ollama).

8. Continue with [Importing the workflows](#importing-the-workflows).

Run the commands in the rest of this guide inside the distribution. From PowerShell, address the services as `127.0.0.1`, for example `curl.exe http://127.0.0.1:5679/healthz`.

To update D2V's NixOS configuration later, run inside the distribution:

```bash
cd /etc/nixos
sudo nix flake update eggocado-d2v
sudo nixos-rebuild switch --flake /etc/nixos#eggocado-d2v
```

## Prerequisites

This section covers macOS and NixOS. On Windows, [Windows](#windows) covers it.

On NixOS, this repo's `nixosModules.packages` flake output covers this entire section in one import: Docker, ffmpeg, espeak-ng, the bare-metal n8n package, Node.js and npm (for the transcription engine CLIs, with global installs on your `PATH`), `cargo`/`rustc` (for the Rust test suite, see [Testing](README.md#testing)), and the `nix-ld`/`uv`/`cmake`/`gcc`/`gnumake` build tools. The module pulls these packages from its own pinned nixpkgs (tracked in this repo's `flake.lock`), independent of your system's nixpkgs version, so a Node/Java security fix here doesn't wait on a full system nixpkgs bump. Add it as a flake input and import the module, passing your system username as a curried argument:

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

On NixOS, `nixosModules.packages` (see [Prerequisites](#prerequisites) above) already runs Ollama as a service, listening on `http://127.0.0.1:11434`. On Windows, `configuration.winget` installs it (see [Windows](#windows) above). Elsewhere, download from https://ollama.com and run it yourself. Either way, pull a model, for example:

```bash
ollama pull gemma4:31b
```

Whichever models you've pulled are fetched live and offered in the engine/model dropdown at the start of each conversion.

If you install Ollama, it must be running and listening on `http://127.0.0.1:11434` when the workflow executes.

### Transcription Engines
Page transcription can use any of three engines, selected at the start of each conversion (the first form field in the main workflow). Ollama (above) always works out of the box; the other two need their CLI installed and logged in once before they'll work:

- **Local Ollama model**: covered by the Ollama section above.
- **Claude Code**: see Claude Code's ["Quickstart"](https://code.claude.com/docs/en/quickstart) for the current install steps, then run `claude` once and follow the interactive login prompt. Needs a Claude subscription or API access.
- **OpenAI Codex**: see ["Codex CLI"](https://learn.chatgpt.com/docs/codex/cli) for the current install steps, then run `codex login` once. Needs a ChatGPT Plus/Pro/Team/Enterprise plan or API access.

Both CLIs cache their login after the first run, so this is a one-time step per machine. Available models for each engine are fetched live (same pattern as the Kokoro voice picker), so whatever your account has access to shows up in the model dropdown.

### n8n (Bare-Metal)
A second n8n instance runs directly on the host at port 5679, separate from the Docker one. This is what actually runs ffmpeg, calls Ollama's API, and shells out to the Claude Code/Codex CLIs, so it needs host access. See ["Host n8n"](https://docs.n8n.io/hosting/installation/) for the current method.

Start it from the project directory so that relative paths (`temp/`, `output/`) resolve correctly:

```bash
cd /path/to/Eggocado-D2V
N8N_PORT=5679 n8n start
```

This service is required to use D2V. `nix/mac.nix` has a launchd example if you use nix-darwin, or on NixOS, `nix/nixos.nix`'s `n8n-d2v-baremetal` systemd user service, wired into the system configuration and confirmed working end to end on a full document conversion.

### Kokoro-FastAPI (Native, for GPU Acceleration Where Available)
Text-to-speech runs as a separate, native service on port 8881, so it can use the GPU where available, plus a CPU-only instance on port 8882 that the bare-metal n8n starts on demand through its `D2V_START_CPU_TTS_COMMAND` environment variable. Like bare-metal n8n, this service is required to use D2V. Kokoro selects MPS, CUDA, or CPU in that order of availability.

On NixOS, `nix/nixos.nix`'s `kokoro-fastapi` and `kokoro-fastapi-cpu` systemd user services run [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI) entirely from this flake: its source is the pinned `kokoro-fastapi` flake input, Nix builds its Python environment from Kokoro-FastAPI's own `uv.lock`, and Nix downloads the model weights with fixed hashes. The environment uses CPU PyTorch by default. For an NVIDIA GPU, set the CUDA 12.6 build in your configuration:

```nix
eggocado-d2v.kokoro.torchVariant = "gpu";
```

On macOS, clone Kokoro-FastAPI as a sibling of this repo and set it up following [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI)'s Apple Silicon (MPS) instructions. `nix/mac.nix` has a launchd example if you use nix-darwin (optional), which runs the checkout at `eggocado-d2v.kokoroRoot` (by default `~/Documents/GitHub/Kokoro-FastAPI`).

## Docker Services

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

## Importing the Workflows

1. Open Docker n8n at `http://localhost:5678`, complete the one-time owner account setup if prompted
2. Import `d2v-workflow-main.json` and save it
3. Open bare-metal n8n at `http://localhost:5679`, complete its own one-time owner account setup
4. Import `d2v-workflow-bare-metal-services.json` and save it
5. Continue with [Securing the bare-metal webhook](#securing-the-bare-metal-webhook), which ends with publishing both workflows. n8n publishes a workflow once every node has its credential.

## Securing the Bare-Metal Webhook

The bare-metal instance runs arbitrary shell commands sent to it by the main workflow, so both sides need a shared secret before anything will work. Without it, every call from the main workflow to the bare-metal one is rejected with a 403.

1. In the **bare-metal n8n** (`localhost:5679`), go to **Credentials → Create credential → Header Auth**. Set:
   - **Name** (the header name): `X-Webhook-Secret`
   - **Value**: any long random string (generate one with `openssl rand -hex 24`)
   - Save it (call it e.g. "D2V Webhook Shared Secret")
2. Open each webhook trigger node in the bare-metal workflow (`Get Request for Assembling Clip`, `Receive Prompt`, `Receive TTS Request`, `Receive Claude Prompt`, `Receive Codex Prompt`, `Receive Ollama Models Request`, `Receive Codex Models Request`) and set **Authentication** to **Header Auth**, selecting the credential you just created.
3. In the **main n8n** (`localhost:5678`), create a **matching** Header Auth credential with the **same header name and value** as step 1.
4. Attach that credential to every node that calls the bare-metal instance: `Request Image Transcription`, `Request Clip Assembly`, `Request Final Video Assembly`, `Request TTS with Timestamps`, `Knit Images`, `Knit Transcriptions`, `Ensure Base Folders Exist`, `Ensure Document Folder Exists`, `Ensure Output Folder Exists`, `Ensure Sample Folder Exists`, `Ensure Job Output Folder Exists`, `Fetch Ollama Models`, `Fetch Codex Models`, `Detect Video Encoder Support`, `Read Platform Default Acceleration`, `Start CPU Text-to-Speech When Selected`, `Start CPU Text-to-Speech for Voice List`, `Stop CPU Text-to-Speech When Selected`, `Unload Ollama Model After CPU Conversion`, `Check Job State`, `Record Failure and Stop CPU Text-to-Speech`, `List Started Jobs`. Also attach it to `Check Started Job Status`, which calls the main workflow's own status endpoint. For each, set the node's **Authentication** to **Generic Credential Type → Header Auth** and pick the credential. (`Fetch Claude Models` calls `endoflife.date`'s public API directly.) Also pick it in the main workflow's two webhook triggers, `Receive Headless API Request` and `Receive Status Check Request`, which check the same header on headless API requests. The imported nodes already have their authentication set, and n8n selects the credential as you open each node when it is the only Header Auth credential.
5. Save and publish both workflows.
6. Open the main workflow's **Settings** and set **Error Workflow** to `d2v-workflow-main` itself. When a conversion fails, its `Conversion Failed` trigger marks the job as `failed` for the status endpoint and stops the CPU Kokoro-FastAPI instance. When n8n starts, its `n8n Instance Started` trigger does the same for every job that was still running when n8n stopped.

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

### Windows: Empty Project Folder in the Docker n8n

When Docker Desktop restarts while the NixOS-WSL distribution keeps running, the `n8n` container can come back with an empty `/home/node/project`, and conversions fail to find their files. Recreate the Docker services inside the distribution:

```bash
systemctl --user restart d2v-docker-services
```
