# ATRI RVC 歌声转换 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a CLI tool that converts any song's vocals to ATRI's voice using RVC, with one-click vocal separation and remixing.

**Architecture:** Standalone Python project using demucs for vocal separation, RVC core for voice conversion, and ffmpeg for final mixing. Training uses existing 2222 ATRI voice samples from GPT-SoVITS dataset.

**Tech Stack:** Python 3.10+, uv, torch (MPS), demucs, RVC core (extracted), faiss-cpu, ffmpeg

---

### Task 1: Project Scaffolding

**Files:**
- Create: `~/PycharmProjects/atri-rvc/pyproject.toml`
- Create: `~/PycharmProjects/atri-rvc/.gitignore`
- Create: `~/PycharmProjects/atri-rvc/README.md`

**Step 1: Init project with uv**

```bash
cd ~/PycharmProjects
mkdir atri-rvc && cd atri-rvc
git init
uv init --no-readme
```

**Step 2: Write pyproject.toml**

```toml
[project]
name = "atri-rvc"
version = "0.1.0"
description = "ATRI voice singing conversion using RVC"
requires-python = ">=3.10"
dependencies = [
    "torch>=2.0",
    "torchaudio>=2.0",
    "numpy",
    "scipy",
    "librosa",
    "soundfile",
    "demucs",
    "faiss-cpu",
    "praat-parselmouth",
    "pyworld",
    "torchcrepe",
]

[project.scripts]
atri-sing = "scripts.atri_sing:main"
```

**Step 3: Write .gitignore**

```
.venv/
__pycache__/
*.pyc
models/*.pth
models/*.index
pretrained/
dataset/
output/
*.egg-info/
```

**Step 4: Write README.md**

```markdown
# ATRI RVC

ATRI 歌声转换工具。输入歌曲，输出 ATRI 声音版本。

## Setup

\`\`\`bash
uv sync
\`\`\`

## Usage

\`\`\`bash
uv run python scripts/atri_sing.py input.mp3 -o output.mp3
\`\`\`
```

**Step 5: Create directory structure and install deps**

```bash
mkdir -p scripts models pretrained output dataset
uv sync
```

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: init atri-rvc project with uv"
```

---

### Task 2: Download Pretrained Models

**Files:**
- Create: `~/PycharmProjects/atri-rvc/scripts/download_models.py`

**Step 1: Write download script**

Downloads `hubert_base.pt` and `rmvpe.pt` to `pretrained/`. These are required for both training and inference.

```python
"""Download pretrained models for RVC."""
import urllib.request
import os
from pathlib import Path

MODELS = {
    "hubert_base.pt": "https://huggingface.co/lj1995/VoiceConversionWebUI/resolve/main/hubert_base.pt",
    "rmvpe.pt": "https://huggingface.co/lj1995/VoiceConversionWebUI/resolve/main/rmvpe.pt",
}

def download(url: str, dest: Path):
    if dest.exists():
        print(f"  Already exists: {dest}")
        return
    print(f"  Downloading {dest.name} ...")
    urllib.request.urlretrieve(url, dest)
    print(f"  Done: {dest} ({dest.stat().st_size / 1e6:.1f} MB)")

def main():
    pretrained = Path(__file__).parent.parent / "pretrained"
    pretrained.mkdir(exist_ok=True)
    for name, url in MODELS.items():
        download(url, pretrained / name)

if __name__ == "__main__":
    main()
```

**Step 2: Run it**

```bash
uv run python scripts/download_models.py
```

Expected: `pretrained/hubert_base.pt` (~190MB) and `pretrained/rmvpe.pt` (~140MB) downloaded.

**Step 3: Commit**

```bash
git add scripts/download_models.py
git commit -m "feat: add pretrained model download script"
```

---

### Task 3: Data Preparation Script

**Files:**
- Create: `~/PycharmProjects/atri-rvc/scripts/prepare_dataset.py`

**Step 1: Write prepare_dataset.py**

Converts ATRI opus files to wav, filters short/bad samples.

```python
"""Convert ATRI opus files to wav for RVC training."""
import argparse
import subprocess
from pathlib import Path

def convert_opus_to_wav(src: Path, dst: Path, min_duration: float = 1.0):
    """Convert opus to 16bit mono 44100Hz wav, skip if too short."""
    # Probe duration
    result = subprocess.run(
        ["ffprobe", "-v", "quiet", "-show_entries", "format=duration",
         "-of", "csv=p=0", str(src)],
        capture_output=True, text=True
    )
    try:
        duration = float(result.stdout.strip())
    except ValueError:
        return False

    if duration < min_duration:
        return False

    subprocess.run(
        ["ffmpeg", "-y", "-i", str(src), "-ar", "44100", "-ac", "1",
         "-sample_fmt", "s16", str(dst)],
        capture_output=True
    )
    return dst.exists()

