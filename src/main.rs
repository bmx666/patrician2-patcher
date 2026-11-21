//! Patrician 2 patcher: applies the embedded patch to an original GOG UK v1.1 game folder.
//!
//! - Every source file is verified by size + SHA-256 before anything is written.
//! - Files that get overwritten are first saved to `BACKUP_DIR` (once: the first copy is kept).
//! - Re-running uses the saved originals, so a newer patch applies over an older one.
//! - `--restore` puts the saved originals back and removes files the patch added.

mod diff;
mod format;
mod make;

use flate2::read::ZlibDecoder;
use format::{FileEntry, Header};
use std::{
    env, fs,
    io::{self, BufReader, Cursor},
    path::{Path, PathBuf},
    process::ExitCode,
};

static EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded.p2d"));

const BACKUP_DIR: &str = "patrician2-patcher-backup";
const GAME_EXE: &str = "Patrician 2.exe";

pub type Result<T> = std::result::Result<T, String>;

enum Mode {
    Apply,
    Restore,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    // Started by double-click: keep the console window open at the end.
    let interactive = args.is_empty();
    let code = match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ERROR: {e}");
            ExitCode::FAILURE
        }
    };
    if interactive && cfg!(windows) {
        println!();
        println!("Press Enter to close this window.");
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
    }
    code
}

fn print_usage() {
    println!(
        "Usage:
  patrician2-patcher [GAME_DIR]             apply the patch
  patrician2-patcher --restore [GAME_DIR]   restore the original game files
  patrician2-patcher --patch FILE [GAME_DIR] use a patch file instead of the embedded one
  patrician2-patcher make --orig DIR --new DIR --out FILE [--version V] [--exclude PATH]...

Without GAME_DIR the folder of the patcher, then the current folder is used."
    );
}

fn run(args: &[String]) -> Result<()> {
    if args.first().map(String::as_str) == Some("make") {
        return make::run(&args[1..]);
    }
    let mut mode = Mode::Apply;
    let mut patch_file: Option<PathBuf> = None;
    let mut game: Option<PathBuf> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            "--restore" => mode = Mode::Restore,
            "--patch" => {
                let file = it.next().ok_or("--patch needs a file")?;
                patch_file = Some(PathBuf::from(file));
            }
            s if s.starts_with('-') => return Err(format!("unknown option {s} (see --help)")),
            s => {
                if game.is_some() {
                    return Err("more than one game folder given".into());
                }
                game = Some(PathBuf::from(s));
            }
        }
    }

    let owned;
    let data: &[u8] = match &patch_file {
        Some(p) => {
            owned = read_file(p)?;
            &owned
        }
        None => EMBEDDED,
    };
    if data.is_empty() {
        return Err("this build has no embedded patch; use --patch FILE".into());
    }
    let game = locate_game(game)?;
    let mut cur = Cursor::new(data);
    let header = Header::read(&mut cur).map_err(|e| format!("invalid patch: {e}"))?;
    println!(
        "Patrician 2 patcher {}, patch {}",
        env!("CARGO_PKG_VERSION"),
        header.version
    );
    println!("Game folder: {}", game.display());
    match mode {
        Mode::Apply => apply(&game, &header, cur),
        Mode::Restore => restore(&game, &header),
    }
}

fn locate_game(arg: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = arg {
        if p.join(GAME_EXE).is_file() {
            return Ok(p);
        }
        return Err(format!("{GAME_EXE} not found in {}", p.display()));
    }
    let mut candidates = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
        }
    }
    if let Ok(dir) = env::current_dir() {
        candidates.push(dir);
    }
    candidates
        .into_iter()
        .find(|d| d.join(GAME_EXE).is_file())
        .ok_or_else(|| {
            format!(
                "{GAME_EXE} not found. Put the patcher into the game folder \
                 or pass the game folder as an argument."
            )
        })
}

