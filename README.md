# Eggocado D2V: Document to Video

Eggocado D2V converts a PDF document into a narrated video. Each page is transcribed by a vision-language model, normalized for speech, synthesized to audio, and assembled into a video clip. Clips are concatenated into a final video with subtitles and chapters.

> **Platform support:** macOS (Apple Silicon) and Linux/NixOS. Windows support is planned.

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

| Component | Port | Where | Role |
|---|---|---|---|
| Docker n8n | `5678` | Docker | Main workflow: PDF to images, orchestrating everything |
| stirling-pdf | `8080` | Docker | Converts PDF pages to images |
| nemo-normalizer | `8765` | Docker | Normalizes transcribed text for speech |
| Bare-metal n8n | `5679` | Host process | Runs ffmpeg and shells out to the Claude Code/Codex CLIs, directly on the host so they can use the GPU and the CLIs' own host-level logins |
| Kokoro-FastAPI | `8881` | Host process | Text-to-speech, running natively: Apple Silicon GPU (MPS) acceleration on macOS, CPU inference on Linux |
| Ollama | `11434` | Host process | One of three transcription engines (optional, only if you use it), running natively so it can use the GPU |

Claude Code and Codex run as CLI calls from bare-metal n8n, not standalone services: each uses its own cached host-level login instead of listening on a port.

The Docker/host-process split exists because GPU-accelerated tools (ffmpeg hardware encoding, Ollama, Kokoro) need direct host GPU access, only available outside Docker Desktop's VM on macOS. As much as possible (PDF rendering, text normalization, orchestration) stays in the easy-to-set-up Docker stack. Only the pieces that need the GPU run on the host.

**The transcription engine is chosen per run**, from a dropdown at the start of each conversion: a local vision-capable Ollama model (any model you've pulled), Claude Code, or OpenAI Codex. The two CLI-based engines bill to your own existing subscription or API account, an alternative to running a large local model for transcription. Whichever engine you pick, its available models are fetched live.

### Security note

The bare-metal n8n instance executes shell commands (ffmpeg, Ollama calls) sent to it by the Docker instance over HTTP. Both sides are gated by a shared-secret header (see [INSTALL.md](INSTALL.md) for setup); without the secret, requests are rejected. That said, **this is a local single-user tool by design.** The security model is "block casual/opportunistic access," not "safe to expose to the internet": whoever holds the secret can run arbitrary shell commands on the host. If you choose to host the bare-metal service somewhere reachable beyond your own machine, that's possible, but the security implications are entirely your own responsibility.

## Output

Each document gets its own folder: `temp/{document}/` for intermediate files, `output/{document}/final_video.mp4` for the finished video. Sample Video mode's per-voice files live in `temp/sample/`.

## Glossary

Paths below are relative to the repo's main folder. Every per-page file lives in `temp/{document}/`, prefixed with its page number (or `{a}-{b}_knit_` for a knitted pair). The finished video's files live in `output/{document}/`.

| Term | File | What it is |
|---|---|---|
| Page image | `temp/{document}/{page}_image.png` | The page rendered from the PDF |
| Transcription | `temp/{document}/{page}_transcription.txt` | The page's text, read from its image by the transcription engine |
| Normalized text | `temp/{document}/{page}_transcription_normalized.txt` | The transcription rewritten for speech |
| Narration audio | `temp/{document}/{page}_audio.wav` | The page's narration, synthesized from the normalized text |
| Timestamps | `temp/{document}/{page}_timestamps.json` | Word-level timing for the narration audio, produced in the same call as the narration audio |
| Word map | `temp/{document}/{page}_wordmap.json` | The mapping between original and normalized words, used to build subtitles |
| Clip | `temp/{document}/{page}_clip.mp4` | The page's image and narration audio, assembled into one short video |
| Final video | `output/{document}/final_video.mp4` | Every clip in the document, concatenated with chapters |
| Final subtitles | `output/{document}/final_video.srt` | The final video's subtitle file |

## Reprocessing pages

The workflow skips a page's stages when their output files already exist in `temp/{document}/`. To regenerate something, delete its files below, then run Continue mode.

| What you want to recreate | How |
|---|---|
| The final video, its subtitles, or its chapters | Run Continue mode; finalization always rebuilds these from the current clips |
| A page's clip | Delete `temp/{document}/{page}_clip.mp4`, then run Continue mode |
| A page's narration audio | Delete `temp/{document}/{page}_audio.wav` and `temp/{document}/{page}_clip.mp4`, then run Continue mode |
| The normalized text a page's narration audio is read from | Delete `temp/{document}/{page}_transcription_normalized.txt`, `temp/{document}/{page}_audio.wav`, and `temp/{document}/{page}_clip.mp4`, then run Continue mode |
| A page's transcription | Delete `temp/{document}/{page}_transcription.txt`, `temp/{document}/{page}_transcription_normalized.txt`, `temp/{document}/{page}_audio.wav`, and `temp/{document}/{page}_clip.mp4`, then run Continue mode |
| Everything for one page | Delete all of that page's files in `temp/{document}/`, then run Continue mode |
| Everything for the whole document | Delete `temp/{document}/`, then run Continue mode |

> Deleting a page's narration audio also regenerates its timestamps and word map, since Kokoro produces all three from one call.

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