def main():
    parser = argparse.ArgumentParser(description="Prepare ATRI dataset for RVC training")
    parser.add_argument("--src", type=Path,
                        default=Path.home() / "GPT-sovits" / "resource" / "vol1")
    parser.add_argument("--dst", type=Path,
                        default=Path(__file__).parent.parent / "dataset" / "atri")
    parser.add_argument("--min-duration", type=float, default=1.0)
    args = parser.parse_args()

    args.dst.mkdir(parents=True, exist_ok=True)
    opus_files = sorted(args.src.glob("ATR_*.opus"))
    print(f"Found {len(opus_files)} ATRI opus files")

    converted = 0
    skipped = 0
    for f in opus_files:
        wav_path = args.dst / f"{f.stem}.wav"
        if convert_opus_to_wav(f, wav_path, args.min_duration):
            converted += 1
        else:
            skipped += 1

    print(f"Done: {converted} converted, {skipped} skipped (< {args.min_duration}s)")

if __name__ == "__main__":
    main()
```

**Step 2: Run it**

```bash
uv run python scripts/prepare_dataset.py
```

Expected: `dataset/atri/` populated with wav files, short samples skipped.

**Step 3: Verify output**

```bash
ls dataset/atri/ | wc -l
ffprobe -v quiet -show_entries format=duration -of csv=p=0 dataset/atri/ATR_b101_013.wav
```

Expected: ~2000+ wav files, duration matches original.

**Step 4: Commit**

```bash
git add scripts/prepare_dataset.py
git commit -m "feat: add dataset preparation script (opus→wav)"
```

---

### Task 4: Extract RVC Core Code

**Files:**
- Create: `~/PycharmProjects/atri-rvc/rvc_core/__init__.py`
- Create: `~/PycharmProjects/atri-rvc/rvc_core/rmvpe.py` — f0 extraction with RMVPE
- Create: `~/PycharmProjects/atri-rvc/rvc_core/hubert.py` — HUBERT feature extraction
- Create: `~/PycharmProjects/atri-rvc/rvc_core/models.py` — RVC SynthesizerTrn model definition
- Create: `~/PycharmProjects/atri-rvc/rvc_core/pipeline.py` — voice conversion inference pipeline
- Create: `~/PycharmProjects/atri-rvc/rvc_core/train.py` — training pipeline

This is the most complex task. Extract the minimal RVC inference and training code from the RVC-Project repository. Key components needed:

1. **RMVPE** — F0 pitch extraction network (for both training and inference)
2. **HUBERT** — Feature extraction from fairseq (for both training and inference)
3. **SynthesizerTrn** — The core RVC voice conversion model
4. **VC Pipeline** — Orchestrates feature extraction + model inference + resampling
5. **Training loop** — Preprocess → extract features → train model

**Approach:** Clone RVC-Project temporarily, study its structure, extract and adapt the core files. Remove WebUI/Gradio dependencies. Ensure MPS compatibility.

**Step 1: Clone RVC repo for reference**

```bash
cd /tmp && git clone --depth 1 https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI.git rvc-ref
```

**Step 2: Study and extract core modules**

Extract from the RVC repo:
- `infer/lib/rmvpe.py` → `rvc_core/rmvpe.py`
- `infer/modules/vc/pipeline.py` → `rvc_core/pipeline.py`
- `infer/lib/infer_pack/models.py` → `rvc_core/models.py`
- Training scripts from `infer/modules/train/`

Adapt: remove Gradio imports, hardcode paths to `pretrained/`, ensure torch MPS device support.

**Step 3: Verify imports work**

```bash
uv run python -c "from rvc_core import pipeline; print('OK')"
```

**Step 4: Commit**

```bash
git add rvc_core/
git commit -m "feat: extract RVC core inference and training code"
```

---

### Task 5: Training Script

**Files:**
- Create: `~/PycharmProjects/atri-rvc/scripts/train.py`

**Step 1: Write train.py**

Orchestrates the full RVC training pipeline:

1. Preprocess: slice audio, resample to target SR (40k)
2. Extract HUBERT features from each audio file
3. Extract f0 (pitch) using RMVPE
4. Train the voice conversion model
5. Build FAISS index for retrieval
6. Save model + index to `models/`

```python
"""Train RVC model on ATRI dataset."""
import argparse
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description="Train ATRI RVC model")
    parser.add_argument("--dataset", type=Path,
                        default=Path(__file__).parent.parent / "dataset" / "atri")
    parser.add_argument("--output", type=Path,
                        default=Path(__file__).parent.parent / "models")
    parser.add_argument("--name", default="atri")
    parser.add_argument("--sr", type=int, default=40000, choices=[32000, 40000, 48000])
    parser.add_argument("--epochs", type=int, default=200)
    parser.add_argument("--batch-size", type=int, default=4)
    parser.add_argument("--f0-method", default="rmvpe")
    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)

    # Import here after arg parsing for faster --help
    from rvc_core.train import RVCTrainer

    trainer = RVCTrainer(
        dataset_dir=args.dataset,
        output_dir=args.output,
        model_name=args.name,
        sample_rate=args.sr,
        f0_method=args.f0_method,
        epochs=args.epochs,
        batch_size=args.batch_size,
    )
    trainer.preprocess()
    trainer.extract_features()
    trainer.train()
    trainer.build_index()
    print(f"\nDone! Model: {args.output / args.name}.pth")
    print(f"       Index: {args.output / args.name}.index")

