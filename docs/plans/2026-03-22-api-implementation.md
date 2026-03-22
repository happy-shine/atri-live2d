# ATRI HTTP API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add HTTP API (axum on localhost:3210) so external LLM skills can control ATRI's expressions, speech bubbles, motions, and lip-synced audio playback.

**Architecture:** Rust embeds an axum HTTP server in a background thread. Each endpoint deserializes the request, emits a Tauri event to the frontend. Frontend listens for events and drives pixi-live2d-display accordingly.

**Tech Stack:** axum, tokio, serde, Tauri events, pixi-live2d-display, Web Audio API

---

### Task 1: Add axum HTTP server skeleton in Rust

**Files:**
- Modify: `src-tauri/Cargo.toml` (add axum, tokio deps)
- Create: `src-tauri/src/api.rs` (HTTP server module)
- Modify: `src-tauri/src/lib.rs` (spawn server in setup)

**Step 1: Add dependencies to Cargo.toml**

Add after `serde_json = "1"`:
```toml
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors"] }
```

**Step 2: Create `src-tauri/src/api.rs`**

```rust
use axum::{
    Router,
    routing::{get, post},
    extract::State,
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct ApiState {
    pub app_handle: AppHandle,
}

#[derive(Deserialize)]
pub struct ExpressionReq {
    pub id: Option<u32>,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct MotionReq {
    pub group: String,
    pub index: Option<u32>,
}

#[derive(Deserialize)]
pub struct SpeakReq {
    pub text: String,
    pub audio_url: Option<String>,
    pub expression: Option<u32>,
}

#[derive(Deserialize)]
pub struct BubbleReq {
    pub text: String,
    pub duration: Option<u64>,
}

#[derive(Deserialize)]
pub struct LipsyncReq {
    pub audio_url: String,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub ok: bool,
    pub message: String,
}

fn ok_response(msg: &str) -> Json<ApiResponse> {
    Json(ApiResponse { ok: true, message: msg.to_string() })
}

pub async fn set_expression(
    State(state): State<ApiState>,
    Json(req): Json<ExpressionReq>,
) -> Json<ApiResponse> {
    let _ = state.app_handle.emit("api:expression", serde_json::to_value(&req).unwrap());
    ok_response("expression set")
}

pub async fn play_motion(
    State(state): State<ApiState>,
    Json(req): Json<MotionReq>,
) -> Json<ApiResponse> {
    let _ = state.app_handle.emit("api:motion", serde_json::to_value(&req).unwrap());
    ok_response("motion played")
}

pub async fn speak(
    State(state): State<ApiState>,
    Json(req): Json<SpeakReq>,
) -> Json<ApiResponse> {
    let _ = state.app_handle.emit("api:speak", serde_json::to_value(&req).unwrap());
    ok_response("speaking")
}

pub async fn show_bubble(
    State(state): State<ApiState>,
    Json(req): Json<BubbleReq>,
) -> Json<ApiResponse> {
    let _ = state.app_handle.emit("api:bubble", serde_json::to_value(&req).unwrap());
    ok_response("bubble shown")
}

pub async fn lipsync_start(
    State(state): State<ApiState>,
    Json(req): Json<LipsyncReq>,
) -> Json<ApiResponse> {
    let _ = state.app_handle.emit("api:lipsync:start", serde_json::to_value(&req).unwrap());
    ok_response("lipsync started")
}

pub async fn lipsync_stop(
    State(state): State<ApiState>,
) -> Json<ApiResponse> {
    let _ = state.app_handle.emit("api:lipsync:stop", ());
    ok_response("lipsync stopped")
}

pub async fn get_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "locked": true,
        "speaking": false
    }))
}

pub async fn get_expressions() -> Json<serde_json::Value> {
    Json(serde_json::json!([
        {"id": 1, "name": "害羞"},
        {"id": 2, "name": "失去高光"},
        {"id": 3, "name": "吊带睡衣"},
        {"id": 4, "name": "内衣"},
        {"id": 5, "name": "穿凉鞋"},
        {"id": 6, "name": "穿皮鞋"},
        {"id": 7, "name": "愣住"},
        {"id": 8, "name": "白框"},
        {"id": 9, "name": "染血"},
        {"id": 10, "name": "小鸟"},
        {"id": 11, "name": "螃蟹"},
        {"id": 12, "name": "NO"},
        {"id": 13, "name": "YES"},
        {"id": 14, "name": "睡衣2"},
        {"id": 15, "name": "阴影"},
        {"id": 16, "name": "exp_16"},
        {"id": 17, "name": "exp_17"},
        {"id": 18, "name": "exp_18"},
        {"id": 19, "name": "exp_19"}
    ]))
}

pub fn create_router(app_handle: AppHandle) -> Router {
    let state = ApiState { app_handle };

    Router::new()
        .route("/expression", post(set_expression))
        .route("/motion", post(play_motion))
        .route("/speak", post(speak))
        .route("/bubble", post(show_bubble))
        .route("/lipsync/start", post(lipsync_start))
        .route("/lipsync/stop", post(lipsync_stop))
        .route("/status", get(get_status))
        .route("/expressions", get(get_expressions))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn start_server(app_handle: AppHandle) {
    let router = create_router(app_handle);
    let port = std::env::var("ATRI_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3210u16);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .expect("Failed to bind API server");

    println!("ATRI API server listening on http://127.0.0.1:{}", port);

    axum::serve(listener, router).await.unwrap();
}
```

