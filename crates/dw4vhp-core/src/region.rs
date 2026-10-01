/// A same-length, in-place edit: `bytes` replaces the input's bytes starting at
/// `offset`. The writer rejects any region whose bytes would run past the end of
/// the file instead of growing or moving it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteRegion {
    pub offset: u64,
    pub bytes: Vec<u8>,
}
