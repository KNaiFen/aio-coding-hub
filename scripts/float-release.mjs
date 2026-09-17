import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));

export function validateFloatVersion({ cargo, config, lock }, tag) {
  const version = config.version;
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error("Invalid Float version");
  }
  if (tag !== undefined && tag !== `aio-float-v${version}`) {
    throw new Error("Float release tag does not match its version");
  }
  const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargo)?.[1];
  const lockVersion = /\[\[package\]\]\s+name\s*=\s*"aio-float"\s+version\s*=\s*"([^"]+)"/m.exec(lock)?.[1];
  if (cargoVersion !== version || lockVersion !== version) {
    throw new Error("Float Cargo, Tauri and lock versions must agree");
  }
  return version;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [command, tag] = process.argv.slice(2);
  if (command !== "validate") throw new Error("Expected validate [aio-float-vX.Y.Z]");
  const read = (path) => readFileSync(join(root, path), "utf8");
  const version = validateFloatVersion({
    cargo: read("src-tauri/crates/aio-float/Cargo.toml"),
    config: JSON.parse(read("src-tauri/crates/aio-float/tauri.conf.json")),
    lock: read("src-tauri/Cargo.lock"),
  }, tag);
  console.log(`Float ${version}: versions consistent`);
}