fn apply(game: &Path, header: &Header, body: Cursor<&[u8]>) -> Result<()> {
    let backup = game.join(BACKUP_DIR);
    let total: u64 = header.sources.iter().chain(&header.targets).map(|e| e.size).sum();
    let mut dict = Vec::with_capacity(usize::try_from(total).map_err(|_| "patch too large")?);
    for s in &header.sources {
        let target = header.targets.iter().find(|t| t.name == s.name);
        let bytes = load_original(game, &backup, s, target)?;
        dict.extend_from_slice(&bytes);
    }

    // Build and verify every file in memory before writing anything.
    let mut dec = BufReader::new(ZlibDecoder::new(body));
    let mut starts = Vec::with_capacity(header.targets.len());
    for t in &header.targets {
        let start = dict.len();
        format::decode_target(&mut dec, &mut dict, t.size)
            .map_err(|e| format!("{}: corrupt patch data: {e}", t.name))?;
        if format::sha256(&dict[start..]) != t.sha256 {
            return Err(format!("{}: patched result has a wrong checksum", t.name));
        }
        starts.push(start);
    }

    let (mut written, mut skipped) = (0, 0);
    for (t, &start) in header.targets.iter().zip(&starts) {
        let data = &dict[start..start + t.size as usize];
        let path = join_rel(game, &t.name);
        match fs::read(&path) {
            Ok(current) => {
                if t.matches(&current) {
                    println!("  up to date  {}", t.name);
                    skipped += 1;
                    continue;
                }
                let saved = join_rel(&backup, &t.name);
                if !saved.exists() {
                    write_file(&saved, &current)?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
        write_file(&path, data)?;
        println!("  patched     {}", t.name);
        written += 1;
    }
    println!("Done: {written} file(s) written, {skipped} already up to date.");
    if backup.is_dir() {
        println!("Original files are kept in {BACKUP_DIR}/. Run with --restore to undo.");
    }
    Ok(())
}

/// Original bytes of a source: the backup copy if present, else the game file.
fn load_original(
    game: &Path,
    backup: &Path,
    s: &FileEntry,
    target: Option<&FileEntry>,
) -> Result<Vec<u8>> {
    let saved = join_rel(backup, &s.name);
    if saved.is_file() {
        let bytes = read_file(&saved)?;
        if s.matches(&bytes) {
            return Ok(bytes);
        }
        return Err(format!(
            "{}: the backup copy does not match the original GOG UK v1.1 file",
            saved.display()
        ));
    }
    let path = join_rel(game, &s.name);
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(format!(
                "{} is missing. Patrician 2 GOG UK v1.1 is required.",
                s.name
            ))
        }
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if s.matches(&bytes) {
        return Ok(bytes);
    }
    if target.is_some_and(|t| t.matches(&bytes)) {
        return Err(format!(
            "{} is already patched, but its original copy is missing in {BACKUP_DIR}/. \
             Restore the original game files (reinstall or verify them in GOG Galaxy) \
             and run the patcher again.",
            s.name
        ));
    }
    Err(format!(
        "{} does not match Patrician 2 GOG UK v1.1. Only this version is supported.",
        s.name
    ))
}

fn restore(game: &Path, header: &Header) -> Result<()> {
    let backup = game.join(BACKUP_DIR);
    if !backup.is_dir() {
        return Err(format!("{BACKUP_DIR}/ not found: nothing to restore"));
    }
    let saved = list_files(&backup)?;
    for rel in &saved {
        let bytes = read_file(&join_rel(&backup, rel))?;
        write_file(&join_rel(game, rel), &bytes)?;
        println!("  restored    {rel}");
    }
    for t in header.targets.iter().filter(|t| !saved.contains(&t.name)) {
        let path = join_rel(game, &t.name);
        if !path.is_file() {
            continue;
        }
        if t.matches(&read_file(&path)?) {
            fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            println!("  removed     {}", t.name);
        } else {
            println!("  kept        {} (not written by this patch version)", t.name);
        }
    }
    fs::remove_dir_all(&backup).map_err(|e| format!("{}: {e}", backup.display()))?;
    println!("Done: original game files restored.");
    Ok(())
}

pub(crate) fn join_rel(base: &Path, name: &str) -> PathBuf {
    name.split('/').fold(base.to_path_buf(), |p, c| p.join(c))
}

pub(crate) fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes via a temporary file in the same folder, then renames over the target.
fn write_file(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".p2tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, data).map_err(|e| format!("{}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("{}: {e}", path.display())
    })
}

/// All regular files under `root`, as sorted relative paths with `/` separators.
pub(crate) fn list_files(root: &Path) -> Result<Vec<String>> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) -> Result<()> {
        let mut entries = fs::read_dir(dir)
            .and_then(|rd| rd.collect::<io::Result<Vec<_>>>())
            .map_err(|e| format!("{}: {e}", dir.display()))?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e
                .file_name()
                .into_string()
                .map_err(|n| format!("non-UTF-8 file name {n:?}"))?;
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let ft = e.file_type().map_err(|err| format!("{rel}: {err}"))?;
            if ft.is_dir() {
                walk(&e.path(), &rel, out)?;
            } else if ft.is_file() {
                out.push(rel);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, "", &mut out)?;
    Ok(out)
}
