//! Tests for the shared [`bump_entry`] helper and the TTL policy constants.
//!
//! The assertions run against real persistent storage in the SDK test host
//! (`Env::default()` with `testutils` enabled). All storage operations —
//! writes, TTL bumps, and the test-only `storage::Persistent::get_ttl`
//! reads — run inside `env.as_contract` for a registered test contract, the
//! same frame a caller's `bump_entry` executes in. The policy constants are
//! unchanged byte-for-byte from the escrow/multi-sig copies this helper
//! consolidates (issue #127).

use super::{bump_entry, BUMP_AMOUNT, BUMP_THRESHOLD};
use soroban_sdk::{
    contract, contracttype,
    testutils::{storage::Persistent as _, Ledger as _},
    Env,
};

/// A do-nothing contract registered only so the tests have a contract
/// context for storage access (`env.as_contract`).
#[contract]
struct TestContract;

/// A key written to persistent storage in these tests, mirroring the
/// `#[contracttype]` key enums the contract crates pass to the helper.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum TestKey {
    /// A persistent record keyed by id.
    Record(u64),
}

/// The day constant behind both policy values: 17,280 ledgers ≈ 1 day.
const DAY_IN_LEDGERS: u32 = 17_280;

/// Fresh-entry no-op: a bump on an entry whose remaining TTL is above
/// [`BUMP_THRESHOLD`] must not shorten or otherwise change it. The
/// persistent-entry floor is raised above the threshold for the duration of
/// the test, so the entry is "fresh" relative to the policy and the bump
/// must leave it alone.
#[test]
fn fresh_entry_bump_is_a_no_op() {
    let env = Env::default();
    let contract_id = env.register(TestContract, ());
    let key = TestKey::Record(1);

    env.as_contract(&contract_id, || {
        // Put the persistent-entry floor comfortably above the threshold so
        // the entry is fresh regardless of the host's exact floor arithmetic.
        env.ledger()
            .set_min_persistent_entry_ttl(BUMP_THRESHOLD + 10);
        env.storage().persistent().set(&key, &42_u64);

        let ttl_before = env.storage().persistent().get_ttl(&key);
        assert!(
            ttl_before >= BUMP_THRESHOLD,
            "entry (ttl {ttl_before}) must be fresh relative to the threshold"
        );

        bump_entry(&env, &key);

        assert_eq!(
            env.storage().persistent().get_ttl(&key),
            ttl_before,
            "a fresh entry must not be touched"
        );
        // The stored value is untouched: the helper only extends the TTL.
        assert_eq!(
            env.storage().persistent().get::<TestKey, u64>(&key),
            Some(42)
        );
    });
}

/// The core case: an entry inside the threshold window is extended to the
/// [`BUMP_AMOUNT`] horizon. Demonstrates that a future persistent-storage
/// migration (vesting, subscriptions, DAO, marketplace-style records) can
/// use the helper unmodified with any `#[contracttype]` key.
#[test]
fn bump_extends_an_eligible_entry_to_the_policy_horizon() {
    let env = Env::default();
    let contract_id = env.register(TestContract, ());
    let key = TestKey::Record(7);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &1_u64);
        let ttl_before = env.storage().persistent().get_ttl(&key);
        assert!(
            ttl_before < BUMP_THRESHOLD,
            "fresh entry (ttl {ttl_before}) is inside the threshold window"
        );

        bump_entry(&env, &key);

        let ttl_after = env.storage().persistent().get_ttl(&key);
        assert_eq!(
            ttl_after, BUMP_AMOUNT,
            "eligible entry lands on the horizon"
        );
        assert!(
            ttl_after > ttl_before,
            "the bump must actually extend the entry"
        );
        // The stored value is untouched: the helper only extends the TTL.
        assert_eq!(
            env.storage().persistent().get::<TestKey, u64>(&key),
            Some(1)
        );
    });
}

/// The helper extends a short-lived entry that is far inside the threshold
/// window — the near-expiry case the threshold exists for — and a repeated
/// bump on the freshly extended entry does not stack: the threshold/extend
/// pattern re-lands on the horizon rather than adding lifetimes.
#[test]
fn repeated_bumps_keep_an_aged_entry_at_the_horizon() {
    let env = Env::default();
    let contract_id = env.register(TestContract, ());
    let key = TestKey::Record(2);

    env.as_contract(&contract_id, || {
        // A low floor makes the freshly written entry short-lived — the
        // aged-entry situation a keeper bump exists to fix.
        env.ledger().set_min_persistent_entry_ttl(100);
        env.storage().persistent().set(&key, &1_u64);
        let aged_ttl = env.storage().persistent().get_ttl(&key);
        assert!(
            aged_ttl < 1_000,
            "entry (ttl {aged_ttl}) must be far inside the threshold window"
        );

        bump_entry(&env, &key);
        let extended = env.storage().persistent().get_ttl(&key);
        assert_eq!(
            extended, BUMP_AMOUNT,
            "aged entry lands on the policy horizon"
        );

        // A second bump on the freshly extended entry (ttl == BUMP_AMOUNT,
        // above the threshold) must not change it.
        bump_entry(&env, &key);
        assert_eq!(env.storage().persistent().get_ttl(&key), extended);
    });
}

/// Policy values are byte-identical to the copies this helper consolidates:
/// 30 days of ledgers and a threshold one day behind the amount.
#[test]
fn policy_constants_are_the_workspace_standard() {
    assert_eq!(BUMP_AMOUNT, 30 * DAY_IN_LEDGERS);
    assert_eq!(BUMP_AMOUNT, 518_400);
    assert_eq!(BUMP_THRESHOLD, 518_400 - 17_280);
    assert_eq!(BUMP_THRESHOLD, 501_120);
    const _: () = assert!(BUMP_THRESHOLD < BUMP_AMOUNT);
}
