# Patrician 2 — Localization & Fix Patch

An unofficial patcher for **Patrician 2** (Ascaron, 2001), GOG UK version 1.1.

The goal is to make the game playable in many languages. One patched game holds every
supported language, you switch between them right from the main menu, and new languages can be
added through a simple translation workflow. Alongside the translations, the patch adds
widescreen support, fixes bugs in the original game, brings small quality-of-life features, and
makes the game run smoothly on Apple Silicon Macs under Wine.

This repository contains only the patcher. It holds no original game files, texts or images:
the patch is a binary difference that is applied to your own copy of the game. The only texts
here are this project's translations of the tutorial speech in [`voice/text/`](voice/), for people
who want to [voice the tutorial themselves](#make-your-own-voice-over).

## Installation

1. Install Patrician 2 (GOG UK v1.1).
2. Download `patrician2-patcher.exe` from [Releases](../../releases).
3. Copy it into the game folder, next to `Patrician 2.exe`, and run it.
   - Windows: double-click it.
   - macOS / Linux (Wine): in the game folder run `wine patrician2-patcher.exe`.
   - The game folder can also be passed as an argument:
     `patrician2-patcher.exe "C:\GOG Games\Patrician 2"`.
4. Start the game and pick a language with the flag selector in the top-left corner of the main menu.

The patcher checks that every game file it needs is the original GOG UK v1.1 version and builds
all patched files in memory before writing anything. Original files that get replaced are saved
to `patrician2-patcher-backup/` in the game folder.

Running the patcher again is safe: files that are already up to date are skipped, and a newer
patcher version is applied on top of the saved originals.

To remove the patch and get the original game back:

```
patrician2-patcher.exe --restore
```

### Files written by the patcher

| File | |
|---|---|
| `Patrician 2.exe` | patched game |
| `ddraw_dll.dll`, `AIM.dll` | rebuilt graphics and image libraries (smooth under Rosetta 2) |
| `Win32/Mp3dec.asi` | new MP3 music decoder |
| `Win32/Aacdec.asi` | AAC decoder for the tutorial voice packs |
| `p2lang_<code>.cpr` | one archive per language (texts, fonts, pictures, widescreen data) |

The original archives `p2arch0_eng.cpr` and `p2arch1_eng.cpr` are never modified.

## cnc-ddraw (optional)

[cnc-ddraw](https://github.com/FunkyFr3sh/cnc-ddraw) runs the game in a window or borderless
full screen and works well under Wine (e.g. in GameHub). It is not part of this patch.

1. Download `cnc-ddraw.zip` of [release 7.1](https://github.com/FunkyFr3sh/cnc-ddraw/releases).
2. Extract `ddraw.dll`, `ddraw.ini` and the `Shaders` folder into the game folder. This replaces
   the GOG `ddraw.dll`; keep a copy of it if you want to go back.
3. Add this section to the end of `ddraw.ini` (the section name is the name of the game
   executable without `.exe`):

```ini
[Patrician2]
windowed=true
fullscreen=true
border=false
width=0
height=0
maintas=true
adjmouse=true
renderer=opengl
singlecpu=false
```

## Languages

| Language | Texts | Tutorial voice-over |
|---|---|---|
| English | original | original |
| Danish | yes | optional voice pack |
| Estonian | yes | optional voice pack |
| Finnish | yes | optional voice pack |
| German | yes | optional voice pack |
| Latvian | yes | optional voice pack |
| Lithuanian | yes | optional voice pack |
| Norwegian | yes | optional voice pack |
| Russian | yes | optional voice pack |
| Swedish | yes | optional voice pack |

Everything is translated: menus, the game interface, letters and messages, the tutorial,
mission scripts, credits and the main menu picture. Russian also translates ship and person names.

## Tutorial Voice-over (optional)

The English tutorial uses the original voice. Voice packs for the other languages are separate
downloads and are not part of this patch. Install the voice pack of a language to hear the
tutorial speech in that language. Without a voice file the tutorial screen stays silent (no
English fallback).

Extract a voice pack into the game folder, so that its files end up here:

| Language | Folder | Files |
|---|---|---|
| Danish | `Musik/tutorial_dan/` | `1_1da.aac` … (160 files) |
| Estonian | `Musik/tutorial_est/` | `1_1et.aac` … (160 files) |
| Finnish | `Musik/tutorial_fin/` | `1_1fi.aac` … (160 files) |
| German | `Musik/tutorial_deu/` | `1_1de.aac` … (160 files) |
| Latvian | `Musik/tutorial_lav/` | `1_1lv.aac` … (160 files) |
| Lithuanian | `Musik/tutorial_lit/` | `1_1lt.aac` … (160 files) |
| Norwegian | `Musik/tutorial_nor/` | `1_1nb.aac` … (160 files) |
| Russian | `Musik/tutorial_rus/` | `1_1ru.aac` … (160 files) |
| Swedish | `Musik/tutorial_swe/` | `1_1sv.aac` … (160 files) |

Folder = `tutorial_<ISO 639-2 language code>`, suffix = ISO 639-1 language code.
A file name is `<chapter>_<screen><suffix>.aac` or `.mp3`, with the same chapter and screen
numbers as the original `Musik/Tutorial/<chapter>_<screen>uk.mp3`. The game looks for the file at
run time: whichever of `.aac` / `.mp3` exists is played, so both formats can be mixed in one folder
and files can be added or replaced at any time. If both exist for a screen, the one named in the
tutorial script is played.

File formats the game can play:

- **AAC** (voice packs): ADTS stream (no MP4/M4A container), AAC-LC, mono, 44.1 kHz, 64 kbps.
  Every file starts with an ID3v2.4 tag holding `TLEN` (the length in milliseconds); without it
  the game computes a wrong length for the speech. Decoded by `Win32/Aacdec.asi`.
- **MP3**: MPEG-1 Layer III, 128 kbps CBR, 44.1 kHz, mono, without ID3 or Xing headers: the same
  format as the original tutorial files. Decoded by `Win32/Mp3dec.asi`.

### Make your own voice-over

The spoken text of every tutorial screen is in [`voice/text/<code>/`](voice/text/), one file per
screen (`1_1.txt` → `Musik/tutorial_<code>/1_1<suffix>.mp3`). Record it yourself or use a TTS engine.
[`voice/README.md`](voice/README.md) explains:

- macOS: generate the whole language with the system voices (`voice/say.sh`) or Siri voices
  (`voice/siri.sh`), then copy the output folder into `Musik/` as `tutorial_<code>`;
- converting any audio file to a game-ready MP3 or AAC with ffmpeg.

## New Features

- **One game, ten languages.** A single patched game runs in any supported language.
- **Language selector on the main menu.** Flip through the flags with the arrows and confirm.
  The game then restarts by itself in the new language.
- **Translated main menu picture.** Menu items and their decorative initial letters are redrawn in
  every language, in the style of the original. German uses Ascaron's original German book page,
  colour-matched to the Patrician 2 picture.
- **Fonts with national letters.** The game fonts gained Cyrillic, Scandinavian, Baltic and other
  letters. Decorative initial letters in letters and texts support them too.
- **Widescreen.** The highest resolution option now offers many modern screen sizes, from 4:3 to
  32:9, up to 4096×2400, including Mac Retina sizes. The game screen, the side panels and the
  fast-travel map adapt to the chosen size. You can switch resolution during a game.
- **Game speed buttons.** A small bar of six buttons below the top bar of the game screen sets the
  game speed with one click. The current speed is shown as pressed.
- **Game speed hotkeys.** `+` / `-` (on the main keyboard or the numpad) make the game one step
  faster or slower.
- **WASD map scrolling.** W/A/S/D scroll the town view and the sea map, just like the arrow keys.
  Typing in text fields is not affected.
- **Correct plurals** for ship types in auction messages, for languages that have several plural
  forms (Russian, Lithuanian, Latvian).
- **More precise ship speed.** The ship panel shows the current speed to a tenth of a knot
  (e.g. 5.5 instead of 5).
- **Readable building windows.** The background behind the text in building windows (church,
  office, smithy, ...) is less transparent, so the text is easier to read.
- **Translated shipyard book.** The ship stats page in the shipyard book is translated into every
  language.
- **Tutorial voice-over** for Russian (chapter 1), Swedish, German, Danish, Norwegian and Finnish
  with the [optional voice packs](#tutorial-voice-over-optional). The new AAC decoder keeps the
  voice files about half the size of MP3.

## Original Game Bugs Fixed

- **Music fading out for minutes.** With a low music volume, switching tracks could leave the old
  track fading for a very long time. The fade now always ends within a few seconds.
- **Building sounds stuck during the tutorial.** Building ambience sounds stopped working after
  you left a building in the tutorial. Fixed.
- **Wrong game speed at campaign start.** When you started a campaign, the game ran at a different
  speed than the one shown in the settings. The selected speed now actually applies.
- **Wrong ship speeds in the shipyard.** The shipyard listed the wrong speed in knots for each ship
  type. It now matches the real game values.
- **Wrong title on the single player screen.** The "New single game" screen showed the title
  "Game settings". It now shows the correct title.
- **Wrong ship stats in the shipyard book.** The stats printed in the shipyard book picture were
  partly wrong. They are redrawn with the real values.

## macOS (Apple Silicon) Fixes

On Apple Silicon the game runs through Rosetta 2, which translates the old x86 code. Some parts
of the game worked very badly with it:

- **Laggy graphics.** The game's graphics library rewrote its own code while drawing, which forces
  Rosetta to re-translate it again and again. The patch ships rebuilt copies of the graphics and
  image libraries that work the same way without rewriting themselves, so the game runs smoothly.
- **Freeze when the music changes.** The original MP3 music decoder uses old processor
  instructions that are very slow under Rosetta, and the game froze on every track change. The
  patch includes a new, modern MP3 decoder, and the music sounds the same.
- **Retina screen sizes** are included in the widescreen presets.
- **Mac keyboards.** Speed hotkeys use `+` / `-`, because Mac keyboards have no Pause key.
  WASD scrolling works as an alternative to the arrow keys.

## Wine Fixes

The patch is developed and tested with the game running under Wine on a Mac:

- Works with **Proton / Game Porting Toolkit (GPTK) / MoltenVK** together with the GOG graphics
  library, and with windowed [cnc-ddraw](#cnc-ddraw-optional) (e.g. in GameHub).
- The new music and voice decoders depend only on the most basic Windows functions, with no
  extra runtime libraries, so they load reliably under Wine.
- The automatic restart after a language change starts a new copy of the game only after the old
  one has fully closed, which also works under Wine.
- The graphics and music fixes above also remove the stutters and freezes seen under Wine on
  Apple Silicon.

## For Developers

The patcher is written in Rust and has no dependencies besides `flate2` and `sha2`.

### Patch format

`data/patrician2.p2d` is embedded into the patcher at build time (`build.rs`). It contains:

- the list of original files with size and SHA-256 (`Patrician 2.exe`, `AIM.dll`,
  `ddraw_dll.dll`, `p2arch0_eng.cpr`, `p2arch1_eng.cpr`);
- the list of output files with size and SHA-256;
- one zlib stream with copy/add operations per output file. Copies refer to the original files
  and to output files built before, so content taken from the original game is never stored in
  the patch.

The release is a single file: the patch is inside `patrician2-patcher.exe`, so
`data/patrician2.p2d` is never copied to the game folder. Build order:

1. Create `data/patrician2.p2d` (native build, see below).
2. Build the Windows patcher. `build.rs` embeds the patch file present at that moment; build
   again after every new `make`.
3. Check: a patcher built without a patch file stops with
   `this build has no embedded patch; use --patch FILE`.

### Creating the patch

The patch is created from the original game folder and the build output of the localization
project (not part of this repository):

```sh
cargo run --release -- make \
  --orig "/path/to/Patrician 2" \
  --new /path/to/build/install \
  --out data/patrician2.p2d \
  --exclude Musik
```

`--exclude` skips files or folders of the build output (`Musik` = tutorial voice packs, released
separately). `--version` sets the version shown by the patcher (default: the crate version).

### Building the patcher

Windows (MSVC), 32-bit, runs on every Windows and under Wine:

```sh
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
```

macOS / Linux, cross build with [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)
(needs [Zig](https://ziglang.org)):

```sh
brew install zig                       # macOS; on Linux use the package manager or ziglang.org
cargo install --locked cargo-zigbuild
rustup target add i686-pc-windows-gnu
cargo zigbuild --release --target i686-pc-windows-gnu
```

macOS / Linux, cross build with MinGW-w64 instead:

```sh
brew install mingw-w64                 # Debian/Ubuntu: apt install gcc-mingw-w64-i686
rustup target add i686-pc-windows-gnu
CARGO_TARGET_I686_PC_WINDOWS_GNU_LINKER=i686-w64-mingw32-gcc \
  cargo build --release --target i686-pc-windows-gnu
```

The patcher is in `target/<target>/release/patrician2-patcher.exe`. A native build
(`cargo build --release`) works too and patches a game folder directly, e.g. a Wine prefix.

A patch file can also be applied without rebuilding (for testing):

```sh
patrician2-patcher --patch data/patrician2.p2d "/path/to/Patrician 2"
```

Checks:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Layout

```text
patrician2-patcher/
├── Cargo.toml
├── build.rs           embeds data/patrician2.p2d
├── .cargo/config.toml static C runtime for MSVC builds
├── data/              patrician2.p2d (generated, not committed)
├── voice/             tutorial speech texts and TTS scripts (not used by the patcher)
└── src/
    ├── main.rs        apply / restore
    ├── make.rs        `make` command
    ├── diff.rs        delta encoder
    └── format.rs      patch container
```

## License

The patcher code is licensed under the [MIT License](LICENSE). Original game content
© Ascaron Entertainment and its successors; it is not included in this repository and is not
covered by this license. This project is not affiliated with Kalypso Media or GOG.

## Credits

- Original game: © Ascaron Entertainment. Not included; you need your own copy.
- Distribution as a patcher follows [patrizier3_german](https://github.com/onethree7/patrizier3_german)
  by onethree7.
- MP3 decoder based on [minimp3](https://github.com/lieff/minimp3) (CC0).
- AAC decoder based on FAAD3 (libfaad) from
  [FreewareAdvancedAudio/faac](https://github.com/FreewareAdvancedAudio/faac)
  (LGPL-2.1-or-later), commit `63f5a05b590b958e263be5a6998de7b8dcfda100`, unmodified.
- Fonts: Alegreya, Literata, Marck Script, Old Standard TT, PT Serif (SIL Open Font License 1.1).
  Used to render the game fonts and pictures.
- [cnc-ddraw](https://github.com/FunkyFr3sh/cnc-ddraw) by FunkyFr3sh (MIT). Not included.
