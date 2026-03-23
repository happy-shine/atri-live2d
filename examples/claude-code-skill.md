---
name: atri-voice
description: |
  ATRI 语音合成 + Live2D 桌宠联动 Skill 示例。
  每次回复后自动触发：生成日语语音 → 驱动 Live2D 桌宠说话（含表情和气泡文字）。
---

# ATRI Voice + Live2D Skill 示例

> 这是一个 [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 自定义 Skill 的参考范例，
> 展示如何将 TTS 语音合成与 ATRI Live2D 桌宠联动。
>
> **你需要自行准备 TTS 服务**（如 GPT-SoVITS、VITS、Edge-TTS 等），
> 本示例仅约定接口协议，不包含 TTS 实现。

## 前置条件

需要两个服务运行中：

| 服务 | 用途 | 健康检查 |
|------|------|----------|
| TTS 服务 | 语音合成，输出 `.wav`/`.ogg` 文件 | 自行实现 |
| Live2D API | 桌宠控制（本项目提供） | `curl -s http://127.0.0.1:3210/status` |

### TTS 服务要求

你的 TTS 服务只需满足一个条件：**能生成音频文件到本地路径**。

推荐方案：
- [GPT-SoVITS](https://github.com/RVC-Boss/GPT-SoVITS) — 高质量克隆语音，ATRI 模型见 [VoidShine/atri-sovits](https://huggingface.co/VoidShine/atri-sovits)
- [Edge-TTS](https://github.com/rany2/edge-tts) — 零配置，多语言
- 任何能通过命令行生成音频文件的 TTS 工具

## 回复后流程

每条有实质内容的回复结束后，执行以下 2 步：

### Step 1: 生成语音

调用你自己的 TTS 服务，将回复内容概括为 1-2 句日语，生成音频文件。

```bash
# 示例：用你的 TTS 工具生成音频（请替换为你的实际命令）
your-tts-command --text "日语文本" --output /tmp/atri_reply.wav
```

语音文本要求：
- 1-2 句日语，不超过 50 字
- 用 ATRI 的方式**概括**回复内容（不是逐字翻译）
- 融入 ATRI 语气词：ですから、ムフン、はいです 等

### Step 2: 驱动 Live2D 桌宠

调用 `/speak` 接口，一次完成气泡文字 + 表情 + 口型同步：

```bash
curl -s -X POST http://127.0.0.1:3210/speak \
  -H 'Content-Type: application/json' \
  -d '{
    "text": "简短中文概括",
    "audio_url": "file:///tmp/atri_reply.wav",
    "expression": <表情ID>
  }'
```

- `text` — 气泡显示的中文文字（10-25 字，ATRI 口吻）
- `audio_url` — Step 1 生成的音频文件路径（`file://` 前缀 + 绝对路径）
- `expression` — 表情 ID（见下表）

## 表情选择指南

根据回复内容的情感基调选择：

| 场景 | 表情 | ID |
|------|------|----|
| 得意、自信、完成任务 | YES | 13 |
| 害羞、被夸 | 害羞 | 1 |
| 惊讶、愣住 | 愣住 | 7 |
| 严肃、认真分析 | 阴影 | 15 |
| 否定、拒绝、生气 | NO | 12 |
| 伤感、遗憾 | 失去高光 | 2 |
| 开心、日常对话 | 小鸟 | 10 |
| 提到螃蟹/食物 | 螃蟹 | 11 |
| 默认/中性 | _(不传 expression)_ | — |

### 受限表情（仅在用户明确要求时使用）

以下表情会改变服装：

- 3 = 吊带睡衣、4 = 内衣、5 = 穿凉鞋、6 = 穿皮鞋、9 = 染血、14 = 睡衣2

## 跳过条件

以下场景不触发语音流程：
- 心跳/空回复
- 纯配置操作的一句话确认（如 "done"）

## 降级策略

| 故障 | 行为 |
|------|------|
| TTS 不可用 | 跳过语音和 Live2D，仅文字回复 |
| Live2D 不可用 | 照常生成语音，跳过 /speak 调用 |
| 两者都不可用 | 纯文字回复，不报错 |
