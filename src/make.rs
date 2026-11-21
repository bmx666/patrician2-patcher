//! `make` command: builds the patch file from the original game folder and the patched files.

use crate::{
    diff::Dictionary,
    format::{self, FileEntry, Header, OP_END},
    join_rel, list_files, read_file, Result,
};
use flate2::{write::ZlibEncoder, Compression};
use std::{fs, io::Write, path::PathBuf, slice::Iter};

/// Original GOG UK v1.1 files the patch is computed against (dictionary order).
pub const SOURCES: &[&str] = &[
    "Patrician 2.exe",
    "AIM.dll",
    "ddraw_dll.dll",
    "p2arch0_eng.cpr",
    "p2arch1_eng.cpr",
];

fn value<'a>(it: &mut Iter<'a, String>, opt: &str) -> Result<&'a String> {
    it.next().ok_or_else(|| format!("{opt} needs a value"))
}

pub fn run(args: &[String]) -> Result<()> {
    let mut orig: Option<PathBuf> = None;
    let mut new: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut version = env!("CARGO_PKG_VERSION").to_string();
    let mut excludes: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--orig" => orig = Some(PathBuf::from(value(&mut it, a)?)),
            "--new" => new = Some(PathBuf::from(value(&mut it, a)?)),
            "--out" => out = Some(PathBuf::from(value(&mut it, a)?)),
            "--version" => version = value(&mut it, a)?.clone(),
            "--exclude" => {
                let ex = value(&mut it, a)?.replace('\\', "/");
                excludes.push(ex.trim_matches('/').to_string());
            }
            other => return Err(format!("make: unknown argument {other}")),
        }
    }
    let orig = orig.ok_or("make: --orig DIR is required")?;
    let new = new.ok_or("make: --new DIR is required")?;
    let out = out.ok_or("make: --out FILE is required")?;

    let mut dict = Dictionary::new();
    let mut sources = Vec::new();
    for name in SOURCES {
        let bytes = read_file(&join_rel(&orig, name))?;
        sources.push(FileEntry::of(name, &bytes));
        dict.append(&bytes)?;
    }

    let mut used = vec![false; excludes.len()];
    let names: Vec<String> = list_files(&new)?
        .into_iter()
        .filter(|n| !n.split('/').any(|c| c.starts_with('.')))
        .filter(|n| {
            let hit = excludes
                .iter()
                .position(|ex| n == ex || n.starts_with(&format!("{ex}/")));
            if let Some(k) = hit {
                used[k] = true;
            }
            hit.is_none()
        })
        .collect();
    for (ex, used) in excludes.iter().zip(&used) {
        if !used {
            println!("WARNING: --exclude {ex} matched no file");
        }
    }
    if names.is_empty() {
        return Err(format!("make: no files in {}", new.display()));
    }

    let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
    let mut targets = Vec::new();
    for name in &names {
        format::validate_name(name)?;
        let bytes = read_file(&join_rel(&new, name))?;
        let st = dict
            .encode(&bytes, &mut enc)
            .and_then(|st| enc.write_all(&[OP_END]).map(|()| st))
            .map_err(|e| format!("{name}: {e}"))?;
        println!(
            "  {name}: {} bytes, {} copied, {} added",
            bytes.len(),
            st.copied,
            st.added
        );
        targets.push(FileEntry::of(name, &bytes));
        dict.append(&bytes)?;
    }
    let body = enc.finish().map_err(|e| format!("compress: {e}"))?;

    let header = Header {
        version,
        sources,
        targets,
    };
    let mut patch = Vec::new();
    header
        .write(&mut patch)
        .map_err(|e| format!("write header: {e}"))?;
    patch.extend_from_slice(&body);
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(&out, &patch).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "Patch {} ({} files, version {}): {} bytes",
        out.display(),
        header.targets.len(),
        header.version,
        patch.len()
    );
    Ok(())
}