**Step 3: Update `src-tauri/src/lib.rs`**

Add `mod api;` at top. In `setup()`, after tray build, spawn the server:

```rust
let app_handle_clone = app.handle().clone();
std::thread::spawn(move || {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(api::start_server(app_handle_clone));
});
```

**Step 4: Build and verify server starts**

Run: `npm run tauri dev`
Expected: Console shows "ATRI API server listening on http://127.0.0.1:3210"

**Step 5: Test with curl**

Run: `curl http://localhost:3210/expressions`
Expected: JSON array of expressions

Run: `curl http://localhost:3210/status`
Expected: JSON with ok/locked/speaking fields

**Step 6: Commit**

```bash
git add src-tauri/src/api.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add axum HTTP API server skeleton with all endpoints"
```

---

### Task 2: Frontend expression and motion event handlers

**Files:**
- Modify: `src/main.ts` (add event listeners for api:expression, api:motion)

**Step 1: Add expression name mapping**

```typescript
const EXPRESSION_NAMES: Record<string, number> = {
  "害羞": 0, "失去高光": 1, "吊带睡衣": 2, "内衣": 3,
  "穿凉鞋": 4, "穿皮鞋": 5, "愣住": 6, "白框": 7,
  "染血": 8, "小鸟": 9, "螃蟹": 10, "NO": 11,
  "YES": 12, "睡衣2": 13, "阴影": 14,
};
```

**Step 2: Add event listeners after `loadModel()`**

```typescript
listen("api:expression", (event: any) => {
  if (!currentModel) return;
  const { id, name } = event.payload;
  if (id !== undefined) {
    currentModel.expression(id - 1); // API uses 1-based
  } else if (name && name in EXPRESSION_NAMES) {
    currentModel.expression(EXPRESSION_NAMES[name]);
  }
});

listen("api:motion", (event: any) => {
  if (!currentModel) return;
  const { group, index } = event.payload;
  currentModel.motion(group, index ?? 0);
});
```

**Step 3: Test with curl**

Run: `curl -X POST http://localhost:3210/expression -H 'Content-Type: application/json' -d '{"id": 7}'`
Expected: ATRI switches to "愣住" expression

Run: `curl -X POST http://localhost:3210/expression -H 'Content-Type: application/json' -d '{"name": "YES"}'`
Expected: ATRI switches to YES expression

**Step 4: Commit**

```bash
git add src/main.ts
git commit -m "feat: frontend expression and motion API event handlers"
```

---

### Task 3: Bubble text overlay

**Files:**
- Modify: `index.html` (add bubble container div)
- Modify: `src/styles.css` (bubble styles)
- Modify: `src/main.ts` (bubble show/hide logic + api:bubble event)

