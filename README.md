# D2V — Document to Video

D2V converts a PDF document into a narrated video. Each page is transcribed by a vision-language model, normalized for speech, synthesized to audio, and assembled into a video clip. All clips are then concatenated into a single output file.

## Architecture

Two n8n instances run in parallel:

| Instance | Port | Role |
|---|---|---|
| Docker n8n | ```5678``` | Main workflow: converting PDF pages into images, calling AI tools, TTS, calling ffmpeg |
| Bare-metal n8n | ```5679``` | Hardware accellerated tools: running ffmpeg commands for video processing, running Gemma through  Ollama |

The split exists for a couple of reasons:
- As much as possible is configured in the Docker n8n for easy setup
- ffmpeg and Gemma is running on a bare-metal instance of n8n for a couple of reasons:
  - Enabling GPU hardware accelleration for these services that might be too demanding for the CPU
  - Allowing you to run the proccesses that require hardware accelleration on one machine, and the ones that doesn't require hardware accelleration on another, giving you more flexibility in adapting this tool to your needs.

## Output

Completed clips are written to `temp/` and the final video to `output/final_video.mp4`, both relative to the project directory.

## Reprocessing pages

The workflow skips stages that already have output files in `temp/`. To reprocess a specific stage, delete the corresponding files and re-run the workflow. All files listed in a row must be deleted to trigger the stages shown — if any earlier-stage file still exists, that stage is skipped and all later stages are skipped with it.

| Files to delete | Stages that rerun |
|---|---|
| `temp/*_clip.mp4` | Clip assembly |
| `temp/*_timestamps.json` + `temp/*_clip.mp4` | TTS, clip assembly |
| `temp/*_transcription_normalized.txt` + `temp/*_timestamps.json` + `temp/*_clip.mp4` | Normalization, TTS, clip assembly |
| `temp/*_transcription.txt` + `temp/*_transcription_normalized.txt` + `temp/*_timestamps.json` + `temp/*_clip.mp4` | Transcription, normalization, TTS, clip assembly |
| All `temp/` files | Full pipeline from scratch |

> Audio files (`*_audio.wav`) do not need to be deleted separately. Kokoro's API returns audio and word-level timestamps in a single response, so deleting the timestamps file is sufficient to trigger a TTS re-run, which regenerates both.

## Setup

See [INSTALL.md](INSTALL.md) for all installation requirements.
