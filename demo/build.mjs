// Assemble the static demo site into demo/_site:
//   node bindings/wasm/build.mjs && node demo/build.mjs
import { cpSync, mkdirSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const site = join(here, "_site");
rmSync(site, { recursive: true, force: true });
mkdirSync(join(site, "pkg"), { recursive: true });
for (const f of ["index.html", "style.css", "app.js"]) cpSync(join(here, f), join(site, f));
const pkg = join(root, "bindings", "wasm");
for (const f of ["index.js", "dist", "data"]) cpSync(join(pkg, f), join(site, "pkg", f), { recursive: true });
const brand = join(root, "docs", "assets", "brand");
cpSync(join(brand, "logo-mark.svg"), join(site, "logo-mark.svg"));
cpSync(join(brand, "favicon", "favicon.svg"), join(site, "favicon.svg"));
cpSync(join(brand, "favicon", "favicon.ico"), join(site, "favicon.ico"));
cpSync(join(brand, "favicon", "apple-touch-icon.png"), join(site, "apple-touch-icon.png"));
cpSync(join(brand, "social-preview.png"), join(site, "og.png"));
for (const f of ["LICENSE", "NOTICE", "DATA_LICENSES.md"]) cpSync(join(root, f), join(site, f));
const kb = (p) => `${(statSync(p).size / 1024).toFixed(0)} KiB`;
console.log(`site: ${site}`);
console.log(`  wasm ${kb(join(site, "pkg", "dist", "uccharan_wasm_bg.wasm"))}, en-us ${kb(join(site, "pkg", "data", "en-us.ucd"))}, hi ${kb(join(site, "pkg", "data", "hi.ucd"))}`);
