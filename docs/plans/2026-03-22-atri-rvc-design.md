# ATRI RVC 歌声转换设计

## 目标

使用 RVC（Retrieval-based Voice Conversion）实现 ATRI 声音的歌声转换。输入一首完整歌曲，一键输出 ATRI 声音版本。

## 需求

- 用现有 2222 个 ATRI 语音素材（`~/GPT-sovits/resource/vol1/ATR_*.opus`）训练 RVC 模型
- 一键 CLI：输入歌曲 → 人声分离 → RVC 变声 → 混回伴奏 → 输出
- 本地 macOS 运行（MPS 加速）
- 离线批处理，非实时

## 项目结构

新建独立项目 `~/PycharmProjects/atri-rvc/`：

```
atri-rvc/
├── scripts/
│   ├── prepare_dataset.py    # opus→wav 转换 + 数据集准备
│   ├── train.py              # 调用 RVC 训练流程
│   └── atri_sing.py          # 一键 CLI 入口
├── models/                   # 训练输出的 RVC 模型
├── pretrained/               # 预训练模型（hubert_base.pt, rmvpe.pt）
├── output/                   # 推理输出目录
├── pyproject.toml            # uv 项目配置
└── README.md
```

## 训练流程

### Step 1 — 数据准备（prepare_dataset.py）

- 读取 `~/GPT-sovits/resource/vol1/ATR_*.opus`（2222 个文件）
- ffmpeg 批量转 wav（16bit, 单声道, 44100Hz）
- 输出到 `atri-rvc/dataset/atri/`
- 过滤过短样本（< 1秒）和纯效果音

### Step 2 — RVC 训练（train.py）

调用 RVC 训练 pipeline：

1. 预处理：重采样 + 切片
2. 特征提取：HUBERT（hubert_base.pt）
3. f0 提取：rmvpe（唱歌场景精度最高）
4. 模型训练

训练参数：
- 采样率：40k
- f0 提取：rmvpe
- epochs：200-300
- batch_size：4-8（macOS MPS）

输出：`models/atri.pth` + `models/atri.index`

预计训练时间：macOS MPS 上约 1-2 小时

## 推理流程（atri_sing.py）

### 用法

```bash
python scripts/atri_sing.py input.mp3 -o output.mp3

# 可选参数
python scripts/atri_sing.py input.mp3 -o output.mp3 \
  --pitch 2 \          # 变调（半音）
  --index-rate 0.5 \   # 检索特征混合比
  --no-mix \           # 只输出变声人声
  --keep-tmp           # 保留中间文件
```

### Step 1 — 人声分离（demucs）

- 输入：input.mp3
- 使用 `demucs -n htdemucs --two-stems vocals`
- 输出：`vocals.wav`（人声）+ `no_vocals.wav`（伴奏）
- 临时文件存放在 `output/tmp/`

### Step 2 — RVC 变声

- 加载 `models/atri.pth` + `models/atri.index`
- 输入 `vocals.wav` → RVC 推理
- 参数：
  - f0_method: rmvpe
  - f0_up_key: 用户指定的变调（默认 0）
  - index_rate: 0.5
  - protect: 0.33（保护清辅音）
- 输出：`vocals_atri.wav`

### Step 3 — 混合输出（ffmpeg）

- ffmpeg 将 `vocals_atri.wav` + `no_vocals.wav` 混合
- 输出：`output.mp3`

### 错误处理

- 每步失败时清晰报错并保留中间文件供调试
- `--keep-tmp` 保留所有临时文件

## 环境与依赖

- **包管理**：uv + pyproject.toml
- **Python**：3.10+
- **RVC 核心代码**：提取推理核心到项目中（轻量自包含，不用 submodule）

关键依赖：
- torch + torchaudio（MPS 加速）
- fairseq（HUBERT 特征提取）
- demucs（人声分离）
- faiss-cpu（检索索引）
- scipy, librosa, soundfile（音频处理）
- 系统 ffmpeg

预训练模型下载：
- `hubert_base.pt`（~190MB）→ `pretrained/`
- `rmvpe.pt`（~140MB）→ `pretrained/`

## 方案选型说明

选择 RVC-WebUI 核心引擎改造方案（方案 A），因为：
- RVC 是 SVC 领域最成熟方案
- 训练快，唱歌效果好
- 社区大，文档完善
- 与现有 GPT-SoVITS 环境共享部分依赖

GPT-SoVITS 的 ATR_e8_s3952.pth 不能直接用于 RVC（架构不同），需用同一批音频素材重新训练 RVC 模型。
