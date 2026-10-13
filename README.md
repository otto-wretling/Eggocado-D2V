# Eggocado D2V: Document to Video

Eggocado D2V converts a PDF document into a narrated video. Each page is transcribed by a vision-language model, normalized for speech, synthesized to audio, and assembled into a video clip. Clips are concatenated into a final video with subtitles and chapters.

> **Platform support:** macOS (Apple Silicon) only, for now. Linux/Windows support is on the roadmap but not implemented yet.

> **Language support:** English only, for now. Kokoro and some other TTS backends support additional languages, but the transcription prompt, text normalization, and the TTS call are all currently hardcoded for English.

## Modes

Everything is driven through one n8n form:

| Mode | What it does |
|---|---|
| 🆕 Convert entire document | Upload a new PDF and process every page start to finish |
| 🔁 Continue converting entire document | Resume an in-progress document, re-finalize the video, or pick up after an interruption |
| 🧶 Knit Pages Together | Merge two adjacent pages (e.g. a two-page spread) into a single narrated unit |
| 🎙️ Convert single page | Re-narrate one page (or knitted pair) in a different voice: the whole page, or just a selected phrase |
| 🔊 Create Sample Video | Generate a short demo clip for every available TTS voice, useful for picking one |

Any mode can be combined with any other: knit two pages, then change voice on that pair, then resume and re-finalize, and it all stays consistent.

## Architecture

Three things run side by side:

| Component | Port | Role |
|---|---|---|
| Docker n8n | `5678` | Main workflow: PDF to images, calling the transcription/normalization/TTS services, orchestrating everything |
| Bare-metal n8n | `5679` | Runs ffmpeg and Ollama directly on the host, outside Docker, so they can use the GPU |
| Native Kokoro-FastAPI | `8881` | Text-to-speech, running natively (not in Docker) for Apple Silicon GPU (MPS) acceleration |

The split exists because GPU-accelerated tools (ffmpeg hardware encoding, Ollama, Kokoro) don't get useful GPU access inside Docker Desktop on macOS. As much as possible (PDF rendering, text normalization, orchestration) stays in the easy-to-set-up Docker instance. Only the pieces that need the GPU run natively.

**The transcription model is swappable.** Transcription runs through Ollama, so you can use any vision-capable model Ollama supports, not just the one in the default config. This is one of the more useful things to customize for your own documents and hardware. The model name lives in the `Populate Variables` node in the main workflow, not hardcoded into any logic.

### Security note

The bare-metal n8n instance executes shell commands (ffmpeg, Ollama calls) sent to it by the Docker instance over HTTP. Both sides are gated by a shared-secret header (see [INSTALL.md](INSTALL.md) for setup); without the secret, requests are rejected. That said, **this is a local single-user tool by design.** The security model is "block casual/opportunistic access," not "safe to expose to the internet": whoever holds the secret can run arbitrary shell commands on the host. If you choose to host the bare-metal service somewhere reachable beyond your own machine, that's possible, but the security implications are entirely your own responsibility.

## Output

Each document gets its own folder: `temp/{document}/` for intermediate files, `output/{document}/final_video.mp4` for the finished video. Sample Video mode's per-voice files live in `temp/sample/`.

## Reprocessing pages

The workflow skips stages that already have output files in `temp/{document}/`. To reprocess a specific stage, delete the corresponding files and re-run Continue mode. All files listed in a row must be deleted to trigger the stages shown. If any earlier-stage file still exists, that stage is skipped and all later stages are skipped with it.

| Files to delete | Stages that rerun |
|---|---|
| `{page}_clip.mp4` | Clip assembly |
| `{page}_timestamps.json` + `{page}_clip.mp4` | TTS, clip assembly |
| `{page}_transcription_normalized.txt` + `{page}_timestamps.json` + `{page}_clip.mp4` | Normalization, TTS, clip assembly |
| `{page}_transcription.txt` + `{page}_transcription_normalized.txt` + `{page}_timestamps.json` + `{page}_clip.mp4` | Transcription, normalization, TTS, clip assembly |
| All files for that document | Full pipeline from scratch |

> Audio files (`{page}_audio.wav`) don't need to be deleted separately. Kokoro's API returns audio and word-level timestamps in a single response, so deleting the timestamps file is sufficient to trigger a TTS re-run, which regenerates both.

## Setup

See [INSTALL.md](INSTALL.md) for all installation requirements, including the one-time webhook security setup.

## Credits

Eggocado D2V orchestrates these tools, run locally. None of their code is bundled, so none of these licenses constrain this project's own, but each deserves the credit:

- [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI) and the Kokoro TTS model (Apache 2.0)
- [NVIDIA NeMo text processing](https://github.com/NVIDIA/NeMo-text-processing) for normalization (Apache 2.0)
- [Ollama](https://ollama.com) (MIT) running whichever vision model you configure. The default config uses Google's [Gemma](https://ai.google.dev/gemma/terms), but any Ollama-compatible vision model works
- [Stirling PDF](https://github.com/Stirling-Tools/Stirling-PDF) for PDF-to-image conversion (MIT for the self-hosted core this project uses; some other parts of that project are separately licensed)
- [ffmpeg](https://ffmpeg.org) for video assembly
- [n8n](https://n8n.io) for workflow orchestration (n8n itself runs under its own [Sustainable Use License](https://github.com/n8n-io/n8n/blob/master/LICENSE.md), not included in this project's license)

## License

Apache 2.0. See [LICENSE](LICENSE).
