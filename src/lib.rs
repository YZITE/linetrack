#![no_std]

extern crate alloc;
use alloc::vec::Vec;

// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

/// A pre-computed line cache, caching
/// line ending offsets to speed up later line:col computations
#[derive(Clone, Debug)]
pub struct LineCache(Vec<(usize, usize)>);

impl LineCache {
    pub fn new(s: &str) -> Self {
        Self(
            s.bytes()
                .enumerate()
                .filter(|&(_, i)| i == b'\n')
                .enumerate()
                .map(|(lnr, (bkpt, _))| (lnr + 1, bkpt))
                .collect(),
        )
    }

    /// returns the zero-based (line, col) information
    pub fn run(&self, pos: usize) -> (usize, usize) {
        // if the line cache returns e.g. lnr=1, the line 0 ends
        // before our position, so we are in line 1. etc.
        let (lnr, bkpt) = self
            .0
            .iter()
            .copied()
            .take_while(|&(_, bkpt)| bkpt <= pos)
            .last()
            .unwrap_or((0, 0));
        (lnr, pos - bkpt)
    }
}

/// A position tracker, only useful if the requested offset information only
/// proceeds forwards or the implied failure at backwards moves is just annoying,
/// but not fatal.
#[derive(Clone, Copy, Debug, Default)]
pub struct PosTrackerExtern {
    offset: usize,
    line: usize,
    column: usize,
}

impl PosTrackerExtern {
    // always give `dat` as an argument (but it's start address shouldn't change),
    // to prevent borrowing conflicts or such.
    pub fn update<'a>(
        &mut self,
        dat: &'a [u8],
        new_offset: usize,
    ) -> Option<(&'a [u8], usize, usize)> {
        new_offset.checked_sub(self.offset)?;
        let mut ldif = 0;
        let mut cdif = 0;
        let slc = &dat[self.offset..new_offset];
        for &i in slc {
            if i == b'\n' {
                cdif = 0;
                ldif += 1;
            } else if i != b'\r' {
                cdif += 1;
            }
        }
        self.offset = new_offset;
        self.line += ldif;
        if ldif != 0 {
            self.column = 0;
        }
        self.column += cdif;
        Some((slc, ldif, cdif))
    }
}

/// Similar to [`PosTrackerExtern`], but keeps a reference to the source around,
/// so that the users doesn't need to supply the `dat` argument to `update`
/// every time. Basically just a simple convenience shim.
#[derive(Clone, Copy, Debug)]
pub struct PosTrackerDatRef<'a> {
    dat: &'a [u8],
    inner: PosTrackerExtern,
}

impl<'a> PosTrackerDatRef<'a> {
    #[inline]
    pub fn new(dat: &'a [u8]) -> Self {
        Self {
            dat,
            inner: Default::default(),
        }
    }

    #[inline(always)]
    pub fn inner(&self) -> PosTrackerExtern {
        self.inner
    }

    /// a simple wrapper around [`PosTrackerExtern::update`].
    #[inline]
    pub fn update(&mut self, new_offset: usize) -> Option<(&'a [u8], usize, usize)> {
        self.inner.update(self.dat, new_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_source_location() {
        const SRC: &str = r#"Das ist ein Test!
Hurra!
"#;
        let lc = LineCache::new(SRC);
        assert_eq!(lc.0, alloc::vec![(1, 17), (2, 24)]);
        assert_eq!(lc.run(3), (0, 3));
        assert_eq!(lc.run(20), (1, 3));
    }
}
