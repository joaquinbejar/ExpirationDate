//! Hand-written comparison and hashing impls for [`ExpirationDate`].
//!
//! Equality and ordering are routed through [`ExpirationDate::get_days`] so
//! that `Days` and `DateTime` variants compare on a common day-count scale,
//! with an [`EPSILON`] tolerance on equality. These impls are hand-written on
//! purpose — do not replace them with `#[derive]`, as that would change
//! observable semantics (see crate-level docs and skill rules).

use crate::{EPSILON, ExpirationDate};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

impl Hash for ExpirationDate {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Days(d) => {
                state.write_u8(0);
                d.hash(state);
            }
            Self::DateTime(dt) => {
                state.write_u8(1);
                dt.timestamp().hash(state);
                dt.timestamp_subsec_nanos().hash(state);
            }
        }
    }
}

impl PartialEq for ExpirationDate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.get_days(), other.get_days()) {
            (Ok(s), Ok(o)) => (s.to_dec() - o.to_dec()).abs() < EPSILON,
            // If day conversion fails for either side, avoid silently treating it as zero.
            _ => false,
        }
    }
}

impl Eq for ExpirationDate {}

impl PartialOrd for ExpirationDate {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpirationDate {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.get_days(), other.get_days()) {
            (Ok(self_days), Ok(other_days)) => self_days.cmp(&other_days),
            // Keep a total order even on conversion errors, without masking them as ZERO.
            (Err(self_err), Err(other_err)) => self_err.to_string().cmp(&other_err.to_string()),
            (Err(_), Ok(_)) => Ordering::Less,
            (Ok(_), Err(_)) => Ordering::Greater,
        }
    }
}
