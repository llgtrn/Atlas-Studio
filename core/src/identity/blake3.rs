//! BLAKE3 unkeyed hash, natively owned by Atlas (ADR 0005).
//!
//! A faithful transcription of the hash mode of BLAKE3's published reference implementation
//! (`BLAKE3-team/BLAKE3`, `reference_impl/reference_impl.rs`, CC0-1.0 OR Apache-2.0), pinned by the
//! official `test_vectors/test_vectors.json` below. Deliberately NOT included: `keyed_hash` and
//! `derive_key`. Those modes take secret inputs, where timing side channels and constant-time
//! comparison matter; any future Atlas use of a secret key must go through a vetted external
//! implementation (`EXTERNAL_BOUNDARY`), never through this module. Unkeyed hashing of public
//! bytes has no secret to leak, and BLAKE3's add-rotate-xor design has no table lookups.
//!
//! Structure mirrors the reference implementation section for section so it can be audited
//! against it line by line.

const OUT_LEN: usize = 32;
const BLOCK_LEN: usize = 64;
const CHUNK_LEN: usize = 1024;

const CHUNK_START: u32 = 1 << 0;
const CHUNK_END: u32 = 1 << 1;
const PARENT: u32 = 1 << 2;
const ROOT: u32 = 1 << 3;

const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

const MSG_PERMUTATION: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