**Step 1: Add bubble HTML to `index.html`**

After `<div id="drag-overlay"></div>`:
```html
<div id="bubble" class="bubble hidden">
  <span id="bubble-text"></span>
</div>
```

**Step 2: Add bubble CSS to `src/styles.css`**

```css
.bubble {
  position: fixed;
  top: 8%;
  left: 50%;
  transform: translateX(-50%);
  max-width: 80%;
  padding: 10px 16px;
  background: rgba(255, 255, 255, 0.92);
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15);
  font-family: "Hiragino Sans", "PingFang SC", sans-serif;
  font-size: 14px;
  color: #333;
  z-index: 9000;
  pointer-events: none;
  transition: opacity 0.3s;
}

.bubble::after {
  content: "";
  position: absolute;
  bottom: -8px;
  left: 50%;
  transform: translateX(-50%);
  border-left: 8px solid transparent;
  border-right: 8px solid transparent;
  border-top: 8px solid rgba(255, 255, 255, 0.92);
}

.bubble.hidden {
  opacity: 0;
  pointer-events: none;
}
```

**Step 3: Add bubble logic to `src/main.ts`**

```typescript
const bubbleEl = document.getElementById("bubble")!;
const bubbleText = document.getElementById("bubble-text")!;
let bubbleTimer: number | null = null;
let typewriterTimer: number | null = null;

function showBubble(text: string, duration: number = 5000) {
  // Clear previous
  if (bubbleTimer) clearTimeout(bubbleTimer);
  if (typewriterTimer) clearInterval(typewriterTimer);

  bubbleText.textContent = "";
  bubbleEl.classList.remove("hidden");

  // Typewriter effect
  let i = 0;
  typewriterTimer = window.setInterval(() => {
    if (i < text.length) {
      bubbleText.textContent += text[i];
      i++;
    } else {
      if (typewriterTimer) clearInterval(typewriterTimer);
    }
  }, 50);

  // Auto dismiss
  bubbleTimer = window.setTimeout(() => {
    bubbleEl.classList.add("hidden");
  }, duration);
}

function hideBubble() {
  if (bubbleTimer) clearTimeout(bubbleTimer);
  if (typewriterTimer) clearInterval(typewriterTimer);
  bubbleEl.classList.add("hidden");
}

listen("api:bubble", (event: any) => {
  const { text, duration } = event.payload;
  showBubble(text, duration ?? 5000);
});
```

**Step 4: Test with curl**

Run: `curl -X POST http://localhost:3210/bubble -H 'Content-Type: application/json' -d '{"text": "ご主人様、こんにちは！", "duration": 4000}'`
Expected: Bubble appears above ATRI with typewriter effect, dismisses after 4s

**Step 5: Commit**

```bash
git add index.html src/styles.css src/main.ts
git commit -m "feat: add speech bubble overlay with typewriter effect"
```

---

### Task 4: Audio playback and lip sync

**Files:**
- Modify: `src/main.ts` (audio playback, volume analysis, mouth parameter control)

**Step 1: Add lip sync module**

```typescript
let audioContext: AudioContext | null = null;
let currentAudio: HTMLAudioElement | null = null;
let analyserNode: AnalyserNode | null = null;
let lipsyncActive = false;

function startLipsync(audioUrl: string, onEnd?: () => void) {
  stopLipsync();

  if (!audioContext) {
    audioContext = new AudioContext();
  }

  const audio = new Audio(audioUrl);
  currentAudio = audio;

  const source = audioContext.createMediaElementSource(audio);
  const analyser = audioContext.createAnalyser();
  analyser.fftSize = 256;
  source.connect(analyser);
  analyser.connect(audioContext.destination);
  analyserNode = analyser;
  lipsyncActive = true;

  const dataArray = new Uint8Array(analyser.frequencyBinCount);

  function updateMouth() {
    if (!lipsyncActive || !analyserNode) return;
    analyserNode.getByteFrequencyData(dataArray);

    // Average volume from low-mid frequencies (voice range)
    let sum = 0;
    for (let i = 0; i < 32; i++) sum += dataArray[i];
    const volume = sum / 32 / 255; // 0..1

    if (currentModel) {
      const coreModel = (currentModel as any).internalModel?.coreModel;
      if (coreModel) {
        coreModel.setParameterValueById("ParamMouthOpenY", volume * 1.2);
      }
    }

    requestAnimationFrame(updateMouth);
  }

  audio.addEventListener("ended", () => {
    stopLipsync();
    if (onEnd) onEnd();
  });

  audio.play();
  updateMouth();
}

function stopLipsync() {
  lipsyncActive = false;
  if (currentAudio) {
    currentAudio.pause();
    currentAudio = null;
  }
  analyserNode = null;
  // Reset mouth
  if (currentModel) {
    const coreModel = (currentModel as any).internalModel?.coreModel;
    if (coreModel) {
      coreModel.setParameterValueById("ParamMouthOpenY", 0);
    }
  }
}
```

