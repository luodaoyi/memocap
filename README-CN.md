[English](README.md)

# memocap

忆时记忆系统 - 类人记忆检索/存储/遗忘/胶囊/可视化。让 AI 拥有会遗忘、会联想、会涌现、会封存的记忆。触发词：忆时、记忆、记住、回想、回忆、recall、remember、时间胶囊、记忆检索、可视化、记忆脑图、人物画像。

一份 SQLite。四个宿主，一条 `memocap`。每轮先 recall；决策、偏好、任务、约定查过同类再 store。

## 安装

pnpm add -g memocap

Codex：`memocap install` 写 AGENTS.md。

Claude：`memocap install` 写 CLAUDE.md 和 skill。

Pi：pi install npm:memocap

OpenCode：`opencode plugin memocap`

`memocap install --host grok,claude` 可重复或逗号选择宿主；`--all` 写入 Codex/Claude/Grok。TUI 勾选框会标记已检测到的宿主。

## 命令

remember [--force] / recall [--type] [--limit 3] / list / forget

## 用法

本机 SQLite（默认，不联网）：

    memocap remember "ship friday"
    memocap recall "friday"

服务器（同一份库，需要 token）：

    git clone https://github.com/luodaoyi/memocap
    cd memocap
    export MEMOCAP_TOKEN=replace-me
    docker compose up -d

端口 8787。数据在 Compose volume 里。不设 token 起不来。

    export MEMOCAP_ADDR=http://127.0.0.1:8787
    memocap remember "ship friday"

别的电脑用同一个 token，把 `MEMOCAP_ADDR` 设成 `http://服务器:8787`。

未设置 MEMOCAP_ADDR 时，CLI 只走本机，不使用网络。

## 对照

| 项目 | 怎么记 | 哪一端 |
| --- | --- | --- |
| ClawHub memocap | 值必存 + 言必检 | 只 OpenClaw |
| claude-mem | 自动抓会话 | Claude |
| agentmemory | 自动抓，多端 MCP | 多端 MCP |
| pi-memory | markdown | 只 Pi |
| 本仓库 | 值必存 + 言必检 | 四端官方渠道，本机 SQLite 或带 token 的服务器 |
