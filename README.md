# Eggocado D2V: Document to Video

Eggocado D2V converts a PDF document into a narrated video. Each page is transcribed by a vision-language model, normalized for speech, synthesized to audio, and assembled into a video clip. Clips are concatenated into a final video with subtitles and chapters.

> **Platform support:** Windows, Linux(NixOS), and macOS (Apple Silicon) are officially supported.

> **Language support:** English is the only officially supported language. Additional languages may work without issues using Kokoro or other TTS backends. However, until more languages are officially supported by this project, it will be on you to verify whether these languages work as intended.

## Contents

- [Installation](#installation)
- [Updating](#updating)
- [Usage](#usage)
  - [Converting a New Document](#converting-a-new-document)
  - [Continuing a Conversion](#continuing-a-conversion)
  - [Knitting Pages Together](#knitting-pages-together)
  - [Convert Single Page](#convert-single-page)
  - [Creating a Sample Video](#creating-a-sample-video)
  - [File Glossary](#file-glossary)
  - [Headless API](#headless-api)
- [Testing](#testing)
- [Architecture](#architecture)
  - [Security Note](#security-note)
- [Credits](#credits)
- [License](#license)

## Installation

See [INSTALL.md](INSTALL.md) for how to get started.

## Updating

See [UPDATE.md](UPDATE.md) for how to update an installation to the latest release.

## Usage

Eggocado D2V can be operated using its built-in n8n form that guides the user to select mode, AI models, a PDF file, and other required data. To start a run: open the `d2v-workflow-main` workflow in the Docker n8n editor (`http://localhost:5678`) and click **📋 Execute** to open the form.

### Converting a New Document

Use this mode to upload a new PDF and process every page from start to finish.

Each document gets its own folder: `temp/{document}/` for intermediate files, and `output/{document}/` for the finished video. Converting a document with the same file name more than once overwrites output files in those folders.

If you would like to stop the conversion and continue later (e.g. a conversion overnight that hasn't finished by the time you want to use the computer for other things), you can simply use the [continue mode](#continuing-a-conversion) to pick up the conversion from the same point where you left it.

### Continuing a Conversion

Use this mode to resume an in-progress document, make small changes to a video, or pick up after an interruption. This mode requires that you have started a [new conversion](#converting-a-new-document) or [converted a single page](#convert-single-page) of a PDF.

This mode looks at what already exists in the `temp/{document}/` folder and skips anything that has already been created, thus allowing you to pick up from where you left off, or redoing one or more parts of a page (or multiple). To regenerate a specific part of a page's video, delete the appropriate files according to the table below and run this mode:

| What you want to recreate | How |
|---|---|
| Everything for the whole document | Run [Converting a New Document](#converting-a-new-document) with the same document name and confirm in the form that the document will be overwritten |
| Everything for one page | Delete all of that page's files in `temp/{document}/`, then run this mode |
| A page's transcription (e.g., the transcription engine misread a word in the page image) | Delete the following files and run this mode:<ul><li>`temp/{document}/{page}_audio.wav`</li><li>`temp/{document}/{page}_clip.mp4`</li><li>`temp/{document}/{page}_transcription.txt`</li><li>`temp/{document}/{page}_transcription_normalized.txt`</li></ul> |
| The normalized text a page's narration audio is read from (e.g., Kokoro mispronounces a correctly spelled word) | Delete the following files and run this mode:<ul><li>`temp/{document}/{page}_audio.wav`</li><li>`temp/{document}/{page}_clip.mp4`</li><li>`temp/{document}/{page}_transcription_normalized.txt`</li></ul> |
| A page's narration audio (not necessary for changing the voice of a page or part of a page; use [Convert Single Page](#convert-single-page) for that). | Delete the following files and run this mode:<ul><li>`temp/{document}/{page}_audio.wav`</li><li>`temp/{document}/{page}_clip.mp4`</li></ul> Deleting a page's narration audio also regenerates its timestamps and word map.|
| A page's clip | Delete `temp/{document}/{page}_clip.mp4`, then run this mode |
| The final video, its subtitles, or its chapters | Run this mode; it always rebuilds these files on each run |

### Knitting Pages Together

Merge two adjacent pages (e.g., a two-page spread) into a single narrated unit. The two pages are combined into one double-width image, and their transcriptions are merged into one, reordered to read naturally across the full spread (image and chart descriptions move to the end, noting where they appear). The merged text is narrated once, producing a single clip that replaces the two individual page clips. Requires both pages to already be converted first, via New conversion or Continue — knitting a document with no processed pages yet, or a pair where one page hasn't been processed, fails.

Every page's clip, knitted or not, renders on the same double-width canvas: a blurred, stretched copy of the page fills the width a knit would otherwise need, behind the sharp page centered on top. That keeps the whole video one consistent width throughout, so a knitted pair's naturally double-width clip drops in without resizing anything else.

### Convert Single Page

Re-narrate one page (or knitted pair) in a different voice: the whole page, or just a selected phrase. Requires the target page to already be converted; re-narrating a selected phrase additionally requires that exact phrase to appear in the page's transcript.

### Creating a Sample Video

Generate a short demo clip for every available TTS voice, useful for picking one. Doesn't depend on any document, just the phrase each voice reads. Per-voice files live in `temp/sample/`.

---

Any mode can be combined with any other: knit two pages, then change voice on that pair, then resume and re-finalize, and it all stays consistent.

### File Glossary

Every file type the modes above create, in one place. Paths below are relative to the repo's main folder. Every per-page file lives in `temp/{document}/`, prefixed with its page number (or `{a}-{b}_knit_` for a knitted pair). The finished video's files live in `output/{document}/`.

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

### Headless API

Every mode is also reachable over HTTP, for scripting or automation, without the n8n form:

| Endpoint | Method | What it does |
|---|---|---|
| `/webhook/d2v-headless` | POST | Validates the request, then either starts a job and returns `{jobId, mode, executionId}` once the job has written its `output/<jobId>/.started` marker, returns `400` with `{error}` if a required parameter is missing or invalid, or returns `500` if the job fails before writing that marker. The job keeps running in the background |
| `/webhook/d2v-headless-status?jobId=...` | GET | Polls a job, returning `{jobId, status}`, where `status` is `not_found`, `processing`, `done`, or `failed` (with a `message` field) |

Both endpoints require the same `x-webhook-secret` header value as the one set up in [INSTALL.md](INSTALL.md#securing-the-bare-metal-webhook) (the main n8n instance's own Header Auth credential, reused here). A start request is `multipart/form-data`. The returned `jobId` is the document name, slugified, except Sample Video mode, which uses `sample-<n8n execution id>`.

All required parameters are checked before the job starts: if one is missing or invalid, the POST call itself returns `400` with `{error: "<message>"}` and no job is created. A few checks can only be evaluated after the job starts (e.g., `document` naming something that was never converted, or `overwrite` being needed); those still surface asynchronously as a `failed` status.

<details>
<summary>Parameters and example requests for every mode</summary>

Each example sends `D2V_BASE_URL=http://localhost:5678` and `SECRET` set to the shared webhook secret. `engineModel` takes the form `<engine>:<model>`, matching whichever engine and model you have available (see [Transcription Engines](INSTALL.md#transcription-engines)).

| Parameter | Used by | Required | If missing/invalid | What it does |
|---|---|---|---|---|
| `mode` | every request | Yes | `400` | Which operation to run: `new`, `continue`, `knit`, `changeVoice`, or `sample` |
| `document` | `new`, `continue`, `knit`, `changeVoice` | `new`: No. Others: Yes | `new`: falls back to the literal job ID `document`. Others: `400` | The document's name; also determines the job ID |
| `data` | `new` | Yes | `400` | The PDF file to convert. Other modes use the document's already-saved PDF |
| `engineModel` | `new`, `continue`, `knit` | Yes | `400` | Transcription engine and model, as `<engine>:<model>`. `changeVoice` only re-synthesizes narration, so it has no use for this field |
| `voice` | `new`, `continue`, `knit`, `changeVoice` | Yes | `400` | Kokoro voice ID used for narration |
| `overwrite` | `new` | No | Treated as refused; job starts but fails asynchronously if `document` already has a conversion | Confirms replacing an existing document's conversion |
| `leftPage`, `rightPage` | `knit` | Yes | `400` | The two page numbers to knit together |
| `pageSelection` | `changeVoice` | Yes | `400` | Which page to target, as `Page N` |
| `target` | `changeVoice` | Yes | `400` | `wholePage` or `selectedText` |
| `textToChange` | `changeVoice` | Yes, when `target` is `selectedText` | `400` | The exact phrase to re-narrate |
| `samplePhrase` | `sample` | Yes | `400` | The phrase every voice sample speaks |
| `acceleration` | every request | No | Missing: `platformDefault`. Invalid: `400` | Which hardware processes the conversion: `platformDefault` (the `eggocado-d2v.acceleration.platformDefault` option, `automatic` unless set), `automatic`, or `cpu` |

Convert entire document:

```bash
curl -X POST "$D2V_BASE_URL/webhook/d2v-headless" \
  -H "x-webhook-secret: $SECRET" \
  -F "mode=new" \
  -F "document=my-document.pdf" \
  -F "engineModel=codex:gpt-5.6-sol" \
  -F "voice=af_heart" \
  -F "overwrite=true" \
  -F "data=@/path/to/my-document.pdf;type=application/pdf"
```

Returns `{"jobId":"my-document","mode":"new","executionId":"1234"}` as soon as the job has started. `overwrite=true` is only needed when `document` names a document that was already converted; omitting `document` entirely falls back to the literal job ID `document`.

Continue converting entire document:

```bash
curl -X POST "$D2V_BASE_URL/webhook/d2v-headless" \
  -H "x-webhook-secret: $SECRET" \
  -F "mode=continue" \
  -F "document=my-document.pdf" \
  -F "engineModel=codex:gpt-5.6-sol" \
  -F "voice=af_heart"
```

Knit pages together:

```bash
curl -X POST "$D2V_BASE_URL/webhook/d2v-headless" \
  -H "x-webhook-secret: $SECRET" \
  -F "mode=knit" \
  -F "document=my-document.pdf" \
  -F "engineModel=codex:gpt-5.6-sol" \
  -F "leftPage=1" \
  -F "rightPage=2" \
  -F "voice=af_heart"
```

Convert single page, whole page (`changeVoice` only re-synthesizes narration, so it takes no `engineModel`):

```bash
curl -X POST "$D2V_BASE_URL/webhook/d2v-headless" \
  -H "x-webhook-secret: $SECRET" \
  -F "mode=changeVoice" \
  -F "document=my-document.pdf" \
  -F "pageSelection=Page 1" \
  -F "target=wholePage" \
  -F "voice=af_bella"
```

Convert single page, selected phrase:

```bash
curl -X POST "$D2V_BASE_URL/webhook/d2v-headless" \
  -H "x-webhook-secret: $SECRET" \
  -F "mode=changeVoice" \
  -F "document=my-document.pdf" \
  -F "pageSelection=Page 2" \
  -F "target=selectedText" \
  -F "textToChange=Product" \
  -F "voice=af_bella"
```

Create sample video:

```bash
curl -X POST "$D2V_BASE_URL/webhook/d2v-headless" \
  -H "x-webhook-secret: $SECRET" \
  -F "mode=sample" \
  -F "samplePhrase=Hello, this is a sample of my voice."
```

Returns `{"jobId":"sample-1234","mode":"sample","executionId":"1234"}`, where `1234` is the n8n execution ID — each sample request gets its own job, so concurrent calls never collide.

A request with a missing or invalid required parameter, e.g., `knit` without `leftPage`/`rightPage`, returns `400` instead of starting a job:

```bash
curl -X POST "$D2V_BASE_URL/webhook/d2v-headless" \
  -H "x-webhook-secret: $SECRET" \
  -F "mode=knit" \
  -F "document=my-document.pdf" \
  -F "engineModel=codex:gpt-5.6-sol" \
  -F "voice=af_heart"
# 400 {"error":"leftPage and rightPage are required for knit mode"}
```

Checking status for any of the above:

```bash
curl "$D2V_BASE_URL/webhook/d2v-headless-status?jobId=my-document" \
  -H "x-webhook-secret: $SECRET"
```

`status` is one of:

| `status` | Meaning |
|---|---|
| `not_found` | No job with this `jobId` has ever started (or its output was deleted). Once a start request returns its `jobId`, that job reports one of the other statuses |
| `processing` | The job has started and is still running |
| `done` | The job finished; `output/<jobId>/final_video.mp4` and `.srt` are ready |
| `failed` | The job started but could not finish; `message` has the reason |

```json
{"jobId": "my-document", "status": "not_found"}
{"jobId": "my-document", "status": "processing"}
{"jobId": "my-document", "status": "done"}
{"jobId": "my-document", "status": "failed", "message": "This document has no processed pages yet, so this mode cannot run."}
```

A job that was running when n8n stopped reports `failed` once n8n starts again: the main workflow's `n8n Instance Started` trigger checks every started job through the status endpoint and records each one still `processing` as failed. While n8n is down, every status request fails to connect, and a test waiting on a job fails after a minute of that.

</details>

## Testing

`d2v-headless-api-tests/` is a Rust integration test suite that exercises the headless API above end-to-end against a running instance: every mode's happy path, Continue mode, and every one of the API's documented failure cases, both the synchronous `400` ones and the asynchronous `failed` ones. There's no mocking: each test calls the real endpoints, polling the live status endpoint for the cases that run as a background job.

Set the four required environment variables, then run `cargo test` from that directory:

```bash
cd d2v-headless-api-tests
D2V_BASE_URL=http://localhost:5678 \
D2V_WEBHOOK_SECRET=<your webhook secret> \
D2V_PROJECT_ROOT=/path/to/Eggocado-D2V \
D2V_ENGINE_MODEL=<engine>:<model> \
cargo test
```

`D2V_ENGINE_MODEL` is whichever transcription engine and model you have available (see [Transcription Engines](INSTALL.md#transcription-engines)), for example `codex:gpt-5.6-sol`, `claude:claude-sonnet-5`, or `ollama:gemma4:31b`. The suite never hardcodes one, so it runs the same way regardless of which engine you use.

Before its first test, each test file runs `ffprobe -version` and `ffmpeg -version`, which the tests use to inspect every finished video, and checks the credentials through the headless API itself: a status request with `D2V_WEBHOOK_SECRET` has to answer `200`, which passes the main workflow's status webhook, its `Check Job State` call and the bare-metal webhook that call reaches, and an empty start request has to answer the API's own `400`. A missing program or a refused credential on those paths fails every test in that file within a second. A job that reaches any other node with a missing credential reports `failed` at that node, and the test fails as soon as it sees that status. A test waiting on a job fails once the status endpoint stays unreachable for a minute.

Tests run one at a time, enforced via that directory's own `.cargo/config.toml`, since they share the same `temp/`/`output/` folders and the single-worker Kokoro TTS service a real conversion would use. The same file builds the test binaries one at a time, so linking fits alongside the running services on a machine with 8 GB of memory, such as a default WSL distribution on a 16 GB PC. A full run takes a while: several tests wait for a multi-minute video conversion to complete.

## Architecture

| Component | Port | Where | Role |
|---|---|---|---|
| Docker n8n | `5678` | Docker | Main workflow: PDF to images, orchestrating everything |
| stirling-pdf | `8080` | Docker | Converts PDF pages to PNG images |
| nemo-normalizer | `8765` | Docker | Normalizes transcribed text for speech |
| Bare-metal n8n | `5679` | Host process | Runs ffmpeg and shells out to the Claude Code/Codex CLIs, directly on the host so they can use the GPU and the CLIs' own host-level logins |
| Kokoro-FastAPI | `8881` | Host process | Text-to-speech, running natively: Apple Silicon GPU (MPS) acceleration on macOS, CPU inference on Linux |
| Kokoro-FastAPI (CPU) | `8882` | Host process | Text-to-speech on the CPU, started for conversions that run on the CPU and stopped after the final video |
| Ollama | `11434` | Host process | One of three transcription engines (optional, only if you use it), running natively so it can use the GPU |

Claude Code and Codex run as CLI calls from bare-metal n8n, authenticated via each CLI's own cached host-level login.

The Docker/host-process split exists because GPU-accelerated tools (ffmpeg hardware encoding, Ollama, Kokoro) need direct host GPU access, only available outside Docker. As much as possible (PDF rendering, text normalization, orchestration) stays in the easy-to-set-up Docker stack. Only the pieces that need the GPU run on the host.

**The transcription engine is chosen per run**, from a dropdown at the start of each conversion: a local vision-capable Ollama model (any model you've pulled), Claude Code, or OpenAI Codex.

**The hardware is chosen per run as well**: Use platform default, Automatic, or CPU. Automatic uses the GPU wherever a stage supports it: Ollama, Kokoro-FastAPI, and VA-API, VideoToolbox, or NVENC video encoding. CPU runs Ollama on the CPU, starts the CPU Kokoro-FastAPI instance, and encodes with libx264. When an accelerated stage fails, the conversion retries that stage on the CPU and keeps every later stage on the CPU.

### Security Note

The bare-metal n8n instance executes shell commands (ffmpeg, Ollama calls) sent to it by the Docker instance over HTTP. Both sides are gated by a shared-secret header (see [INSTALL.md](INSTALL.md) for setup): every request must include the matching secret to be accepted. That said, **this is a local single-user tool by design.** The security model blocks casual or opportunistic access: whoever holds the secret can run arbitrary shell commands on the host. If you choose to host the bare-metal service somewhere reachable beyond your own machine, that's possible, but the security implications are entirely your own responsibility.

## Credits

Eggocado D2V orchestrates these tools, run locally. None of their code is bundled, so none of these licenses constrain this project's own, but each deserves the credit:

- [Kokoro-FastAPI](https://github.com/remsky/Kokoro-FastAPI) and the Kokoro TTS model (Apache 2.0)
- [NVIDIA NeMo text processing](https://github.com/NVIDIA/NeMo-text-processing) for normalization (Apache 2.0)
- [Ollama](https://ollama.com) (MIT) running whichever vision model you configure. The default config uses Google's [Gemma](https://ai.google.dev/gemma/terms), but any Ollama-compatible vision model works
- [Stirling PDF](https://github.com/Stirling-Tools/Stirling-PDF) for PDF-to-image conversion (MIT for the self-hosted core this project uses; some other parts of that project are separately licensed)
- [ffmpeg](https://ffmpeg.org) for video assembly
- [n8n](https://n8n.io) for workflow orchestration, under its own [Sustainable Use License](https://github.com/n8n-io/n8n/blob/master/LICENSE.md)

## License

Apache 2.0. See [LICENSE](LICENSE).