if __name__ == "__main__":
    main()
```

**Step 2: Run training**

```bash
uv run python scripts/train.py --epochs 200 --batch-size 4
```

Expected: Training runs, outputs `models/atri.pth` + `models/atri.index`.

**Step 3: Commit**

```bash
git add scripts/train.py
git commit -m "feat: add RVC training script"
```

---

### Task 6: Inference Pipeline — Vocal Separation

**Files:**
- Create: `~/PycharmProjects/atri-rvc/scripts/atri_sing.py` (partial — separation step)

**Step 1: Write vocal separation function**

```python
"""ATRI singing voice conversion CLI."""
import argparse
import subprocess
from pathlib import Path

def separate_vocals(input_path: Path, tmp_dir: Path) -> tuple[Path, Path]:
    """Use demucs to separate vocals from accompaniment."""
    tmp_dir.mkdir(parents=True, exist_ok=True)
    print(f"[1/3] Separating vocals with demucs...")

    subprocess.run(
        ["python", "-m", "demucs", "-n", "htdemucs", "--two-stems", "vocals",
         "-o", str(tmp_dir), str(input_path)],
        check=True
    )

    stem_name = input_path.stem
    vocals = tmp_dir / "htdemucs" / stem_name / "vocals.wav"
    no_vocals = tmp_dir / "htdemucs" / stem_name / "no_vocals.wav"

    if not vocals.exists() or not no_vocals.exists():
        raise FileNotFoundError(f"Demucs output not found in {tmp_dir / 'htdemucs' / stem_name}")

    print(f"  Vocals: {vocals}")
    print(f"  Accompaniment: {no_vocals}")
    return vocals, no_vocals
```

**Step 2: Test separation standalone**

```bash
uv run python -c "
from pathlib import Path
from scripts.atri_sing import separate_vocals
v, nv = separate_vocals(Path('test_song.mp3'), Path('output/tmp'))
print('OK:', v, nv)
"
```

**Step 3: Commit**

```bash
git add scripts/atri_sing.py
git commit -m "feat: add vocal separation step (demucs)"
```

---

### Task 7: Inference Pipeline — RVC Voice Conversion

**Files:**
- Modify: `~/PycharmProjects/atri-rvc/scripts/atri_sing.py` — add RVC conversion step

**Step 1: Add RVC conversion function**

```python
def convert_voice(vocals_path: Path, output_path: Path,
                  model_path: Path, index_path: Path,
                  f0_up_key: int = 0, index_rate: float = 0.5,
                  protect: float = 0.33):
    """Run RVC voice conversion on separated vocals."""
    print(f"[2/3] Converting voice with RVC...")

    from rvc_core.pipeline import VoiceConverter

    converter = VoiceConverter(
        model_path=model_path,
        index_path=index_path,
        hubert_path=Path(__file__).parent.parent / "pretrained" / "hubert_base.pt",
        rmvpe_path=Path(__file__).parent.parent / "pretrained" / "rmvpe.pt",
    )

    converter.convert(
        input_path=vocals_path,
        output_path=output_path,
        f0_up_key=f0_up_key,
        f0_method="rmvpe",
        index_rate=index_rate,
        protect=protect,
    )
    print(f"  Output: {output_path}")
```

**Step 2: Commit**

```bash
git add scripts/atri_sing.py
git commit -m "feat: add RVC voice conversion step"
```

---

### Task 8: Inference Pipeline — Mix & CLI Entry Point

**Files:**
- Modify: `~/PycharmProjects/atri-rvc/scripts/atri_sing.py` — add mixing and main()

**Step 1: Add mix function and CLI entry point**

```python
def mix_audio(vocals_path: Path, accompaniment_path: Path, output_path: Path,
              vocal_volume: float = 1.0):
    """Mix converted vocals back with accompaniment using ffmpeg."""
    print(f"[3/3] Mixing vocals + accompaniment...")

    filter_str = f"[0:a]volume={vocal_volume}[v];[1:a][v]amix=inputs=2:duration=longest"
    subprocess.run(
        ["ffmpeg", "-y", "-i", str(accompaniment_path), "-i", str(vocals_path),
         "-filter_complex", filter_str, str(output_path)],
        check=True
    )
    print(f"  Output: {output_path}")


