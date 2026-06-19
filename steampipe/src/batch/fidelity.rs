/// The three compression fidelity levels used when generating batch layers.
///
/// The numeric suffix reflects how much of the original code is visible —
/// a higher number means more of the code has been removed.
///
/// | Level | Folder name         | What survives                                              |
/// |-------|---------------------|------------------------------------------------------------|
/// | L0    | `L0_partial`        | All code minus comments, debug statements, verbose literals|
/// | L1    | `L1_skeletal`       | Types, signatures, and struct/class outlines only          |
/// | L2    | `L2_architectural`  | Top-level declarations and public interfaces only          |
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Fidelity {
    /// Partial — least compressed. Strips comments, debug statements, and verbose literals.
    L0,
    /// Skeletal — mid compression. Retains all types and function signatures; bodies replaced with `{ ... }`.
    L1,
    /// Architectural — most compressed. Retains only top-level declarations and public interfaces.
    L2,
}

impl Fidelity {
    pub fn all() -> Vec<Self> {
        vec![Fidelity::L0, Fidelity::L1, Fidelity::L2]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Fidelity::L0 => "L0_partial",
            Fidelity::L1 => "L1_skeletal",
            Fidelity::L2 => "L2_architectural",
        }
    }
}
