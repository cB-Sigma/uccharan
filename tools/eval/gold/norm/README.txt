Gold sets for normalise-only mode (uccharan.normalize).

One file per language: <lang>.tsv with columns
  split   dev | heldout (fixed when the set was written, before any tuning)
  class   cardinal, decimal, ordinal, year, money, percent, time, date, phone,
          range, fraction, measure, abbreviation, acronym, url, email, symbol,
          latin, plain, scale
  input   written text
  expected  spoken form; several accepted forms separated by " || "
  options   optional key=value pairs separated by ";" (normalize options)

Comparison folds case, punctuation, hyphens, the nukta sign and
chandrabindu/anusvara before matching (see tools/eval/norm_eval.py).
Never tune on the heldout rows.

te, kn, ml, gu, pa, or and ur, and the rows after the tenth pair of bn, mr, ne
and ta, were written by hand when the written number words were added (the
split column was fixed then, before the first run).
