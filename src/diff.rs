//! Delta encoder: rsync-style rolling hash over a growing dictionary.
//!
//! The dictionary is indexed at every `BLOCK`-aligned offset. The target is scanned with a
//! rolling hash; a verified block match is extended backwards and forwards byte by byte and
//! emitted as `OP_COPY`, everything between matches as `OP_ADD`.

use crate::format::{write_varint, OP_ADD, OP_COPY};
use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
    io::{self, Write},
};

const BLOCK: usize = 32;

/// Keys are already well-mixed buzhash values; only spread them for the table.
#[derive(Default)]
struct KeyHasher(u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(8) ^ u64::from(b)).wrapping_mul(MIX);
        }
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = v.wrapping_mul(MIX);
    }
}

const MIX: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Default, Debug, Clone, Copy)]
pub struct Stats {
    pub copied: u64,
    pub added: u64,
}

pub struct Dictionary {
    data: Vec<u8>,
    index: HashMap<u64, u32, BuildHasherDefault<KeyHasher>>,
    table: [u64; 256],
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl Dictionary {
    pub fn new() -> Self {
        // Deterministic splitmix64 table for buzhash.
        let mut table = [0u64; 256];
        let mut s = 0x5032_5041_5443_4831u64;
        for t in table.iter_mut() {
            s = s.wrapping_add(MIX);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *t = z ^ (z >> 31);
        }
        Self {
            data: Vec::new(),
            index: HashMap::default(),
            table,
        }
    }

    fn hash(&self, window: &[u8]) -> u64 {
        window
            .iter()
            .fold(0u64, |h, &b| h.rotate_left(1) ^ self.table[b as usize])
    }

    fn roll(&self, h: u64, out: u8, inp: u8) -> u64 {
        let gone = self.table[out as usize].rotate_left(BLOCK as u32);
        h.rotate_left(1) ^ gone ^ self.table[inp as usize]
    }

    /// Adds bytes to the dictionary (sources first, then every produced target in order).
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), String> {
        let off = self.data.len();
        if (off as u64) + (bytes.len() as u64) > u64::from(u32::MAX) {
            return Err("dictionary exceeds 4 GiB".into());
        }
        self.data.extend_from_slice(bytes);
        let mut p = off;
        while p + BLOCK <= self.data.len() {
            let h = self.hash(&self.data[p..p + BLOCK]);
            self.index.entry(h).or_insert(p as u32);
            p += BLOCK;
        }
        Ok(())
    }

    /// Writes the ops for `target` (without `OP_END`).
    pub fn encode<W: Write>(&self, target: &[u8], out: &mut W) -> io::Result<Stats> {
        let d = &self.data;
        let mut st = Stats::default();
        let mut lit = 0usize;
        let mut i = 0usize;
        let mut h = if target.len() >= BLOCK {
            self.hash(&target[..BLOCK])
        } else {
            0
        };
        while i + BLOCK <= target.len() {
            if let Some(&p) = self.index.get(&h) {
                let p = p as usize;
                if d[p..p + BLOCK] == target[i..i + BLOCK] {
                    let (mut s, mut q0) = (i, p);
                    while s > lit && q0 > 0 && d[q0 - 1] == target[s - 1] {
                        s -= 1;
                        q0 -= 1;
                    }
                    let (mut e, mut q1) = (i + BLOCK, p + BLOCK);
                    while e < target.len() && q1 < d.len() && d[q1] == target[e] {
                        e += 1;
                        q1 += 1;
                    }
                    emit_add(out, &target[lit..s], &mut st)?;
                    emit_copy(out, q0, e - s, &mut st)?;
                    i = e;
                    lit = e;
                    if i + BLOCK <= target.len() {
                        h = self.hash(&target[i..i + BLOCK]);
                    }
                    continue;
                }
            }
            if i + BLOCK < target.len() {
                h = self.roll(h, target[i], target[i + BLOCK]);
            }
            i += 1;
        }
        emit_add(out, &target[lit..], &mut st)?;
        Ok(st)
    }
}

fn emit_add<W: Write>(out: &mut W, bytes: &[u8], st: &mut Stats) -> io::Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    out.write_all(&[OP_ADD])?;
    write_varint(out, bytes.len() as u64)?;
    out.write_all(bytes)?;
    st.added += bytes.len() as u64;
    Ok(())
}

fn emit_copy<W: Write>(out: &mut W, pos: usize, len: usize, st: &mut Stats) -> io::Result<()> {
    out.write_all(&[OP_COPY])?;
    write_varint(out, pos as u64)?;
    write_varint(out, len as u64)?;
    st.copied += len as u64;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{decode_target, OP_END};

    fn pseudo(n: u32, seed: u32) -> Vec<u8> {
        (0..n)
            .map(|i| (i.wrapping_add(seed).wrapping_mul(2_654_435_761) >> 13) as u8)
            .collect()
    }

    #[test]
    fn roundtrip_with_previous_target() {
        let src = pseudo(20_000, 1);
        let mut t1 = src[5_000..12_000].to_vec();
        t1.extend_from_slice(b"new bytes");
        t1.extend_from_slice(&src[100..3_000]);
        t1[200] ^= 0xff;
        let mut t2 = pseudo(4_000, 7);
        t2.extend_from_slice(&t1[1_000..6_000]);

        let mut dict = Dictionary::new();
        dict.append(&src).unwrap();
        let mut ops = Vec::new();
        let st1 = dict.encode(&t1, &mut ops).unwrap();
        ops.push(OP_END);
        dict.append(&t1).unwrap();
        let st2 = dict.encode(&t2, &mut ops).unwrap();
        ops.push(OP_END);
        assert!(st1.copied > st1.added);
        assert!(st2.copied >= 5_000);

        let mut out = src.clone();
        let mut r = ops.as_slice();
        decode_target(&mut r, &mut out, t1.len() as u64).unwrap();
        decode_target(&mut r, &mut out, t2.len() as u64).unwrap();
        assert_eq!(&out[src.len()..src.len() + t1.len()], &t1[..]);
        assert_eq!(&out[src.len() + t1.len()..], &t2[..]);
        assert!(r.is_empty());
    }

    #[test]
    fn short_and_empty_targets() {
        let mut dict = Dictionary::new();
        dict.append(&pseudo(1_000, 3)).unwrap();
        for t in [Vec::new(), b"abc".to_vec()] {
            let mut ops = Vec::new();
            dict.encode(&t, &mut ops).unwrap();
            ops.push(OP_END);
            let mut out = vec![0u8; 1_000];
            decode_target(&mut ops.as_slice(), &mut out, t.len() as u64).unwrap();
            assert_eq!(&out[1_000..], &t[..]);
        }
    }
}
