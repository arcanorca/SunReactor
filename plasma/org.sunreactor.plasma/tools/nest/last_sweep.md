# Stall sweep

One nested session per built-in theme (`tools/nest/sweep.py`), default font, no stored
size, bottom panel. Each session: open, Brightness -> Weather -> Brightness -> Weather,
close, reopen. STALL = plasmashell's main thread above 90% CPU for more than 6 s in the
12 s after the popup shows, or no daemon status poll 1-11 s after it shows. Heights are
the popup's window height in px at open / weather / back / reopen.

| theme | verdict | open | weather | back | reopen | main-thread busy | status polls 1-11 s | checks |
|---|---|---:|---:|---:|---:|---:|---:|---|
| amber | OK | 608 | 360 | 608 | 608 | 0 s | 9 | ok |
| amiga | OK | 501 | 335 | 501 | 501 | 0 s | 9 | ok |
| ayudark | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| ayumirage | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| braun | OK | 608 | 360 | 608 | 608 | 0 s | 9 | ok |
| casiodigital | OK | 589 | 411 | 589 | 589 | 0 s | 9 | ok |
| catppuccinmocha | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| classicmacintosh | OK | 547 | 379 | 547 | 547 | 0 s | 9 | ok |
| commodore64 | OK | 473 | 313 | 473 | 473 | 0 s | 9 | ok |
| cyberpunk | OK | 563 | 327 | 563 | 563 | 0 s | 9 | ok |
| dracula | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| everforest | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| grayscale | OK | 608 | 360 | 608 | 608 | 0 s | 9 | ok |
| gruvbox | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| hackergreen | OK | 515 | 413 | 515 | 515 | 0 s | 9 | ok |
| handheld | OK | 547 | 379 | 547 | 547 | 0 s | 9 | ok |
| kanagawa | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| macos | OK | 487 | 319 | 487 | 487 | 0 s | 9 | ok |
| materialocean | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| monokai | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| nds | OK | 591 | 436 | 591 | 591 | 0 s | 9 | ok |
| nightowl | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| nixie | OK | 608 | 359 | 608 | 608 | 0 s | 9 | ok |
| nord | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| nothing | OK | 608 | 369 | 608 | 608 | 0 s | 9 | ok |
| onedark | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| oscilloscope | OK | 608 | 359 | 608 | 608 | 0 s | 9 | ok |
| phosphorblue | OK | 608 | 360 | 608 | 608 | 0 s | 9 | ok |
| ps3 | OK | 512 | 355 | 512 | 512 | 0 s | 9 | ok |
| rosepine | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| solarizeddark | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| synthwave84 | OK | 563 | 331 | 563 | 563 | 0 s | 11 | ok |
| terminal | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| thinkpad | OK | 608 | 359 | 608 | 608 | 0 s | 9 | ok |
| tokyonight | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| unixworkstation | OK | 608 | 360 | 608 | 608 | 0 s | 9 | ok |
| vfd_hifi | OK | 608 | 360 | 608 | 608 | 0 s | 9 | ok |
| wii | OK | 550 | 360 | 550 | 550 | 0 s | 9 | ok |
| winxp | OK | 511 | 344 | 511 | 511 | 0 s | 9 | ok |
| zenburn | OK | 498 | 413 | 498 | 498 | 0 s | 9 | ok |
| edge_noshades | OK | 529 | 361 | 529 | 529 | 0 s | 9 | ok |
| edge_partial | OK | 608 | 260 | 608 | 608 | 0 s | 9 | ok |

Totals: OK 42.

Notes

- The handheld stall was a feedback loop in the page layout, fixed in
  `FullRepresentation.qml`: a page's column took its width from the scroll view's
  available width, so a scrollbar that appeared narrowed the text; the taller
  three-line status line then made the page scroll again, and the width toggled
  every pass (main thread busy in `Repeater::setModel`). The column now keeps one
  grid unit for the scrollbar whether or not the page scrolls.
- Sessions with a stored popup size are not part of this sweep (see `check.py`).
