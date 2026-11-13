#!/usr/bin/env python3
"""Build the brand set in docs/assets/brand.

    pip install fonttools uharfbuzz pillow
    python tools/brand/build.py --fonts DIR      # writes the SVGs and banner pages
    node tools/brand/render.mjs                  # rasterises them with Chromium (Playwright)
    python tools/brand/build.py --fonts DIR --ico

DIR must hold Inter-SemiBold.ttf, Inter-Regular.ttf, Inter-Medium.ttf and
Hind-SemiBold.ttf (both families: SIL Open Font License 1.1). Text in the
SVGs is shaped with HarfBuzz and written as outlines, so the published files
carry no font.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from glyphs import Face

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs" / "assets" / "brand"
WORK = Path(__file__).resolve().parent / ".work"

BLUE_LIGHT, BLUE_DARK = "#1f62b3", "#256abf"
ACCENT_LIGHT, ACCENT_DARK = "#2a78d6", "#3987e5"
INK_LIGHT, INK_DARK = "#0b0b0b", "#f0efec"
SECOND_LIGHT, SECOND_DARK = "#63625d", "#a3a29a"
PAGE_LIGHT, PAGE_DARK = "#f6f6f4", "#0d0d0d"


def bars(x: float, cy: float, heights=(7, 13, 5), w: float = 2.3, gap: float = 1.0) -> str:
    return "".join(
        f'<rect x="{x + i * (w + gap):.2f}" y="{cy - h / 2:.2f}" width="{w}" height="{h}" rx="{w / 2}"/>' for i, h in enumerate(heights)
    )


def mark_shapes(hind: Face, small: bool = False) -> str:
    """White shapes on the 32-unit grid: the letter उ, then the voice bars."""
    if small:
        d, _ = hind.path("उ", 23.5, 3.0, 24.6)
        return f'<path d="{d}"/>' + bars(20.0, 16.0, heights=(8, 14), w=3.0, gap=2.0)
    d, _ = hind.path("उ", 21.5, 3.4, 23.9)
    return f'<path d="{d}"/>' + bars(19.2, 16.6)


def tile(shapes: str, blue: str, size: int = 64, clip_id: str = "c0", rx: float = 7) -> str:
    k = size / 32
    return (
        f'<g transform="scale({k:g})"><clipPath id="{clip_id}"><rect width="32" height="32" rx="{rx}"/></clipPath>'
        f'<rect width="32" height="32" rx="{rx}" fill="{blue}"/><g clip-path="url(#{clip_id})"><g fill="#fff">{shapes}</g></g></g>'
    )


def svg(w: float, h: float, body: str, label: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:g} {h:g}" width="{w:g}" height="{h:g}" role="img" aria-label="{label}">\n'
        f"<title>{label}</title>\n{body}\n</svg>\n"
    )


def wordmark(inter: Face, hind: Face, dark: bool, with_mark: bool = True) -> tuple[str, float]:
    ink, second, blue = (INK_DARK, SECOND_DARK, BLUE_DARK) if dark else (INK_LIGHT, SECOND_LIGHT, BLUE_LIGHT)
    x = 82.0 if with_mark else 0.0
    base = 52.6
    latin, lw = inter.path("uccharan", 56, x, base, tracking=-0.035)
    dot_x = x + lw + 15
    deva, dw = hind.path("उच्चारण", 49, dot_x + 17, base)
    body = (tile(mark_shapes(hind), blue, 64, "c1") if with_mark else "") + (
        f'<path fill="{ink}" d="{latin}"/><circle cx="{dot_x:.2f}" cy="{base - 15:.2f}" r="3" fill="{second}"/><path fill="{second}" d="{deva}"/>'
    )
    return body, dot_x + 17 + dw


def page(dark: bool, w: int, h: int, fonts: Path, inner: str, radius: int = 0) -> str:
    bg, ink, second = (PAGE_DARK, INK_DARK, SECOND_DARK) if dark else (PAGE_LIGHT, INK_LIGHT, SECOND_LIGHT)
    accent = ACCENT_DARK if dark else ACCENT_LIGHT
    face = "".join(
        f"@font-face{{font-family:Inter;font-weight:{wt};src:url('file://{fonts}/Inter-{name}.ttf')}}"
        for wt, name in ((400, "Regular"), (500, "Medium"), (600, "SemiBold"))
    )
    return f"""<!doctype html><html><head><meta charset="utf-8"><style>{face}
