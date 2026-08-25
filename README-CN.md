[English](README.md)

# memocap

一份 SQLite。四个宿主，一条 `memocap`。每轮先 recall；决策、偏好、任务、约定查过同类再 store。

## 安装

pnpm add -g memocap

Codex：`memocap install` 写 AGENTS.md。

Claude：`memocap install` 写 CLAUDE.md 和 skill。

Pi：pi install npm:memocap

OpenCode：`opencode plugin memocap`

## 命令

remember / recall / list / forget

## 用法

本机 SQLite（默认，不联网）：

    memocap remember "ship friday"
    memocap recall "friday"

服务器（同一份库，需要 token）：

    export MEMOCAP_TOKEN=replace-me
    docker compose up -d
    export MEMOCAP_ADDR=http://127.0.0.1:8787
    memocap remember "ship friday"

未设置 MEMOCAP_ADDR 时，CLI 只走本机，不使用网络。

## 对照

| 项目 | 怎么记 | 哪一端 |
| --- | --- | --- |
| ClawHub memocap | 值必存 + 言必检 | 只 OpenClaw |
| claude-mem | 自动抓会话 | Claude |
| agentmemory | 自动抓，多端 MCP | 多端 MCP |
| pi-memory | markdown | 只 Pi |
| 本仓库 | 值必存 + 言必检 | 四端官方渠道，本机 SQLite 或带 token 的服务器 |