/// The mixing function G, applied to a column or a diagonal of the state.
fn g(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(mx);
    state[d] = (state[d] ^ state[a]).rotate_right(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(12);
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(my);
    state[d] = (state[d] ^ state[a]).rotate_right(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(7);
}

fn round(state: &mut [u32; 16], m: &[u32; 16]) {
    g(state, 0, 4, 8, 12, m[0], m[1]);
    g(state, 1, 5, 9, 13, m[2], m[3]);
    g(state, 2, 6, 10, 14, m[4], m[5]);
    g(state, 3, 7, 11, 15, m[6], m[7]);
    g(state, 0, 5, 10, 15, m[8], m[9]);
    g(state, 1, 6, 11, 12, m[10], m[11]);
    g(state, 2, 7, 8, 13, m[12], m[13]);
    g(state, 3, 4, 9, 14, m[14], m[15]);
}

fn permute(m: &mut [u32; 16]) {
    let mut permuted = [0; 16];
    for (i, word) in permuted.iter_mut().enumerate() {
        *word = m[MSG_PERMUTATION[i]];
    }
    *m = permuted;
}

fn compress(
    chaining_value: &[u32; 8],
    block_words: &[u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    let counter_low = counter as u32;
    let counter_high = (counter >> 32) as u32;
    #[rustfmt::skip]
    let mut state = [
        chaining_value[0], chaining_value[1], chaining_value[2], chaining_value[3],
        chaining_value[4], chaining_value[5], chaining_value[6], chaining_value[7],
        IV[0],             IV[1],             IV[2],             IV[3],
        counter_low,       counter_high,      block_len,         flags,
    ];
    let mut block = *block_words;
    for round_index in 0..7 {
        round(&mut state, &block);
        if round_index < 6 {
            permute(&mut block);
        }
    }
    for i in 0..8 {
        state[i] ^= state[i + 8];
        state[i + 8] ^= chaining_value[i];
    }
    state
}

fn first_8_words(compression_output: [u32; 16]) -> [u32; 8] {
    let mut words = [0; 8];
    words.copy_from_slice(&compression_output[..8]);
    words
}

fn words_from_little_endian_bytes(bytes: &[u8], words: &mut [u32]) {
    for (four_bytes, word) in bytes.chunks_exact(4).zip(words) {
        *word = u32::from_le_bytes([four_bytes[0], four_bytes[1], four_bytes[2], four_bytes[3]]);
    }
}

/// The state just before choosing between an 8-word chaining value and root output bytes.
struct Output {
    input_chaining_value: [u32; 8],
    block_words: [u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
}

impl Output {
    fn chaining_value(&self) -> [u32; 8] {
        first_8_words(compress(
            &self.input_chaining_value,
            &self.block_words,
            self.counter,
            self.block_len,
            self.flags,
        ))
    }

    fn root_output_bytes(&self, out_slice: &mut [u8]) {
        for (output_block_counter, out_block) in out_slice.chunks_mut(2 * OUT_LEN).enumerate() {
            let words = compress(
                &self.input_chaining_value,
                &self.block_words,
                output_block_counter as u64,
                self.block_len,
                self.flags | ROOT,
            );
            for (word, out_word) in words.iter().zip(out_block.chunks_mut(4)) {
                out_word.copy_from_slice(&word.to_le_bytes()[..out_word.len()]);
            }
        }
    }
}

struct ChunkState {
    chaining_value: [u32; 8],
    chunk_counter: u64,
    block: [u8; BLOCK_LEN],
    block_len: u8,
    blocks_compressed: u8,
}

impl ChunkState {
    fn new(chunk_counter: u64) -> Self {
        Self {
            chaining_value: IV,
            chunk_counter,
            block: [0; BLOCK_LEN],
            block_len: 0,
            blocks_compressed: 0,
        }
    }

    fn len(&self) -> usize {
        BLOCK_LEN * self.blocks_compressed as usize + self.block_len as usize
    }

    fn start_flag(&self) -> u32 {
        if self.blocks_compressed == 0 {
            CHUNK_START
        } else {
            0
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            // A full block buffer with more input still to come is never the chunk's last block.
            if self.block_len as usize == BLOCK_LEN {
                let mut block_words = [0; 16];
                words_from_little_endian_bytes(&self.block, &mut block_words);
                self.chaining_value = first_8_words(compress(
                    &self.chaining_value,
                    &block_words,
                    self.chunk_counter,
                    BLOCK_LEN as u32,
                    self.start_flag(),
                ));
                self.blocks_compressed += 1;
                self.block = [0; BLOCK_LEN];
                self.block_len = 0;
            }
            let take = (BLOCK_LEN - self.block_len as usize).min(input.len());
            self.block[self.block_len as usize..][..take].copy_from_slice(&input[..take]);
            self.block_len += take as u8;
            input = &input[take..];
        }
    }

    fn output(&self) -> Output {
        let mut block_words = [0; 16];
        words_from_little_endian_bytes(&self.block, &mut block_words);
        Output {
            input_chaining_value: self.chaining_value,
            block_words,
            counter: self.chunk_counter,
            block_len: self.block_len as u32,
            flags: self.start_flag() | CHUNK_END,
        }
    }
}

fn parent_output(left_child_cv: [u32; 8], right_child_cv: [u32; 8]) -> Output {
    let mut block_words = [0; 16];
    block_words[..8].copy_from_slice(&left_child_cv);
    block_words[8..].copy_from_slice(&right_child_cv);
    Output {
        input_chaining_value: IV,
        block_words,
        counter: 0,
        block_len: BLOCK_LEN as u32,
        flags: PARENT,
    }
}

/// An incremental BLAKE3 hasher (unkeyed hash mode) accepting any number of writes.
pub struct Hasher {
    chunk_state: ChunkState,
    cv_stack: [[u32; 8]; 54],
    cv_stack_len: u8,
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher {
    pub fn new() -> Self {
        Self {
            chunk_state: ChunkState::new(0),
            cv_stack: [[0; 8]; 54],
            cv_stack_len: 0,
        }
    }

    fn push_stack(&mut self, cv: [u32; 8]) {
        self.cv_stack[self.cv_stack_len as usize] = cv;
        self.cv_stack_len += 1;
    }

    fn pop_stack(&mut self) -> [u32; 8] {
        self.cv_stack_len -= 1;
        self.cv_stack[self.cv_stack_len as usize]
    }

    /// Merge every subtree this chunk completes: one per trailing zero bit of the new chunk total.
    fn add_chunk_chaining_value(&mut self, mut new_cv: [u32; 8], mut total_chunks: u64) {
        while total_chunks & 1 == 0 {
            new_cv = parent_output(self.pop_stack(), new_cv).chaining_value();
            total_chunks >>= 1;
        }
        self.push_stack(new_cv);
    }

    pub fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            // A complete chunk with more input still to come is never the root.
            if self.chunk_state.len() == CHUNK_LEN {
                let chunk_cv = self.chunk_state.output().chaining_value();
                let total_chunks = self.chunk_state.chunk_counter + 1;
                self.add_chunk_chaining_value(chunk_cv, total_chunks);
                self.chunk_state = ChunkState::new(total_chunks);
            }
            let take = (CHUNK_LEN - self.chunk_state.len()).min(input.len());
            self.chunk_state.update(&input[..take]);
            input = &input[take..];
        }
    }

    /// Write any number of output bytes (the extendable output; the first 32 are the hash).
    pub fn finalize_xof(&self, out_slice: &mut [u8]) {
        let mut output = self.chunk_state.output();
        let mut parent_nodes_remaining = self.cv_stack_len as usize;
        while parent_nodes_remaining > 0 {
            parent_nodes_remaining -= 1;
            output = parent_output(
                self.cv_stack[parent_nodes_remaining],
                output.chaining_value(),
            );
        }
        output.root_output_bytes(out_slice);
    }

    pub fn finalize(&self) -> [u8; OUT_LEN] {
        let mut out = [0; OUT_LEN];
        self.finalize_xof(&mut out);
        out
    }
}

/// The 32-byte BLAKE3 hash of `input`.
pub fn hash(input: &[u8]) -> [u8; OUT_LEN] {
    let mut hasher = Hasher::new();
    hasher.update(input);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The official vectors' input: byte `i` is `i % 251`.
    fn vector_input(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    // Every case of BLAKE3's official test_vectors/test_vectors.json (`hash` field, first 32
    // bytes): the empty input through multi-level trees. Unkeyed only; the file's `keyed_hash` and
    // `derive_key` fields exercise modes Atlas deliberately does not own.
    const VECTORS: &[(usize, &str)] = include!("blake3_vectors.in");

    #[test]
    fn official_test_vectors_match() {
        assert_eq!(VECTORS.len(), 35);
        for (len, expected) in VECTORS {
            assert_eq!(
                hex(&hash(&vector_input(*len))),
                *expected,
                "BLAKE3 official vector, input length {len}"
            );
        }
    }

    #[test]
    fn extended_output_matches_every_official_131_byte_vector() {
        assert_eq!(XOF_VECTORS.len(), 35);
        for (len, expected) in XOF_VECTORS {
            let mut hasher = Hasher::new();
            hasher.update(&vector_input(*len));
            let mut out = [0u8; 131];
            hasher.finalize_xof(&mut out);
            assert_eq!(hex(&out), *expected, "XOF, input length {len}");
        }
    }

    const XOF_VECTORS: &[(usize, &str)] = include!("blake3_xof_vectors.in");

    #[test]
    fn every_update_split_equals_the_one_shot_hash() {
        // Change detection must never see a different digest for identical bytes merely because
        // they arrived in different read sizes: every split size, including ones straddling block
        // (64) and chunk (1024) boundaries, must reproduce the one-shot result.
        let input = vector_input(5_000);
        let expected = hash(&input);
        for split in [1, 3, 63, 64, 65, 1000, 1023, 1024, 1025, 2049, 4999] {
            let mut hasher = Hasher::new();
            for piece in input.chunks(split) {
                hasher.update(piece);
            }
            assert_eq!(hasher.finalize(), expected, "split size {split}");
        }
    }

    #[test]
    fn compress_uses_the_high_counter_word() {
        // The official vectors never reach 2^32 chunks (4 TiB), so they cannot see a compression
        // function that drops the counter's high word. Differential oracle: these words were
        // computed with the donor's own reference_impl.rs `compress` for the same arguments.
        let block: [u32; 16] = core::array::from_fn(|i| i as u32);
        let low = compress(&IV, &block, 5, 64, CHUNK_START);
        let high = compress(&IV, &block, (1u64 << 32) + 5, 64, CHUNK_START);
        assert_ne!(
            low, high,
            "the high 32 bits of the counter must reach the state"
        );
        assert_eq!(first_8_words(high), HIGH_COUNTER_ORACLE);
    }

    const HIGH_COUNTER_ORACLE: [u32; 8] = include!("blake3_high_counter.in");

    #[test]
    fn every_official_vector_survives_every_update_split() {
        // Byte-at-a-time plus splits straddling block, chunk and multi-chunk boundaries, for every
        // vector length: a streaming reader may deliver bytes in any sizes.
        for (len, expected) in VECTORS {
            let input = vector_input(*len);
            for split in [1, 63, 64, 65, 1023, 1024, 1025, 4096, 65536] {
                let mut hasher = Hasher::new();
                for piece in input.chunks(split) {
                    hasher.update(piece);
                }
                assert_eq!(
                    hex(&hasher.finalize()),
                    *expected,
                    "len {len}, split {split}"
                );
            }
        }
    }

    /// Deterministic xorshift64* stream, so a failing differential case is reproducible.
    struct Stream(u64);

    impl Stream {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn below(&mut self, bound: u64) -> usize {
            (self.next() % bound) as usize
        }
    }

    #[test]
    fn differential_against_the_vetted_blake3_crate() {
        // Independent oracle: the upstream optimized implementation (SIMD, different code path),
        // present only as a dev-dependency. Lengths cluster around the block/chunk/tree boundaries
        // where a port goes wrong, plus random lengths up to 1 MiB, over four input patterns, fed
        // through random update splits; both the 32-byte hash and a 200-byte XOF are compared.
        let mut stream = Stream(0x9E37_79B9_7F4A_7C15);
        let mut lengths = vec![0, 1, 63, 64, 65, 1023, 1024, 1025, 2048, 2049, 3072, 3073];
        lengths.extend([8191, 8192, 8193, 16_384, 16_385, 1 << 20]);
        for _ in 0..24 {
            lengths.push(stream.below(1 << 20) + 1);
        }
        for (case, len) in lengths.into_iter().enumerate() {
            let input: Vec<u8> = match case % 4 {
                0 => vec![0; len],
                1 => vec![0xff; len],
                2 => vector_input(len),
                _ => (0..len).map(|_| stream.next() as u8).collect(),
            };
            let mut ours = Hasher::new();
            let mut rest = &input[..];
            while !rest.is_empty() {
                let take = (stream.below(70_000) + 1).min(rest.len());
                ours.update(&rest[..take]);
                rest = &rest[take..];
            }
            assert_eq!(
                ours.finalize(),
                *::blake3::hash(&input).as_bytes(),
                "case {case}, len {len}"
            );
            let mut ours_xof = [0u8; 200];
            ours.finalize_xof(&mut ours_xof);
            let mut theirs_xof = [0u8; 200];
            ::blake3::Hasher::new()
                .update(&input)
                .finalize_xof()
                .fill(&mut theirs_xof);
            assert_eq!(ours_xof, theirs_xof, "xof case {case}, len {len}");
        }
    }

    #[test]
    fn a_chunk_beyond_two_to_the_32_matches_the_vetted_crate() {
        // Official vectors never reach chunk 2^32; the crate's hazmat subtree API can, so the
        // non-root chaining value of a chunk at that counter is compared directly.
        use ::blake3::hazmat::HasherExt;
        let counter = (1u64 << 32) + 7;
        let data = vector_input(777);
        let mut chunk = ChunkState::new(counter);
        chunk.update(&data);
        let ours: Vec<u8> = chunk
            .output()
            .chaining_value()
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        let theirs = ::blake3::Hasher::new()
            .set_input_offset(counter * CHUNK_LEN as u64)
            .update(&data)
            .finalize_non_root();
        assert_eq!(ours, theirs.to_vec());
    }

    #[test]
    fn production_source_carries_no_keyed_or_derive_key_machinery() {
        // ADR 0005: only the unkeyed hash of public bytes is Atlas-owned. Keyed and derive-key
        // modes (secret inputs) stay EXTERNAL_BOUNDARY; their flags and entry points must never
        // appear in this module's non-test source.
        let source = include_str!("blake3.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "KEYED_HASH",
            "DERIVE_KEY",
            "1 << 4",
            "1 << 5",
            "1 << 6",
            "fn keyed_hash",
            "fn derive_key",
            "new_keyed",
            "new_derive_key",
            "key_words",
        ] {
            assert!(
                !production.contains(forbidden),
                "forbidden secret-input machinery `{forbidden}` in identity::blake3"
            );
        }
    }
}
