/** Languages with a front end. `en-gb` is Received Pronunciation. */
export type Lang = "en-us" | "en-gb" | "hi";
/** Output profiles; see docs/profiles.md. */
export type Profile = "espeak-ipa" | "piper" | "kokoro" | "kitten" | "matcha" | "sherpa-onnx";
/** How numbers and symbols are read. */
export type NormalizeMode = "natural" | "espeak" | "off";

export interface PhonemizeOptions {
  /** Language of the text (default `en-us`). espeak voice names such as `en-gb-x-rp` are accepted. */
  lang?: Lang | string;
  /** Output profile (default `espeak-ipa`). */
  profile?: Profile;
  /** Number and symbol reading (default `natural`). */
  normalize?: NormalizeMode;
  /** Cleaner, not reference-compatible output. */
  improved?: boolean;
  /** Detect romanised Hindi (experimental; `lang: "hi"` only). */
  hinglish?: boolean;
  /** Keep clause punctuation in `espeak-ipa` output (default true). */
  punctuation?: boolean;
  /** Bracket foreign-language words with `(en)`/`(hi)` markers in `espeak-ipa` output. */
  languageFlags?: boolean;
  /** Pronunciation word lists (TSV, PLS or JSON text), highest priority first. */
  wordlists?: string[];
}

export interface InitOptions {
  /** WebAssembly binary or its URL. Defaults to the file shipped in the package. */
  wasm?: BufferSource | URL | string;
  /** Directory URL that holds `<lang>.ucd`. Defaults to the package's `data/` directory. */
  dataBaseUrl?: URL | string;
}

export const LANGUAGES: readonly Lang[];
export const PROFILES: readonly Profile[];

/** Initialise the WebAssembly module (implicit in the async functions). */
export function init(options?: InitOptions): Promise<void>;
/** Load a language's data, from the data directory or from the given bytes. */
export function loadLanguage(lang: Lang | string, bytes?: BufferSource): Promise<void>;
/** Fetch the compiled Kokoro English data for a language (done by `phonemize` for the `kokoro` profile). */
export function loadKokoro(lang?: "en-us" | "en-gb" | string): Promise<void>;
/** Load misaki's English dictionaries from JSON (text or parsed objects) instead of the compiled copy. */
export function loadMisaki(lang: "en-us" | "en-gb" | string, gold: string | object, silver: string | object): Promise<void>;
/** True when misaki's dictionaries are loaded for the language. */
export function hasMisaki(lang: Lang | string): boolean;
/** True when the language's data is loaded. */
export function isLoaded(lang: Lang | string): boolean;
/** Languages whose data a request needs (code-mixed text needs two). */
export function languagesFor(text: string, lang?: Lang | string): Lang[];

/** Convert text to phonemes, loading language data on first use. */
export function phonemize(text: string, options: PhonemizeOptions & { profile: "piper" }): Promise<string[]>;
export function phonemize(text: string, options: PhonemizeOptions & { profile: "sherpa-onnx" }): Promise<number[][]>;
export function phonemize(text: string, options?: PhonemizeOptions): Promise<string>;

/** Synchronous variant; throws unless the needed languages are loaded. */
export function phonemizeSync(text: string, options: PhonemizeOptions & { profile: "piper" }): string[];
export function phonemizeSync(text: string, options: PhonemizeOptions & { profile: "sherpa-onnx" }): number[][];
export function phonemizeSync(text: string, options?: PhonemizeOptions): string;

/** Piper phoneme ids for one sentence string and a voice config (`*.onnx.json`, parsed). */
export function piperIds(
  sentence: string,
  voiceConfig: { phoneme_id_map: Record<string, number[]> } | Record<string, number[]>,
): { ids: number[]; missing: string[] };
/** Kokoro-82M input ids `[0, …, 0]`. */
export function kokoroIds(tokens: string): number[];
/** Kitten TTS 0.8 input ids `[0, …, 10, 0]`. */
export function kittenIds(phonemes: string): number[];
/** Matcha-TTS input ids (interspersed with the blank id 0). */
export function matchaIds(phonemes: string): number[];
/** Library version. */
export function version(): Promise<string>;

/** Load the optional CC BY-SA 4.0 WikiPron pack of a language (improved mode only). */
export function loadWikipron(lang: string, bytes?: Uint8Array | ArrayBuffer): Promise<void>;

export interface NormalizeOptions {
  /** Language of the text (default `en-us`). */
  lang?: Lang | string;
  /** Keep known brand names whole. */
  improved?: boolean;
  /** Which language reads digits: `context` (default), `request` or a language tag. */
  digits?: string;
  /** English large numbers: `auto` (default), `indian` (lakh, crore) or `international`. */
  enScale?: "auto" | "indian" | "international";
  /** Latin-script words inside other scripts: `keep` (default), `transliterate` or `spell`. */
  latin?: "keep" | "transliterate" | "spell";
  /** URLs and email addresses: `read` (default), `keep` or `drop`. */
  urls?: "read" | "keep" | "drop";
  /** Respelling style for word-list entries with IPA only. */
  respell?: "native" | "latin-hyphen" | "latin-plain";
  /** Word lists (TSV, PLS or JSON text), highest priority first. */
  wordlists?: string[];
}

export interface Offsets {
  byte: [number, number];
  char: [number, number];
  utf16: [number, number];
}

export interface NormSpan {
  kind: string;
  lang: string;
  /** UTF-16 offsets into the input. */
  start: number;
  end: number;
  /** UTF-16 offsets into the output text. */
  outStart: number;
  outEnd: number;
  text: string;
  spoken: string;
  src: Offsets;
  out: Offsets;
  /** `[list, entry]` for word-list spans. */
  entry: [number, number] | null;
  value: string | null;
}

export interface Normalized {
  text: string;
  sentences: { start: number; end: number }[];
  spans: NormSpan[];
}

/** Spoken form of `text` for a voice that reads text, with typed spans. */
export function normalize(text: string, options?: NormalizeOptions): Promise<Normalized>;
/** Synchronous variant; the needed languages must already be loaded. */
export function normalizeSync(text: string, options?: NormalizeOptions): Normalized;
/** Respell IPA for a text voice reading `lang`; null when the language has no writer. */
export function respell(
  ipa: string,
  options?: { lang?: Lang | string; source?: string; style?: "native" | "latin-hyphen" | "latin-plain" },
): Promise<{ text: string; readback: string; distance: number } | null>;
/** Convert a word list between TSV, JSON and PLS. */
export function convertWordlist(text: string, to: "tsv" | "json" | "pls", lang?: string): Promise<string>;
