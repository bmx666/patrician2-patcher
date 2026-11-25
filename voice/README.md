# Tutorial Voice-over: Do It Yourself

Record or synthesize the tutorial speech for any translated language. The game picks up the files
at run time, so you don't need to rebuild or re-run the patcher.

| Path | |
|---|---|
| `text/<code>/<chapter>_<screen>.txt` | text to speak on one tutorial screen, 160 files per language |
| `say.sh` | macOS: renders the texts with the system voices (`say`) |
| `siri.sh` | macOS: renders the texts with Siri voices ([siri-say](https://github.com/marcomontalbano/siri-say)) |

The texts are the spoken version of the tutorial: headings are left out and numbers are written as
words. Read them as they are.

## Where the files go

```text
<game>/Musik/tutorial_<code>/<chapter>_<screen><suffix>.mp3   (or .aac)
```

| Language | `<code>` | `<suffix>` | Example |
|---|---|---|---|
| Danish | `dan` | `da` | `Musik/tutorial_dan/1_1da.mp3` |
| Estonian | `est` | `et` | `Musik/tutorial_est/1_1et.mp3` |
| Finnish | `fin` | `fi` | `Musik/tutorial_fin/1_1fi.mp3` |
| German | `deu` | `de` | `Musik/tutorial_deu/1_1de.mp3` |
| Latvian | `lav` | `lv` | `Musik/tutorial_lav/1_1lv.mp3` |
| Lithuanian | `lit` | `lt` | `Musik/tutorial_lit/1_1lt.mp3` |
| Norwegian | `nor` | `nb` | `Musik/tutorial_nor/1_1nb.mp3` |
| Russian | `rus` | `ru` | `Musik/tutorial_rus/1_1ru.mp3` |
| Swedish | `swe` | `sv` | `Musik/tutorial_swe/1_1sv.mp3` |

A screen without a file (or with a misnamed one) is silent. `.mp3` and `.aac` can be mixed.

## Install ffmpeg (includes ffprobe)

- macOS: `brew install ffmpeg`
- Windows: `winget install Gyan.FFmpeg`
- Linux: `sudo apt install ffmpeg`

## macOS: system voices (`say.sh`)

1. Download voices: *System Settings › Accessibility › Spoken Content › System Voice › Manage Voices*
   (*Premium* / *Enhanced* sound best).
2. Listen to a sample (screens `1_1`–`1_3`, every installed voice of the language):
   ```sh
   cd voice
   LANGS=est ./say.sh
   ```
3. Render all screens with the chosen voice:
   ```sh
   LANGS=est VOICES="Anna (Premium)" ./say.sh all
   ```
   Options: `RATE=160` (words per minute, default 175), `AAC=1` (also `aac/…`), screen list
   instead of `all` (`./say.sh 2_5 3_1`).
4. Copy the folder `mp3/est/Anna_-Premium-` to `<game>/Musik/` and rename it to `tutorial_est`.

## macOS: Siri voices (`siri.sh`)

1. Get siri-say (needs macOS 13+ and `xcode-select --install`):
   ```sh
   git clone https://github.com/marcomontalbano/siri-say
   ```
2. Download the Siri voice in *Manage Voices* (see above) and find its identifier:
   ```sh
   siri-say/Sources/siri-say/main.swift --list sv
   ```
3. Sample, then render:
   ```sh
   export SIRI_SAY="$PWD/siri-say/Sources/siri-say/main.swift"
   cd voice
   LANGS=swe ./siri.sh
   LANGS=swe VOICES="<identifier>" ./siri.sh all
   ```
   Options: `RATE=0.45` (0.0–1.0, default 0.48), `AAC=1`, screen list.
   Run `main.swift` directly: a binary from `swift build` does not see Siri voices.
4. Copy the folder `mp3/swe/<voice>` to `<game>/Musik/` and rename it to `tutorial_swe`.

## Any audio file → game format (ffmpeg)

MP3:

```sh
ffmpeg -i in.wav -ar 44100 -ac 1 -c:a libmp3lame -b:a 128k -write_xing 0 -id3v2_version 0 -map_metadata -1 1_1et.mp3
```

AAC (smaller, but needs a `TLEN` tag with the length in ms, otherwise the game gets the length wrong):

```sh
ffmpeg -i in.wav -ar 44100 -ac 1 -c:a aac -b:a 64k -map_metadata -1 -f adts tmp.aac
ffprobe -v error -count_packets -show_entries stream=nb_read_packets -of csv=p=0 tmp.aac   # N
ffmpeg -i tmp.aac -c copy -map_metadata -1 -metadata TLEN=<N*1024*1000/44100> -fflags +bitexact -write_id3v2 1 -f adts 1_1et.aac
```

To cut silence at both ends like the scripts do, add to the first command:
`-af "silenceremove=start_periods=1:start_threshold=0.0061:start_silence=0.1,areverse,silenceremove=start_periods=1:start_threshold=0.0061:start_silence=0.1,areverse"`
