# ATRI Desktop Pet API Design

## Architecture

Rust backend embeds an HTTP server (axum) listening on `localhost:3210`. Requests are received by Rust, forwarded to the frontend via Tauri events, and the frontend executes Live2D operations.

```
External Caller (LLM skill / Python script)
    │  HTTP POST
    ▼
Rust axum server (localhost:3210)
    │  Tauri event emit
    ▼
Frontend (pixi-live2d-display)
    ├── Expression control
    ├── Motion playback
    ├── Lip sync (audio analysis → ParamMouthOpenY)
    └── Bubble overlay (text display)
```

## API Endpoints

### POST /expression
Switch expression.
```json
{"id": 1}        // by index
{"name": "害羞"}  // by name
```

### POST /motion
Play motion.
```json
{"group": "Idle", "index": 0}
```

### POST /speak
Core endpoint — bubble text + expression + audio playback + lip sync in one call.
```json
{
  "text": "こんにちは、ご主人様！",
  "audio_url": "file:///path/to/audio.wav",
  "expression": 1
}
```

### POST /bubble
Show text bubble only.
```json
{"text": "思考中...", "duration": 3000}
```

### POST /lipsync/start
Start lip sync from audio file.
```json
{"audio_url": "file:///path/to/audio.wav"}
```

### POST /lipsync/stop
Stop lip sync.

### GET /status
Returns current state (expression, speaking, etc).

### GET /expressions
Returns list of available expressions.
```json
[{"id": 1, "name": "害羞"}, {"id": 2, "name": "失去高光"}, ...]
```

## Expression Mapping

1. 害羞  2. 失去高光  3. 吊带睡衣  4. 内衣  5. 穿凉鞋
6. 穿皮鞋  7. 愣住  8. 白框  9. 染血  10. 小鸟
11. 螃蟹  12. NO  13. YES  14. 睡衣2  15. 阴影
16-19. (unnamed)

## Bubble UI

- Semi-transparent dialog bubble above ATRI's head
- Typewriter text effect
- Auto-dismiss after audio ends or configurable duration

## Lip Sync

- Frontend loads audio file, analyzes volume in real-time
- Drives `ParamMouthOpenY` parameter proportional to volume
- `ParamMouthForm` kept at default

## Config

- Default port: `3210`
- Configurable via environment variable or config file
