use thiserror::Error;

/// Every failure the engine can report. Player-facing messages name the fix,
/// not just the fault, as required by spec §6.
#[derive(Debug, Error)]
pub enum Error {
    #[error("this is not a Digimon World 4 (USA) disc — use the NTSC-U release, SLUS_208.36")]
    NotThisDisc,

    #[error(
        "unrecognised revision of SLUS_208.36: found {found} copies of the {block} block, expected \
         {expected} — use your original NTSC-U ISO"
    )]
    WrongRevision {
        block: &'static str,
        found: usize,
        expected: usize,
    },

    #[error(
        "this disc is already modded ({rarity_nonzero} rows carry a crown) — use your original ISO"
    )]
    AlreadyModded { rarity_nonzero: usize },

    #[error(
        "the file {name} was not found in the disc image — this is not the supported Digimon \
         World 4 (USA) SLUS_208.36 release, or the image is incomplete"
    )]
    FileNotFound { name: String },

    #[error(
        "a directory-record name must be rewritten at its existing length: got {found} bytes, \
         expected {expected}"
    )]
    NameLengthMismatch { found: usize, expected: usize },

    #[error("SLUS_208.36 was not found in this disc image — use the NTSC-U release, SLUS_208.36")]
    ElfNotFound,

    #[error(
        "unexpected instruction {word:#010X} at {offset:#X} (expected a daddu) — the Very Hard \
         step was skipped, so use your original NTSC-U ISO"
    )]
    UnexpectedInstruction { offset: u64, word: u32 },

    #[error("invalid serial \"{0}\" — use 11 characters in the form SLUS_208.36")]
    SerialInvalid(String),

    #[error(
        "the output path is the same file as the input — choose a different output so your \
         original ISO is never modified"
    )]
    OutputIsInput,

    #[error("the output file already exists — choose a new name, or allow overwriting it")]
    OutputExists,

    #[error(
        "not enough free space at the destination: need {needed} bytes but only {free} are free \
         — free some space or choose another folder"
    )]
    NoSpace { needed: u64, free: u64 },

    #[error(
        "the written file failed verification: {0} — nothing was kept; re-run from your original \
         ISO"
    )]
    VerificationFailed(String),

    #[error(
        "could not read or write the disc image: {0} — check that the file and its folder are \
         readable and writable"
    )]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