**Step 2: Add lipsync event listeners**

```typescript
listen("api:lipsync:start", (event: any) => {
  const { audio_url } = event.payload;
  startLipsync(audio_url);
});

listen("api:lipsync:stop", () => {
  stopLipsync();
});
```

**Step 3: Test with a sample audio file**

Place a test wav/mp3 file somewhere accessible, then:
Run: `curl -X POST http://localhost:3210/lipsync/start -H 'Content-Type: application/json' -d '{"audio_url": "file:///path/to/test.wav"}'`
Expected: Audio plays, ATRI's mouth moves in sync with volume

Run: `curl -X POST http://localhost:3210/lipsync/stop`
Expected: Audio stops, mouth closes

**Step 4: Commit**

```bash
git add src/main.ts
git commit -m "feat: add audio playback with real-time lip sync"
```

---

### Task 5: Speak endpoint (combined bubble + expression + audio)

**Files:**
- Modify: `src/main.ts` (api:speak handler combining all features)

**Step 1: Add speak event handler**

```typescript
listen("api:speak", (event: any) => {
  if (!currentModel) return;
  const { text, audio_url, expression } = event.payload;

  // Set expression if provided
  if (expression !== undefined) {
    currentModel.expression(expression - 1);
  }

  if (audio_url) {
    // Show bubble, play audio with lip sync, hide bubble when done
    showBubble(text, 999999); // keep until audio ends
    startLipsync(audio_url, () => {
      hideBubble();
    });
  } else {
    // No audio: just show bubble with default duration
    showBubble(text, Math.max(text.length * 150, 3000));
  }
});
```

**Step 2: Test the full speak flow**

Run:
```bash
curl -X POST http://localhost:3210/speak \
  -H 'Content-Type: application/json' \
  -d '{"text": "ご主人様、おはようございます！", "expression": 13}'
```
Expected: ATRI switches to YES expression, bubble shows with typewriter text

With audio:
```bash
curl -X POST http://localhost:3210/speak \
  -H 'Content-Type: application/json' \
  -d '{"text": "テスト", "audio_url": "file:///path/to/audio.wav", "expression": 1}'
```
Expected: Expression changes, bubble shows, audio plays with lip sync, bubble hides when audio ends

**Step 3: Commit**

```bash
git add src/main.ts
git commit -m "feat: add /speak endpoint combining bubble + expression + lip sync"
```

---

### Task 6: Final integration test and cleanup

**Step 1: Test all endpoints end-to-end**

```bash
# List expressions
curl http://localhost:3210/expressions | python3 -m json.tool

# Status
curl http://localhost:3210/status | python3 -m json.tool

# Expression by name
curl -X POST http://localhost:3210/expression -H 'Content-Type: application/json' -d '{"name": "害羞"}'

# Bubble
curl -X POST http://localhost:3210/bubble -H 'Content-Type: application/json' -d '{"text": "Hello World!"}'

# Speak without audio
curl -X POST http://localhost:3210/speak -H 'Content-Type: application/json' -d '{"text": "高性能ですから！", "expression": 13}'
```

**Step 2: Commit all remaining changes**

```bash
git add -A
git commit -m "feat: complete ATRI HTTP API for external LLM integration"
```
