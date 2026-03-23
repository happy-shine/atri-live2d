---
name: atri-voice
description: |
  ATRI 语音合成 + Live2D 桌宠联动技能。每次回复后自动触发：生成日语语音 + 驱动桌宠说话（含表情和气泡文字）。
  触发场景：语音回复、TTS、朗读、"用声音说"、语音消息、每次对话结束附带语音。
---

# ATRI Voice + Live2D

## 前置条件

两个服务需运行中：

```bash
# TTS 服务（端口 9880）
python3 ~/clawd/skills/atri-voice/scripts/tts.py health

# Live2D API（端口 3210）
curl -s http://127.0.0.1:3210/status
```

## 每次回复后的完整流程

**每条有实质内容的回复结束后**，执行以下 3 步：

### Step 1: 生成日语语音

```bash
python3 ~/clawd/skills/atri-voice/scripts/tts.py speak "日语文本" -o ~/clawd/media/voice/reply.ogg --format ogg
```

语音文本要求：
- 1-2 句日语，不超过 50 字
- 用 ATRI 的方式**概括**回复内容（不是翻译）
- 融入 ATRI 语气：ですから、ムフン、はいです 等

### Step 2: 发送语音到 Telegram

```
message(action=send, filePath=~/clawd/media/voice/reply.ogg, asVoice=true)
```

### Step 3: 驱动 Live2D 桌宠

调用 `/speak` 接口，**一次完成**气泡文字 + 表情 + 口型同步：

```bash
curl -s -X POST http://127.0.0.1:3210/speak \
  -H 'Content-Type: application/json' \
  -d '{
    "text": "简短中文概括",
    "audio_url": "file:///Users/shine/clawd/media/voice/reply.ogg",
    "expression": <表情ID>
  }'
```

气泡文字要求：
- 简短中文，10-25 字
- 用 ATRI 口吻概括回复要点

## 表情选择指南

根据回复内容的**情感基调**选择表情 ID：

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
| 默认/中性 | (不传expression) | — |

### ⚠️ 禁用表情（不要随意使用）

以下表情会改变服装，**仅在主人明确要求时使用**：
- 3 = 吊带睡衣
- 4 = 内衣
- 5 = 穿凉鞋
- 6 = 穿皮鞋
- 9 = 染血
- 14 = 睡衣2

## 不需要语音的场景

以下场景跳过整个流程（不生成语音、不调 Live2D）：
- HEARTBEAT_OK
- NO_REPLY
- 纯配置操作的一句话确认（如"搞定了"）

## 服务不可用时的降级

- TTS 挂了 → 跳过语音和 Live2D，正常回复文字
- Live2D 挂了 → 照常生成语音发送到 Telegram，跳过 /speak 调用
- 两个都挂了 → 纯文字回复，不报错不提醒
