pub mod debt;
pub mod history;
pub mod quality;
pub mod stability;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub kind: String,
    pub file: String,
    pub subject: String,
    pub source: &'static str,
}

impl EvidenceRecord {
    pub fn new(
        kind: impl Into<String>,
        file: impl Into<String>,
        subject: impl Into<String>,
        source: &'static str,
    ) -> Self {
        Self {
            kind: kind.into(),
            file: file.into(),
            subject: subject.into(),
            source,
        }
    }
}
