#!/usr/bin/env node
"use strict";

const { spawnSync } = require("child_process");
const fs = require("fs");
const https = require("https");
const os = require("os");
const path = require("path");

const VERSION = require("../package.json").version;

const ASSETS = {
  "linux-x64": "memocap-x86_64-unknown-linux-gnu",
  "darwin-arm64": "memocap-aarch64-apple-darwin",
  "win32-x64": "memocap-x86_64-pc-windows-msvc.exe",
};

function assetName() {
  const key = `${process.platform}-${process.arch}`;
  const name = ASSETS[key];
  if (!name) {
    console.error(
      `memocap: unsupported platform ${process.platform}/${process.arch}. Supported: linux/x64, darwin/arm64, win32/x64.`,
    );
    process.exit(1);
  }
  return name;
}

function cacheDir() {
  if (process.platform === "darwin") {
    return path.join(os.homedir(), "Library", "Caches", "memocap", VERSION);
  }
  if (process.platform === "win32") {
    const base =
      process.env.LOCALAPPDATA || path.join(os.homedir(), "AppData", "Local");
    return path.join(base, "memocap", VERSION);
  }
  const base = process.env.XDG_CACHE_HOME || path.join(os.homedir(), ".cache");
  return path.join(base, "memocap", VERSION);
}

function download(url, dest) {
  const tmp = `${dest}.partial`;
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(tmp);
    const fail = (err) => {
      file.close(() => {
        fs.unlink(tmp, () => reject(err));
      });
    };
    const get = (current, hops) => {
      if (hops > 5) {
        fail(new Error("too many redirects"));
        return;
      }
      https
        .get(current, { headers: { "User-Agent": "memocap" } }, (res) => {
          if (
            res.statusCode >= 300 &&
            res.statusCode < 400 &&
            res.headers.location
          ) {
            res.resume();
            get(res.headers.location, hops + 1);
            return;
          }
          if (res.statusCode !== 200) {
            res.resume();
            fail(new Error(`download failed: HTTP ${res.statusCode} ${current}`));
            return;
          }
          res.pipe(file);
          file.on("finish", () => {
            file.close((err) => {
              if (err) {
                fail(err);
                return;
              }
              resolve();
            });
          });
        })
        .on("error", fail);
    };
    file.on("error", fail);
    get(url, 0);
  }).then(() => {
    fs.renameSync(tmp, dest);
    fs.chmodSync(dest, 0o755);
  });
}

async function resolveBinary() {
  if (process.env.MEMOCAP_BINARY) {
    return process.env.MEMOCAP_BINARY;
  }
  const name = assetName();
  const dir = cacheDir();
  fs.mkdirSync(dir, { recursive: true });
  const dest = path.join(dir, name);
  if (fs.existsSync(dest) && fs.statSync(dest).size > 0) {
    return dest;
  }
  const url = `https://github.com/luodaoyi/memocap/releases/download/v${VERSION}/${name}`;
  await download(url, dest);
  return dest;
}

async function main() {
  try {
    const bin = await resolveBinary();
    const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
    if (result.error) {
      console.error(`memocap: ${result.error.message}`);
      process.exit(1);
    }
    process.exit(result.status === null ? 1 : result.status);
  } catch (err) {
    console.error(`memocap: ${err.message}`);
    process.exit(1);
  }
}

main();
