//! Patch container.
//!
//! Layout: `MAGIC`, version string, source list, target list (plain), then one zlib stream with
//! the ops of all targets in list order. Each target is a sequence of `OP_ADD` / `OP_COPY`
//! terminated by `OP_END`. `OP_COPY` positions address the dictionary = all sources concatenated
//! in list order, followed by every target already produced.

use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};

pub const MAGIC: &[u8; 8] = b"P2PATCH\x01";
pub const OP_END: u8 = 0;
pub const OP_ADD: u8 = 1;
pub const OP_COPY: u8 = 2;

const MAX_FILE_SIZE: u64 = 1 << 30;
const MAX_FILES: u64 = 4096;
const MAX_STR: u64 = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

impl FileEntry {
    pub fn of(name: &str, bytes: &[u8]) -> Self {
        Self {
            name: name.to_string(),
            size: bytes.len() as u64,
            sha256: sha256(bytes),
        }
    }

    pub fn matches(&self, bytes: &[u8]) -> bool {
        bytes.len() as u64 == self.size && sha256(bytes) == self.sha256
    }
}

pub struct Header {
    pub version: String,
    pub sources: Vec<FileEntry>,
    pub targets: Vec<FileEntry>,
}

impl Header {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(MAGIC)?;
        write_str(w, &self.version)?;
        for list in [&self.sources, &self.targets] {
            write_varint(w, list.len() as u64)?;
            for e in list {
                write_str(w, &e.name)?;
                write_varint(w, e.size)?;
                w.write_all(&e.sha256)?;
            }
        }
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 8];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(invalid("not a Patrician 2 patch file"));
        }
        let version = read_str(r)?;
        let sources = read_entries(r)?;
        let targets = read_entries(r)?;
        Ok(Self {
            version,
            sources,
            targets,
        })
    }
}

fn read_entries<R: Read>(r: &mut R) -> io::Result<Vec<FileEntry>> {
    let n = read_varint(r)?;
    if n > MAX_FILES {
        return Err(invalid("too many files"));
    }
    let mut list = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let name = read_str(r)?;
        validate_name(&name).map_err(|e| invalid(&e))?;
        let size = read_varint(r)?;
        if size > MAX_FILE_SIZE {
            return Err(invalid("file too large"));
        }
        let mut sha256 = [0u8; 32];
        r.read_exact(&mut sha256)?;
        list.push(FileEntry { name, size, sha256 });
    }
    Ok(list)
}

/// Relative path with `/` separators; no empty, `.`, `..` components, no drive or escape chars.
pub fn validate_name(name: &str) -> Result<(), String> {
    let bad = name.is_empty()
        || name.len() > 255
        || name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == "..");
    if bad {
        Err(format!("invalid file name {name:?}"))
    } else {
        Ok(())
    }
}

/// Appends one target to `dict` by executing its ops from `r` up to `OP_END`.
pub fn decode_target<R: Read>(r: &mut R, dict: &mut Vec<u8>, size: u64) -> io::Result<()> {
    let start = dict.len();
    let end = usize::try_from(size)
        .ok()
        .and_then(|s| start.checked_add(s))
        .ok_or_else(|| invalid("file too large"))?;
    loop {
        match read_u8(r)? {
            OP_END => break,
            OP_ADD => {
                let len = read_len(r, end - dict.len())?;
                let old = dict.len();
                dict.resize(old + len, 0);
                r.read_exact(&mut dict[old..])?;
            }
            OP_COPY => {
                let pos = read_varint(r)?;
                let len = read_len(r, end - dict.len())?;
                let pos = usize::try_from(pos)
                    .ok()
                    .filter(|p| p.checked_add(len).is_some_and(|e| e <= start))
                    .ok_or_else(|| invalid("copy outside the dictionary"))?;
                dict.extend_from_within(pos..pos + len);
            }
            op => return Err(invalid(&format!("unknown op {op}"))),
        }
    }
    if dict.len() != end {
        return Err(invalid("size mismatch"));
    }
    Ok(())
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn write_varint<W: Write>(w: &mut W, mut v: u64) -> io::Result<()> {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            return w.write_all(&[b]);
        }
        w.write_all(&[b | 0x80])?;
    }
}

pub fn read_varint<R: Read>(r: &mut R) -> io::Result<u64> {
    let mut v = 0u64;
    let mut shift = 0u32;
    loop {
        let b = read_u8(r)?;
        if shift > 63 || (shift == 63 && b & 0x7f > 1) {
            return Err(invalid("varint overflow"));
        }
        v |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
        shift += 7;
    }
}

fn read_len<R: Read>(r: &mut R, room: usize) -> io::Result<usize> {
    let n = read_varint(r)?;
    usize::try_from(n)
        .ok()
        .filter(|&n| n <= room)
        .ok_or_else(|| invalid("data exceeds the file size"))
}

fn read_u8<R: Read>(r: &mut R) -> io::Result<u8> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}

fn write_str<W: Write>(w: &mut W, s: &str) -> io::Result<()> {
    write_varint(w, s.len() as u64)?;
    w.write_all(s.as_bytes())
}

fn read_str<R: Read>(r: &mut R) -> io::Result<String> {
    let n = read_varint(r)?;
    if n > MAX_STR {
        return Err(invalid("string too long"));
    }
    let mut buf = vec![0u8; n as usize];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|_| invalid("string is not UTF-8"))
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}
