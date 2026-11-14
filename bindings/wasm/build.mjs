// Builds the npm package in place: compiles the crate to wasm32, runs
// wasm-bindgen (ESM "web" target, which also works in Node when the wrapper
// hands it the bytes), and copies the language data and licence files.
//
// Requirements: the wasm32-unknown-unknown Rust target and a wasm-bindgen CLI
// whose version matches the `wasm-bindgen` crate pinned in Cargo.toml.
import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const run = (cmd, args, cwd = root) => execFileSync(cmd, args, { cwd, stdio: "inherit" });

run("cargo", ["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "uccharan-wasm"]);
rmSync(join(here, "dist"), { recursive: true, force: true });
run("wasm-bindgen", [
  "--target", "web",
  "--out-dir", join(here, "dist"),
  "--out-name", "uccharan_wasm",
  "--omit-default-module-path",
  join(root, "target", "wasm32-unknown-unknown", "release", "uccharan_wasm.wasm"),
]);
rmSync(join(here, "dist", "package.json"), { force: true });
rmSync(join(here, "dist", ".gitignore"), { force: true });

mkdirSync(join(here, "data"), { recursive: true });
for (const name of ["en-us", "en-gb", "hi", "ur", "sd", "de", "nl", "fr", "it", "pt-br", "pt", "pl", "tr", "cs", "sv", "ca", "ro", "hu", "fi", "nb", "da", "kokoro-en-us", "kokoro-en-gb"]) {
  const builtIn = join(root, "crates", "uccharan", "data", `${name}.ucd`);
  const table = join(root, "crates", "uccharan-data", "data", `${name}.ucd`);
  cpSync(existsSync(builtIn) ? builtIn : table, join(here, "data", `${name}.ucd`));
}
for (const f of ["LICENSE", "NOTICE", "DATA_LICENSES.md"]) {
  cpSync(join(root, f), join(here, f));
}
const kb = (p) => (statSync(p).size / 1024).toFixed(0);
console.log(`wasm: ${kb(join(here, "dist", "uccharan_wasm_bg.wasm"))} KiB`);
for (const lang of ["en-us", "en-gb", "hi", "ur", "sd", "de", "nl", "fr", "it", "pt-br", "pt", "pl", "tr", "cs", "sv", "ca", "ro", "hu", "fi", "nb", "da", "kokoro-en-us", "kokoro-en-gb"]) console.log(`data/${lang}.ucd: ${kb(join(here, "data", `${lang}.ucd`))} KiB`);
