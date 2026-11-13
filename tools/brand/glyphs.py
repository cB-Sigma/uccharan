"""Outline helpers for the brand build: shape text with HarfBuzz and return
SVG path data, so that the published SVGs carry no font."""

from __future__ import annotations

import uharfbuzz as hb
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont


class Face:
    def __init__(self, path: str) -> None:
        self.tt = TTFont(path)
        self.glyphs = self.tt.getGlyphSet()
        self.upem = self.tt["head"].unitsPerEm
        self.hb = hb.Font(hb.Face(hb.Blob.from_file_path(path)))

    def shape(self, text: str, tracking: float = 0.0) -> tuple[list[tuple[str, float, float]], float]:
        """[(glyph name, x, y)] in font units and the total advance."""
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        hb.shape(self.hb, buf)
        x = 0.0
        out = []
        for info, pos in zip(buf.glyph_infos, buf.glyph_positions):
            out.append((self.hb.glyph_to_string(info.codepoint), x + pos.x_offset, pos.y_offset))
            x += pos.x_advance + tracking * self.upem
        return out, x

    def path(self, text: str, size: float, x0: float = 0.0, y0: float = 0.0, tracking: float = 0.0) -> tuple[str, float]:
        """One SVG path (y down) for the text set at `size` px with its
        baseline at (x0, y0); returns (d, width in px)."""
        placed, adv = self.shape(text, tracking)
        k = size / self.upem
        pen = SVGPathPen(self.glyphs, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
        for name, gx, gy in placed:
            t = TransformPen(pen, (k, 0, 0, -k, x0 + gx * k, y0 - gy * k))
            self.glyphs[name].draw(t)
        return pen.getCommands(), adv * k
