# Changelog

## 0.1.4 — 2026-09-03

`install` 先探测本机有哪些宿主，再按选择写入，不再一律装 Codex+Claude。

- 探测 Codex、Claude、Grok、Pi、OpenCode。`--host` 可重复或逗号分隔（`codex,claude,grok,pi,opencode`）；`--all` 只覆盖会写文件的 Codex/Claude/Grok。
- 交互 TUI 勾选；已探测到的标 `*`。非交互且未给 `--host` 时，装已探测到的写文件宿主；一个都没有则回退 Codex+Claude。
- Grok 写入 `~/.grok/skills/memocap/SKILL.md`（项目目录则 `./.grok/skills/memocap/SKILL.md`）；仅当 `AGENTS.md` 已存在才 upsert。
- Pi / OpenCode 只打印 `pi install npm:memocap` / `opencode plugin memocap`，不造假文件。
- `uninstall` / `status` 同样认 `--host`。

## 0.1.3 — 2026-09-02

记住前先查重，召回默认少灌一点。

- `remember` 先用 FTS 查同类，撞到就不写；`--force` 才插入，`--id` 覆盖已有行。HTTP `POST /remember` 同样规则，冲突返回 409。
- `recall` 默认 3 条（原先 5），可 `--type` 按 kind 过滤、`--max-chars` 限制总字数；排序在 FTS 之后按新近。
- README 补了忆时记忆系统说明。

## 0.1.2 — 2026-08-25

Docker 镜像升到 rust 1.88；Compose 部署写进 README。Release Action 发三平台二进制和 npm。

## 0.1.1 — 2026-08-25

npm bin 改为 `bin/cli.cjs`，从 GitHub Release 拉二进制。Trusted Publisher 走 Action 发版。

## 0.1.0 — 2026-08-25

第一版。一份 SQLite，四端共用 `remember` / `recall` / `list` / `forget`。不设地址只走本机；设了 ADDR 和 token 走 HTTP / Compose 8787。
