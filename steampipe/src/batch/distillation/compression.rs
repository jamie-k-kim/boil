#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressionLevel {
    DropEntirely,      // 0-4
    KeepSignatures,    // 5-14
    Skeletonize,       // 15-29
    StripComments,     // 30-49
    PreserveAll,       // 50+
}

impl CompressionLevel {
    pub fn from_score(score: f64) -> Self {
        match score as i32 {
            100..=i32::MAX => CompressionLevel::PreserveAll,
            40..=99        => CompressionLevel::StripComments,
            15..=39        => CompressionLevel::Skeletonize,
            5..=14         => CompressionLevel::KeepSignatures,
            _              => CompressionLevel::DropEntirely,
        }
    }
}