def main():
    parser = argparse.ArgumentParser(description="ATRI singing voice conversion")
    parser.add_argument("input", type=Path, help="Input song file (mp3/wav/flac)")
    parser.add_argument("-o", "--output", type=Path, default=None, help="Output file path")
    parser.add_argument("--pitch", type=int, default=0, help="Pitch shift in semitones")
    parser.add_argument("--index-rate", type=float, default=0.5, help="Index feature mix rate")
    parser.add_argument("--protect", type=float, default=0.33, help="Consonant protection")
    parser.add_argument("--no-mix", action="store_true", help="Output vocals only (no remix)")
    parser.add_argument("--keep-tmp", action="store_true", help="Keep temporary files")
    parser.add_argument("--model", type=Path, default=None, help="RVC model path")
    parser.add_argument("--index", type=Path, default=None, help="RVC index path")
    args = parser.parse_args()

    project_root = Path(__file__).parent.parent
    model_path = args.model or project_root / "models" / "atri.pth"
    index_path = args.index or project_root / "models" / "atri.index"
    output_path = args.output or project_root / "output" / f"{args.input.stem}_atri.mp3"
    tmp_dir = project_root / "output" / "tmp"

    if not model_path.exists():
        print(f"Error: Model not found: {model_path}")
        print("Run training first: uv run python scripts/train.py")
        return 1

    # Step 1: Separate vocals
    vocals, accompaniment = separate_vocals(args.input, tmp_dir)

    # Step 2: RVC conversion
    vocals_converted = tmp_dir / "vocals_atri.wav"
    convert_voice(vocals, vocals_converted, model_path, index_path,
                  f0_up_key=args.pitch, index_rate=args.index_rate,
                  protect=args.protect)

    # Step 3: Mix or output vocals only
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if args.no_mix:
        import shutil
        shutil.copy2(vocals_converted, output_path)
    else:
        mix_audio(vocals_converted, accompaniment, output_path)

    # Cleanup
    if not args.keep_tmp:
        import shutil
        shutil.rmtree(tmp_dir, ignore_errors=True)

    print(f"\nDone! Output: {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

**Step 2: Verify CLI help works**

```bash
uv run python scripts/atri_sing.py --help
```

**Step 3: Commit**

```bash
git add scripts/atri_sing.py
git commit -m "feat: complete one-click CLI (separate → convert → mix)"
```

---

### Task 9: End-to-End Test

**Step 1: Prepare a short test song**

Use any mp3/wav file available to test the full pipeline.

**Step 2: Run full pipeline**

```bash
uv run python scripts/atri_sing.py test_song.mp3 -o output/test_atri.mp3 --keep-tmp
```

Expected: `output/test_atri.mp3` with ATRI's voice singing.

**Step 3: Verify intermediate files**

```bash
ls output/tmp/htdemucs/test_song/  # vocals.wav, no_vocals.wav
ls output/tmp/vocals_atri.wav       # converted vocals
ffprobe output/test_atri.mp3        # final output
```

**Step 4: Listen and evaluate quality**

Play the output file. Check:
- Voice sounds like ATRI
- Pitch is correct
- No major artifacts
- Accompaniment is clean

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: verified end-to-end pipeline"
```

---

## Task Dependency Graph

```
Task 1 (Scaffold) → Task 2 (Download Models)
                   → Task 3 (Prepare Dataset)
                   → Task 4 (Extract RVC Core) → Task 5 (Training)
                                                → Task 6 (Separation)
                                                → Task 7 (RVC Inference)
                                                → Task 8 (Mix + CLI)
                                                → Task 9 (E2E Test)
```

Task 4 is the most complex and will require significant research into the RVC codebase to extract the right modules. Tasks 6-8 can be done incrementally in the same file.

## Execution Notes

- **Task 4 is the crux.** RVC's codebase is tangled with Gradio/WebUI code. Extracting clean modules requires careful study. Budget extra time here.
- **MPS compatibility:** Some RVC ops may not support MPS. Fallback to CPU for those (e.g., faiss indexing always runs on CPU).
- **Training time:** ~1-2 hours on MPS with 2222 samples. Start training, work on inference pipeline while it runs.
- **f0 adjustment for singing:** ATRI's voice training data is speech-only. The `--pitch` parameter will be important to match song keys. Typical female anime voice may need +0 to +4 semitones depending on the source vocalist.
