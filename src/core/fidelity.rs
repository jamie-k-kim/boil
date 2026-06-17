#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Fidelity {
    L0, // Architectural - Top-level structure and public interfaces
    L1, // Skeletal - All signatures with empty bodies
    L2, // Partial - Implementation stripped of boilerplate (error handling, logging)
}

impl Fidelity {
    pub fn all() -> Vec<Self> {
        vec![Fidelity::L0, Fidelity::L1, Fidelity::L2]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Fidelity::L0 => "L0-architectural",
            Fidelity::L1 => "L1-skeletal",
            Fidelity::L2 => "L2-partial",
        }
    }
}
