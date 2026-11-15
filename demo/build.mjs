// Assemble the static demo site into demo/_site:
//   node bindings/wasm/build.mjs && node demo/build.mjs [--xlit DIR]
// --xlit DIR adds the transliteration packs (xlit-*.ucd, CC BY-SA 4.0, from the
// release page) found in DIR whose SHA-256 matches this version; without them
// the page's transliteration falls back to the plain rule converter.
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const site = join(here, "_site");
rmSync(site, { recursive: true, force: true });
mkdirSync(join(site, "pkg"), { recursive: true });
for (const f of ["index.html", "style.css", "app.js"]) cpSync(join(here, f), join(site, f));
const pkg = join(root, "bindings", "wasm");
for (const f of ["index.js", "packs.js", "dist", "data"]) cpSync(join(pkg, f), join(site, "pkg", f), { recursive: true });
const brand = join(root, "docs", "assets", "brand");
cpSync(join(brand, "logo-mark.svg"), join(site, "logo-mark.svg"));
cpSync(join(brand, "favicon", "favicon.svg"), join(site, "favicon.svg"));
cpSync(join(brand, "favicon", "favicon.ico"), join(site, "favicon.ico"));
cpSync(join(brand, "favicon", "apple-touch-icon.png"), join(site, "apple-touch-icon.png"));
cpSync(join(brand, "social-preview.png"), join(site, "og.png"));
for (const f of ["LICENSE", "NOTICE", "DATA_LICENSES.md"]) cpSync(join(root, f), join(site, f));
const i = process.argv.indexOf("--xlit");
const xlitDir = i > 0 ? process.argv[i + 1] : null;
if (xlitDir && existsSync(xlitDir)) {
  const { PACKS } = await import(new URL("../bindings/wasm/packs.js", import.meta.url));
  let n = 0;
  for (const f of readdirSync(xlitDir).filter((f) => /^xlit-[a-z-]+\.ucd$/.test(f)).sort()) {
    const bytes = readFileSync(join(xlitDir, f));
    const want = PACKS[f.slice(0, -4)];
    const got = createHash("sha256").update(bytes).digest("hex");
    if (!want || want.sha256 !== got) { console.warn(`skipped ${f}: not the pack of this version`); continue; }
    cpSync(join(xlitDir, f), join(site, "pkg", "data", f));
    n++;
  }
  // The notice names the licence file as LICENSE-CC-BY-SA-4.0.txt next to it.
  for (const [from, to] of [["xlit-NOTICE.txt", "xlit-NOTICE.txt"], ["xlit-LICENSE-CC-BY-SA-4.0.txt", "LICENSE-CC-BY-SA-4.0.txt"]]) {
    if (existsSync(join(xlitDir, from))) cpSync(join(xlitDir, from), join(site, "pkg", "data", to));
  }
  console.log(`  ${n} transliteration packs`);
}
const kb = (p) => `${(statSync(p).size / 1024).toFixed(0)} KiB`;
console.log(`site: ${site}`);
console.log(`  wasm ${kb(join(site, "pkg", "dist", "uccharan_wasm_bg.wasm"))}, en-us ${kb(join(site, "pkg", "data", "en-us.ucd"))}, hi ${kb(join(site, "pkg", "data", "hi.ucd"))}`);