html,body{{margin:0;background:transparent}}
.card{{position:relative;width:{w}px;height:{h}px;background:{bg};border-radius:{radius}px;overflow:hidden;font-family:Inter,sans-serif;color:{ink};
  box-shadow:inset 0 0 0 1px {'#2c2c2a' if dark else '#e1e0d9'}}}
.top{{position:absolute;left:0;top:0;width:100%;height:{max(6, h // 50)}px;background:{accent}}}
.second{{color:{second}}}
.ipa{{font-family:'DejaVu Sans',sans-serif}}
</style></head><body><div class="card"><div class="top"></div>{inner}</div></body></html>"""


def motif(dark: bool, x0: int, top: int, scale: float) -> str:
    """Letters turning into sound: phoneme symbols above the hanging voice bars."""
    accent = ACCENT_DARK if dark else ACCENT_LIGHT
    lengths = [215, 110, 317, 160, 400, 95, 260, 143, 359, 78, 235, 127]
    out = ""
    for i, ln in enumerate(lengths):
        op = 1.0 if i % 2 == 0 else 0.32
        if i % 4 == 2:
            op = 0.72
        out += (
            f'<div style="position:absolute;left:{x0 + i * 45 * scale:.0f}px;top:{top}px;width:{17 * scale:.0f}px;height:{ln * scale:.0f}px;'
            f'background:{accent};opacity:{op};border-radius:0 0 {9 * scale:.0f}px {9 * scale:.0f}px"></div>'
        )
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--fonts", type=Path, required=True)
    ap.add_argument("--ico", action="store_true", help="assemble favicon.ico from the rendered PNGs")
    args = ap.parse_args()
    fav = OUT / "favicon"
    if args.ico:
        from PIL import Image

        imgs = [Image.open(fav / f"favicon-{s}.png").convert("RGBA") for s in (16, 32, 48)]
        imgs[-1].save(fav / "favicon.ico", sizes=[(16, 16), (32, 32), (48, 48)], append_images=imgs[:-1])
        print("favicon.ico")
        return
    fonts = args.fonts.resolve()
    inter, hind = Face(str(fonts / "Inter-SemiBold.ttf")), Face(str(fonts / "Hind-SemiBold.ttf"))
    fav.mkdir(parents=True, exist_ok=True)
    WORK.mkdir(exist_ok=True)
    big, small = mark_shapes(hind), mark_shapes(hind, small=True)

    (OUT / "logo-mark.svg").write_text(svg(64, 64, tile(big, BLUE_LIGHT), "uccharan"), encoding="utf-8")
    (OUT / "logo-mark-small.svg").write_text(svg(32, 32, tile(small, BLUE_LIGHT, 32), "uccharan"), encoding="utf-8")
    (OUT / "logo-mark-mono.svg").write_text(svg(64, 64, f'<g transform="scale(2)" fill="currentColor">{big}</g>', "uccharan"), encoding="utf-8")
    (fav / "favicon.svg").write_text(svg(32, 32, tile(small, BLUE_LIGHT, 32), "uccharan"), encoding="utf-8")
    for dark in (True, False):
        body, width = wordmark(inter, hind, dark)
        name = "dark" if dark else "light"
        (OUT / f"logo-wordmark-{name}.svg").write_text(svg(round(width + 2), 64, body, "uccharan · उच्चारण"), encoding="utf-8")

    # Full-bleed app icons (no rounded corners: the platform masks them).
    full = svg(64, 64, tile(big, BLUE_LIGHT, 64, "c0", rx=0), "uccharan")
    (WORK / "icon-full.svg").write_text(full, encoding="utf-8")
    (WORK / "icon-small.svg").write_text(svg(32, 32, tile(small, BLUE_LIGHT, 32), "uccharan"), encoding="utf-8")
    (WORK / "icon-mark.svg").write_text(svg(64, 64, tile(big, BLUE_LIGHT), "uccharan"), encoding="utf-8")

    # Banners (3200 x 960 at 2x) and the social preview (1280 x 640).
    for dark in (True, False):
        body, width = wordmark(inter, hind, dark)
        k = 2.0
        second = SECOND_DARK if dark else SECOND_LIGHT
        inner = (
            f'<svg style="position:absolute;left:140px;top:118px" width="{width * k:.0f}" height="{64 * k:.0f}" viewBox="0 0 {width:.0f} 64">{body}</svg>'
            '<div style="position:absolute;left:140px;top:282px;font-weight:600;font-size:62px;line-height:72px;letter-spacing:-0.022em">5× faster than espeak-ng.<br>Just as accurate. Free to ship.</div>'
            f'<div class="second" style="position:absolute;left:140px;top:448px;font-size:29px;line-height:41px;width:1120px">'
            "A drop-in phonemiser for Piper, Kokoro and sherpa-onnx voices — English and Hindi, in Rust, Python, C and the browser. Apache-2.0.</div>"
            f'<div class="ipa" style="position:absolute;left:1350px;top:500px;font-size:32px;color:{second};letter-spacing:0.04em">उच्चारण → /ʊcːˈaːɾəɳ/</div>'
            + motif(dark, 1350, 0, 1.0)
        )
        (WORK / f"banner-{'dark' if dark else 'light'}.html").write_text(page(dark, 2000, 600, fonts, inner, radius=22), encoding="utf-8")
    body, width = wordmark(inter, hind, True)
    k = 1.55
    inner = (
        f'<svg style="position:absolute;left:96px;top:100px" width="{width * k:.0f}" height="{64 * k:.0f}" viewBox="0 0 {width:.0f} 64">{body}</svg>'
        '<div style="position:absolute;left:96px;top:250px;font-weight:600;font-size:52px;line-height:62px;letter-spacing:-0.022em">5× faster than espeak-ng.<br>Just as accurate. Free to ship.</div>'
        f'<div class="second" style="position:absolute;left:96px;top:398px;font-size:25px;line-height:36px;width:880px">'
        "A drop-in phonemiser for Piper, Kokoro and sherpa-onnx voices — English and Hindi, in Rust, Python, C and the browser. Apache-2.0.</div>"
        f'<div style="position:absolute;left:96px;top:548px;font-size:23px;font-weight:500">English <span class="second">&nbsp;•&nbsp;</span> Hindi <span class="second">&nbsp;•&nbsp;</span> Hinglish'
        f'<span class="second"> &nbsp;•&nbsp; Rust · Python · C · WebAssembly &nbsp;•&nbsp; Apache-2.0</span></div>'
        + motif(True, 940, 0, 0.73)
    )
    (WORK / "social-preview.html").write_text(page(True, 1280, 640, fonts, inner), encoding="utf-8")
    button = (
        f"<!doctype html><html><head><meta charset='utf-8'><style>@font-face{{font-family:Inter;font-weight:600;src:url('file://{fonts}/Inter-SemiBold.ttf')}}"
        "html,body{margin:0;background:transparent}.b{display:inline-flex;align-items:center;gap:12px;height:52px;padding:0 26px;border-radius:12px;"
        f"background:{BLUE_LIGHT};color:#fff;font:600 21px Inter,sans-serif}}</style></head><body><div class='b' id='b'>Try the live demo"
        "<svg width='18' height='18' viewBox='0 0 18 18' fill='none' stroke='#fff' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'><path d='M4 9h10M9.5 4.5 14 9l-4.5 4.5'/></svg></div></body></html>"
    )
    (WORK / "button.html").write_text(button, encoding="utf-8")
    # Review sheet: the mark at 16/32/64/256 on dark and light.
    sheet = "<html><body style='margin:0;font:13px sans-serif'>"
    for bg, fg in ((PAGE_DARK, "#ccc"), (PAGE_LIGHT, "#333")):
        sheet += f"<div style='background:{bg};color:{fg};padding:20px'>"
        for label, shapes, sizes in (("logo-mark", big, (20, 32, 64, 256)), ("logo-mark-small", small, (16, 18, 20, 32))):
            sheet += f"<div style='margin:8px 0'>{label}: " + "".join(
                f"<span style='display:inline-block;width:{s}px;height:{s}px;margin:8px;vertical-align:bottom'>"
                f"<svg viewBox='0 0 32 32' width='{s}' height='{s}'>{tile(shapes, BLUE_LIGHT, 32, f'k{s}{label}')}</svg></span>" for s in sizes
            ) + "</div>"
        dark = bg == PAGE_DARK
        body, width = wordmark(inter, hind, dark)
        sheet += f"<svg width='{width:.0f}' height='64' viewBox='0 0 {width:.0f} 64'>{body}</svg> &nbsp; <svg width='{width * 0.45:.0f}' height='29' viewBox='0 0 {width:.0f} 64'>{body}</svg></div>"
    (WORK / "sheet.html").write_text(sheet + "</body></html>", encoding="utf-8")
    print("wrote SVGs to", OUT, "and pages to", WORK)


if __name__ == "__main__":
    main()
