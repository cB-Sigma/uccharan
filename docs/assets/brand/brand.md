# uccharan brand assets

uccharan is a sibling of [bolo](https://github.com/cB-Sigma/bolo) and shares its identity: a blue
rounded square with flat white shapes, the same voice bars, the same two typefaces and colours.

## The mark

A letter turning into sound: the Devanagari letter उ (the first letter of उच्चारण,
"pronunciation") followed by three level-meter bars, the same bars that leave the speaker's mouth
in bolo's mark. Where bolo shows *someone speaking*, uccharan shows *writing becoming speech*.

The mark is drawn on a 32-unit grid as flat white shapes with no outlines and always sits in a blue
rounded square (corner radius 7 of 32). The letter is the outline of उ from Hind SemiBold.

There are two drawings:

- `logo-mark.svg`, for 20 px and up: the letter and three bars (short, tall, short).
- `logo-mark-small.svg`, for 16 to 20 px (favicons): a larger letter and two wider bars. At these
  sizes three bars with round ends turn to mush.

How it was chosen: six ideas were drawn and looked at on dark and light at 16, 32, 64 and 256 px
(`tools/brand` builds the sheet):

1. उ + bars — chosen. Reads as "a Devanagari letter, then sound" at every size and is plainly a
   relative of bolo.
2. अ + bars — the first letter of the alphabet says "Hindi" but not "uccharan", and its wider
   shape left no room for the bars below 32 px.
3. Phoneme slashes around the bars, `/ııı/` — the cleanest at 16 px and a good idea (sound written
   between transcription slashes), but it has no letter in it and loses the Indian-language cue.
4. Latin "u" + bars — generic; it could be any audio product.
5. `/उ/` — letter between slashes; three thin shapes of similar weight, cluttered below 64 px.
6. `/उ` + bars — everything at once; unreadable at small sizes.

## The wordmark

"uccharan" and "उच्चारण" side by side on one baseline, separated by a small dot: the Latin in ink,
the Devanagari in the secondary text colour, so the name is read first and its meaning second.

- Latin: Inter SemiBold, tracked in by 3.5 %.
- Devanagari: Hind SemiBold (Indian Type Foundry), set a little smaller (49 against 56 units) so
  that its headline sits just above the Latin x-height.

Both typefaces are licensed under the SIL Open Font License 1.1, which permits this use. The text
was shaped with HarfBuzz and converted to outlines, so the SVGs carry no font and render the same
everywhere. Shaping matters here: उच्चारण contains the conjunct च्च (च + virama + च), which must be
drawn as a half च joined to a full च under one continuous headline, followed by the vowel sign ा.
The outlines were checked for exactly that.

## Files

| File | Use |
| --- | --- |
| `logo-mark.svg` | The mark in its blue tile, for 20 px and up. Works on any background. |
| `logo-mark-small.svg` | The simplified mark for 16 to 20 px. |
| `logo-mark-mono.svg` | Shapes only, `currentColor`. For single-colour contexts. |
| `logo-wordmark-dark.svg` | Mark, "uccharan" and "उच्चारण" in light ink, for **dark** backgrounds. Transparent. |
| `logo-wordmark-light.svg` | The same in dark ink, for **light** backgrounds. Transparent. |
| `banner-dark.png`, `banner-light.png` | README hero, 1600 × 480 at 2× (3200 × 960 px), rounded corners on a transparent background. |
| `social-preview.png` | 1280 × 640. The repository's social preview and the demo's Open Graph image. |
| `button-live-demo.png` | "Try the live demo" button for the README. |
| `favicon/favicon.svg` | Preferred favicon: the small drawing. |
| `favicon/favicon.ico` | 16, 32 and 48 px in one file. |
| `favicon/favicon-16.png`, `-32.png`, `-48.png` | Raster favicons. |
| `favicon/apple-touch-icon.png` | 180 px, full-bleed. |
| `favicon/icon-192.png`, `icon-512.png` | Full-bleed app icons. |

## Colour

The palette is bolo's.

| Role | Light surfaces | Dark surfaces |
| --- | --- | --- |
| Brand blue (tile, primary buttons) | `#1f62b3` | `#256abf` |
| Accent blue (lines, motif) | `#2a78d6` | `#3987e5` |
| Ink | `#0b0b0b` | `#f0efec` |
| Secondary text | `#63625d` | `#a3a29a` |
| Page | `#f6f6f4` | `#0d0d0d` |
| Card | `#fcfcfb` | `#1a1a19` |
| Hairline | `#e1e0d9` | `#2c2c2a` |

## Rebuilding

```sh
pip install fonttools uharfbuzz pillow
python tools/brand/build.py --fonts DIR     # DIR holds the Inter and Hind .ttf files
node tools/brand/render.mjs                 # needs the playwright package and its Chromium
python tools/brand/build.py --fonts DIR --ico
```
