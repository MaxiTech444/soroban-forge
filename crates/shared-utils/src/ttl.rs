//! Ledger-time constants and helper for persistent-storage TTL bumps.
//!
//! Soroban persistent entries expire unless their TTL is extended, and an
//! expired entry is not readable: reads on an expired key fail with the
//! host's entry-missing error (see the expired-entry trap in
//! [KNOWN-LIMITATIONS §5](../../docs/KNOWN-LIMITATIONS.md)). Contracts that
//! keep long-lived records in persistent storage must therefore extend the
//! entry's TTL on every write — and keepers can extend it for entries that
//! sit idle between writes.
//!
//! The policy is the standard threshold/extend-to pattern: extend an entry
//! to [`BUMP_AMOUNT`] ledgers only when its remaining TTL has fallen inside
//! [`BUMP_THRESHOLD`] ledgers. While the entry is fresh the bump is a cheap
//! no-op; near expiry it is decisive. The 30-day horizon comfortably covers
//! a funded contract between keeper touches.

use soroban_sdk::{Env, IntoVal, Val};

/// One ledger closes roughly every 5 seconds, so 17,280 ledgers ≈ 1 day.
const DAY_IN_LEDGERS: u32 = 17_280;

/// Lifetime applied on every TTL touch: 30 days in ledgers.
pub const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// Bump only when the entry is within this window of expiring.
pub const BUMP_THRESHOLD: u32 = BUMP_AMOUNT - DAY_IN_LEDGERS;

/// Bump a persistent entry's TTL to the [`BUMP_AMOUNT`] horizon when it
/// falls inside [`BUMP_THRESHOLD`]. The standard threshold/extend pattern:
/// cheap no-op while the entry is fresh, decisive near expiry.
///
/// Generic over any storage key (`IntoVal<Env, Val>`), so every contract
/// crate can route its persistent entries through this one helper — the
/// crate-local `DataKey` enums satisfy the bound as-is — instead of keeping
/// a per-crate copy with the same constants.
pub fn bump_entry<K: IntoVal<Env, Val>>(env: &Env, key: &K) {
    env.storage()
        .persistent()
        .extend_ttl(key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

#[cfg(test)]
mod tests;
