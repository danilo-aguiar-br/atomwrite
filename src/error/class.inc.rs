/// Classification of error recoverability for retry decisions.
///
/// Used by callers to determine whether an operation can be retried.
/// The NDJSON output serializes this as the `error_class` string field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Transient failure that may resolve on retry (e.g., disk full, I/O).
    Transient,
    /// Conflict requiring state reload before retry (e.g., checksum mismatch).
    Conflict,
    /// Precondition not met; retry without fixing precondition will fail.
    PreconditionFailed,
    /// Permanent failure; retry will not help.
    Permanent,
}

impl ErrorClass {
    /// Return the string representation for NDJSON serialization.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Conflict => "conflict",
            Self::PreconditionFailed => "precondition_failed",
            Self::Permanent => "permanent",
        }
    }

    /// Return true if this class indicates a retry may succeed.
    ///
    /// Both [`Transient`](Self::Transient) and [`Conflict`](Self::Conflict)
    /// are considered retryable.
    #[inline]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient | Self::Conflict)
    }

    /// Return true if this class indicates a permanent failure.
    ///
    /// Only [`Permanent`](Self::Permanent) errors are truly permanent.
    /// [`PreconditionFailed`](Self::PreconditionFailed) errors may succeed
    /// if the caller fixes the precondition first.
    #[inline]
    pub const fn is_permanent(&self) -> bool {
        matches!(self, Self::Permanent)
    }
}
