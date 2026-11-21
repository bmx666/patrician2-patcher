#!/bin/bash
# Patrician 2 tutorial voice-over (Siri voices via siri-say, https://github.com/bmx666/siri-say branch fix-out).
#   ./siri.sh                   -> screens 1_1 1_2 1_3, all installed voices of all languages
#   ./siri.sh 1_1 2_5           -> chosen screens
#   ./siri.sh all               -> all 160 screens
#   LANGS="deu dan" ./siri.sh all                -> only these languages (rus swe deu dan nor fin est lit lav)
#   VOICES="com.apple.ttsbundle.gryphon-neural_Tilde_sv-SE_premium|<id>" ./siri.sh all   -> only these voices
#                                                   (identifiers from: siri-say --list sv)
#   RATE=0.48 (default) 0.0 slowest, 0.5 normal, 1.0 fastest
#   MP3=0                       -> no MP3 (default: mp3/<lang>/<voice>/<id><sfx>.mp3 via ffmpeg)
#   AAC=1 AAC_KBPS=64           -> also aac/<lang>/<voice>/<id><sfx>.aac via ffmpeg + ffprobe (default: none)
#   SIRI_SAY=<siri-say>/Sources/siri-say/main.swift   -> siri-say command (default: siri-say from PATH)
# Output: mp3/<lang>/<voice>/<id><sfx>.mp3 (e.g. mp3/swe/<voice>/1_1sv.mp3), ready for the game: copy the <voice>
# folder to <game>/Musik/ and rename it to tutorial_<lang>. WAV files are kept in wav/.
# <voice> = the identifier without "com.apple.", other characters than A-Z a-z 0-9 _ - replaced by "_".
cd "$(dirname "$0")"
IDS="$*"; [ -z "$IDS" ] && IDS="1_1 1_2 1_3"
[ "$IDS" = all ] && IDS=$(ls text/rus | sed 's/\.txt$//' | sort -t_ -k1,1n -k2,2n | tr '\n' ' ')
RATE="${RATE:-0.48}"
SIRI_SAY="${SIRI_SAY:-siri-say}"
command -v "$SIRI_SAY" >/dev/null || { echo "siri-say not found: set SIRI_SAY=<path to siri-say or main.swift>"; exit 1; }
# MP3: silence cut at both ends (0.1 s kept), MPEG-1 Layer III 44.1 kHz mono 128 kbps CBR, no Xing/ID3.
MP3="${MP3:-1}"
if [ "$MP3" != 0 ] && ! command -v ffmpeg >/dev/null; then echo "ffmpeg not found: WAV only"; MP3=0; fi
TRIM="silenceremove=start_periods=1:start_threshold=0.0061:start_silence=0.1"
to_mp3() {
  [ "$MP3" = 0 ] && return 0
  mkdir -p "$(dirname "$2")"
  ffmpeg -nostdin -y -loglevel error -i "$1" -af "$TRIM,areverse,$TRIM,areverse" -ar 44100 -ac 1 \
    -c:a libmp3lame -b:a 128k -write_xing 0 -id3v2_version 0 -map_metadata -1 "$2"
}
# AAC: same trim, ADTS AAC-LC mono 44.1 kHz, AAC_KBPS kbps, encoder aac_at (Apple) when present, else aac; then an
# ID3v2 TLEN tag (length in ms) is added, which the game needs for the correct length.
AAC="${AAC:-0}"; AAC_KBPS="${AAC_KBPS:-64}"
if [ "$AAC" != 0 ] && ! command -v ffprobe >/dev/null; then echo "ffmpeg/ffprobe not found: no AAC"; AAC=0; fi
AACENC=aac; [ "$AAC" != 0 ] && ffmpeg -hide_banner -encoders 2>/dev/null | grep -q ' aac_at ' && AACENC=aac_at
to_aac() {
  [ "$AAC" = 0 ] && return 0
  mkdir -p "$(dirname "$2")"
  ffmpeg -nostdin -y -loglevel error -i "$1" -af "$TRIM,areverse,$TRIM,areverse" -ar 44100 -ac 1 \
    -c:a "$AACENC" -b:a "${AAC_KBPS}k" -map_metadata -1 -f adts "$2.tmp" || return 1
  n=$(ffprobe -v error -f aac -select_streams a:0 -count_packets -show_entries stream=nb_read_packets -of csv=p=0 "$2.tmp")
  ffmpeg -nostdin -y -loglevel error -f aac -i "$2.tmp" -c copy -map_metadata -1 \
    -metadata TLEN=$(( (n * 1024 * 1000 + 22050) / 44100 )) -fflags +bitexact -write_id3v2 1 -f adts "$2" || return 1
  rm -f "$2.tmp"
}
sfx_of() {
  case "$1" in
    rus) echo ru ;; swe) echo sv ;; deu) echo de ;; dan) echo da ;; nor) echo nb ;;
    fin) echo fi ;; est) echo et ;; lit) echo lt ;; lav) echo lv ;;
  esac
}
lang_of() {
  case "$1" in
    ru_RU) echo rus ;;
    sv_SE) echo swe ;;
    de_DE) echo deu ;;
    da_DK) echo dan ;;
    nb_NO) echo nor ;;
    fi_FI) echo fin ;;
    et_EE) echo est ;;
    lt_LT) echo lit ;;
    lv_LV) echo lav ;;
  esac
}
# siri-say --list rows: "<language>  <identifier>  <name> (<quality>)"; the language column is used, since
# identifiers such as com.apple.siri.natural.Martin carry no locale
VOICELIST=$("$SIRI_SAY" --list | sed -nE 's/^([a-z]{2,3}-[A-Z]{2}) +([^ ]+) .*/\1|\2/p') || exit 1
for row in $VOICELIST; do
  loc=$(echo "${row%%|*}" | tr - _); voice="${row#*|}"
  lang=$(lang_of "$loc"); [ -z "$lang" ] && continue
  if [ -n "$LANGS" ]; then echo " $LANGS " | grep -qF " $lang " || continue; fi
  if [ -n "$VOICES" ]; then echo "|$VOICES|" | grep -qF "|$voice|" || continue; fi
  dir="wav/$lang/$(echo "${voice#com.apple.}" | tr -c 'A-Za-z0-9_\n-' '_')"
  mkdir -p "$dir"; sfx=$(sfx_of "$lang")
  echo "$lang: $voice"
  for id in $IDS; do
    # one line as an argument: siri-say cuts long / multi-line stdin input
    text=$(tr -s '\r\n\t ' ' ' < "text/$lang/$id.txt" | sed 's/^ //; s/ $//')
    "$SIRI_SAY" --voice "$voice" --rate "$RATE" --out "$dir/$id.siri.caf" "$text" || exit 1
    afconvert -f WAVE -d LEI16@44100 -c 1 "$dir/$id.siri.caf" "$dir/$id.wav" || exit 1
    rm -f "$dir/$id.siri.caf"
    to_mp3 "$dir/$id.wav" "mp3/${dir#wav/}/$id$sfx.mp3" || exit 1
    to_aac "$dir/$id.wav" "aac/${dir#wav/}/$id$sfx.aac" || exit 1
  done
done
echo "done: $(find wav -name '*.wav' | wc -l) WAV in $(pwd)/wav, $(find mp3 -name '*.mp3' 2>/dev/null | wc -l) MP3 in $(pwd)/mp3, $(find aac -name '*.aac' 2>/dev/null | wc -l) AAC in $(pwd)/aac"
