//! # Erasure Coding and Recovery
//!
//! Shreds are logically grouped into erasure sets or blocks. Each set contains 16 sequential data
//! shreds and 4 sequential coding shreds.
//!
//! Coding shreds in each set starting from `start_idx`:
//!   For each erasure set:
//!     generate `NUM_CODING` coding_shreds.
//!     index the coding shreds from `start_idx` to `start_idx + NUM_CODING - 1`.
//!
//!  model of an erasure set, with top row being data shreds and second being coding
//!  |<======================= NUM_DATA ==============================>|
//!  |<==== NUM_CODING ===>|
//!  +---+ +---+ +---+ +---+ +---+         +---+ +---+ +---+ +---+ +---+
//!  | D | | D | | D | | D | | D |         | D | | D | | D | | D | | D |
//!  +---+ +---+ +---+ +---+ +---+  . . .  +---+ +---+ +---+ +---+ +---+
//!  | C | | C | | C | | C | |   |         |   | |   | |   | |   | |   |
//!  +---+ +---+ +---+ +---+ +---+         +---+ +---+ +---+ +---+ +---+
//!
//!  shred structure for coding shreds
//!
//!   + ------- meta is set and used by transport, meta.size is actual length
//!   |           of data in the byte array shred.data
//!   |
//!   |          + -- data is stuff shipped over the wire, and has an included
//!   |          |        header
//!   V          V
//!  +----------+------------------------------------------------------------+
//!  | meta     |  data                                                      |
//!  |+---+--   |+---+---+---+---+------------------------------------------+|
//!  || s | .   || i |   | f | s |                                          ||
//!  || i | .   || n | i | l | i |                                          ||
//!  || z | .   || d | d | a | z |     shred.data(), or shred.data_mut()      ||
//!  || e |     || e |   | g | e |                                          ||
//!  |+---+--   || x |   | s |   |                                          ||
//!  |          |+---+---+---+---+------------------------------------------+|
//!  +----------+------------------------------------------------------------+
//!             |                |<=== coding shred part for "coding" =======>|
//!             |                                                            |
//!             |<============== data shred part for "coding"  ==============>|
//!
//!

use {
    reed_solomon_erasure::{galois_8::Field, ReconstructShard, ReedSolomon},
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ErasureConfig {
    num_data: usize,
    num_coding: usize,
}

impl ErasureConfig {
    pub(crate) fn new(num_data: usize, num_coding: usize) -> ErasureConfig {
        ErasureConfig {
            num_data,
            num_coding,
        }
    }

    pub(crate) fn num_data(self) -> usize {
        self.num_data
    }

    pub(crate) fn num_coding(self) -> usize {
        self.num_coding
    }
}

type Result<T> = std::result::Result<T, reed_solomon_erasure::Error>;

/// Represents an erasure "session" with a particular configuration and number of data and coding
/// shreds
#[derive(Debug, Clone)]
pub struct Session(ReedSolomon<Field>);

impl Session {
    pub fn new(data_count: usize, coding_count: usize) -> Result<Session> {
        let rs = ReedSolomon::new(data_count, coding_count)?;

        Ok(Session(rs))
    }

    pub fn new_from_config(config: &ErasureConfig) -> Result<Session> {
        let rs = ReedSolomon::new(config.num_data, config.num_coding)?;

        Ok(Session(rs))
    }

    /// Create coding blocks by overwriting `parity`
    pub fn encode<T, U>(&self, data: &[T], parity: &mut [U]) -> Result<()>
    where
        T: AsRef<[u8]>,
        U: AsRef<[u8]> + AsMut<[u8]>,
    {
        self.0.encode_sep(data, parity)
    }

    /// Recover data + coding blocks into data blocks
    pub fn decode_blocks<T>(&self, blocks: &mut [T]) -> Result<()>
    where
        T: ReconstructShard<Field>,
    {
        self.0.reconstruct_data(blocks)
    }
}

#[cfg(test)]
pub mod test {
    use {super::*, log::*, solana_sdk::clock::Slot};

    /// Specifies the contents of a 16-data-shred and 4-coding-shred erasure set
    /// Exists to be passed to `generate_blockstore_with_coding`
    #[derive(Debug, Copy, Clone)]
    pub struct ErasureSpec {
        /// Which 16-shred erasure set this represents
        pub set_index: u64,
        pub num_data: usize,
        pub num_coding: usize,
    }

    /// Specifies the contents of a slot
    /// Exists to be passed to `generate_blockstore_with_coding`
    #[derive(Debug, Clone)]
    pub struct SlotSpec {
        pub slot: Slot,
        pub set_specs: Vec<ErasureSpec>,
    }

    #[test]
    fn test_coding() {
        const N_DATA: usize = 4;
        const N_CODING: usize = 2;

        let session = Session::new(N_DATA, N_CODING).unwrap();

        let mut vs: Vec<Vec<u8>> = (0..N_DATA as u8).map(|i| (i..(16 + i)).collect()).collect();
        let v_orig: Vec<u8> = vs[0].clone();

        let mut coding_blocks: Vec<_> = (0..N_CODING).map(|_| vec![0u8; 16]).collect();

        let mut coding_blocks_slices: Vec<_> =
            coding_blocks.iter_mut().map(Vec::as_mut_slice).collect();
        let v_slices: Vec<_> = vs.iter().map(Vec::as_slice).collect();

        session
            .encode(v_slices.as_slice(), coding_blocks_slices.as_mut_slice())
            .expect("encoding must succeed");

        trace!("test_coding: coding blocks:");
        for b in &coding_blocks {
            trace!("test_coding: {:?}", b);
        }

        let erasure: usize = 1;
        let mut present = vec![true; N_DATA + N_CODING];
        present[erasure] = false;
        let erased = vs[erasure].clone();

        // clear an entry
        vs[erasure as usize].copy_from_slice(&[0; 16]);

        let mut blocks: Vec<_> = vs
            .iter_mut()
            .chain(coding_blocks.iter_mut())
            .map(Vec::as_mut_slice)
            .zip(present)
            .collect();

        session
            .decode_blocks(blocks.as_mut_slice())
            .expect("decoding must succeed");

        trace!("test_coding: vs:");
        for v in &vs {
            trace!("test_coding: {:?}", v);
        }
        assert_eq!(v_orig, vs[0]);
        assert_eq!(erased, vs[erasure]);
    }
}
