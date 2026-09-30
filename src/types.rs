//! Shared types: policy model, storage keys, errors, and the pure parsed-call
//! representation that the decision engine operates on.
#![allow(missing_docs)] // Soroban type/error macros synthesize undocumented conversion metadata.

use soroban_sdk::{contracterror, contracttype, Address, Symbol, Vec};

/// Warning threshold percentage for dead-man switch health evaluation (80%).
pub const DMS_WARN_THRESHOLD_PERCENT: u64 = 80;

/// Dead-man switch health status returned by `dms_health`.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DmsHealthStatus {
    /// Grace period is below the warning threshold.
    Ok,
    /// Grace period is at least 80% elapsed but has not expired.
    Warn,
    /// Grace period has elapsed.
    Expired,
}

/// Hard bound on rolling-window entries. Above this, the engine merges the two
/// oldest entries forward (conservative over-count) — see SPEC §3.1.
pub const MAX_WINDOW_ENTRIES: usize = 8192;

/// Hard bound on the number of entries in `recipients` and
/// `recipient_window_caps`. Keeps allowlist scans and per-recipient storage
/// bounded and predictable (SPEC §3 / §8).
pub const MAX_RECIPIENT_ENTRIES: usize = 256;

/// Per-policy rolling spend ledger for SAC asset transfers and protocol call counts.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowState {
    /// Cached rolling total (sum of non-expired entries).
    pub total: i128,
    /// Chronological spend entries (oldest first).
    pub entries: Vec<SpendEntry>,
    /// Per-recipient rolling spend ledgers for recipients with an override cap.
    pub recipients: Vec<RecipientWindowState>,
    /// Protocol call count entries (for rate limiting).
    pub protocol_call_entries: Vec<ProtocolCallEntry>,
}

/// Rolling spend ledger for a single recipient.
#[allow(missing_docs)] // contracttype synthesizes private conversion metadata
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipientWindowState {
    /// Account receiving transfers tracked by this ledger.
    pub recipient: Address,
    /// Cached rolling total (sum of non-expired entries).
    pub total: i128,
    /// Chronological spend entries (oldest first).
    pub entries: Vec<SpendEntry>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendEntry {
    pub ts: u64,
    pub amount: i128,
}

/// A protocol call entry in the rolling-window counter (for rate limiting).
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolCallEntry {
    pub ts: u64,
    pub count: u32,
}

/// Per-recipient rolling-window cap override.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipientCap {
    /// Recipient to which this override applies.
    pub recipient: Address,
    /// Rolling cap for this recipient within `window_secs`; 0 = disabled / fall back to global.
    pub cap: i128,
}

/// The policy an admin installs on the account. See SPEC §3/§4.
#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub struct PolicyConfig {
    /// Per asset-transfer call cap; 0 = disabled.
    pub per_tx_cap: i128,
    /// Rolling window width in seconds.
    pub window_secs: u64,
    /// Rolling spend cap within `window_secs`; 0 = disabled.
    pub window_cap: i128,
    /// SAC token contracts whose transfers get parsed and enforced.
    pub assets: Vec<Address>,
    /// Allowlisted non-asset contracts the account may call.
    pub protocols: Vec<ProtocolRule>,
    /// Allowed SAC transfer destinations.
    pub recipients: Vec<Address>,
    /// Per-recipient rolling-window cap overrides; recipients not listed here
    /// use the global `window_cap`. Storage bounded by `MAX_RECIPIENT_ENTRIES`.
    pub recipient_window_caps: Vec<RecipientCap>,
    /// Denied SAC transfer destinations. Checked before the allowlist and
    /// before `allow_any_recipient`; an empty list leaves behavior unchanged.
    pub blocked_recipients: Vec<Address>,
    /// Escape hatch: skip the recipient allowlist (caps still apply).
    pub allow_any_recipient: bool,
    /// Active window start (unix seconds); 0 = unrestricted.
    pub active_from: u64,
    /// Active window end (unix seconds); 0 = unrestricted.
    pub active_until: u64,
    /// Admin kill switch.
    pub paused: bool,
    /// Dead-man switch grace (seconds); 0 = disabled.
    pub dms_grace_secs: u64,
    /// Maximum protocol (non-SAC allowlisted) calls per rolling window; 0 = disabled.
    pub protocol_calls_per_window: u32,
}

/// Manual `Debug` implementation for `PolicyConfig` with stable field order.
///
/// Field order is the declaration order (as of SPEC §3 table) and must not be
/// changed without updating the snapshot test in `tests/debug_policy_config.rs`.
/// This is the *human-readable* format for logs, test fixtures, and dashboard
/// inspect scripts — it is NOT the canonical encoding for `policy_hash`.
/// Canonical encoding for hashing must be a separate, unambiguous serialization
/// (e.g., XDR with deterministic field tags); see SPEC §8/§9 discussion.
impl core::fmt::Debug for PolicyConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PolicyConfig")
            .field("per_tx_cap", &self.per_tx_cap)
            .field("window_secs", &self.window_secs)
            .field("window_cap", &self.window_cap)
            .field("assets", &self.assets)
            .field("protocols", &self.protocols)
            .field("recipients", &self.recipients)
            .field("recipient_window_caps", &self.recipient_window_caps)
            .field("blocked_recipients", &self.blocked_recipients)
            .field("allow_any_recipient", &self.allow_any_recipient)
            .field("active_from", &self.active_from)
            .field("active_until", &self.active_until)
            .field("paused", &self.paused)
            .field("dms_grace_secs", &self.dms_grace_secs)
            .field("protocol_calls_per_window", &self.protocol_calls_per_window)
            .finish()
    }
}

#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
/// A protocol contract and optional function allowlist.
pub struct ProtocolRule {
    /// Contract permitted for non-SAC calls.
    pub contract: Address,
    /// `None` = any function; `Some` = per-function allowlist.
    pub fns: Option<Vec<Symbol>>,
}

/// A single call the account must authorize, parsed into a form the pure
/// decision engine can reason about without touching `Env`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedCall {
    /// A known SAC transfer on an allowlisted asset — fully enforceable.
    AssetTransfer {
        asset: Address,
        to: Address,
        amount: i128,
    },
    /// A call on an allowlisted asset that is not `transfer`/`transfer_from`
    /// (e.g. `mint`, `burn`) — never allowed for the account as authorizer.
    AssetOther { asset: Address, fname: Symbol },
    /// A call on an allowlisted protocol contract.
    Protocol { contract: Address, fname: Symbol },
    /// A call to this account's own functions (e.g. `heartbeat`).
    SelfCall { fname: Symbol },
    /// A host-function contract creation authorized by the account — denied in
    /// v1 (an account that may not call unknown contracts should not create them).
    CreateContract,
    /// Anything else — default deny.
    Unknown { contract: Address, fname: Symbol },
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// Whether a policy is currently installed.
    pub has_policy: bool,
    /// Monotonic revision incremented by policy install/revoke.
    pub policy_revision: u64,
    /// Whether the administrator has frozen the account.
    pub admin_frozen: bool,
    /// Whether the configured heartbeat grace has elapsed.
    pub heartbeat_expired: bool,
    /// Unix timestamp of the last heartbeat, or zero if none.
    pub last_heartbeat: u64,
    /// Ledger unix timestamp used for this snapshot.
    pub now: u64,
}

/// Permissionless policy decision returned by `check`.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    /// The target transfer passes the current policy snapshot.
    Allowed,
    /// The transfer is blocked with a stable reason symbol.
    Blocked(Symbol),
}

impl Error {
    /// Convert an `Error` variant into its corresponding `BlockReason` symbol (as used in `CheckResult::Blocked`).
    #[allow(clippy::must_use_candidate)]
    pub fn to_block_reason(self) -> Symbol {
        // Uses the existing reason() string which matches SPEC §7 / reason glossary.
        Symbol::new(&soroban_sdk::Env::default(), self.reason())
    }

    /// Attempt to convert a `BlockReason` symbol back to an `Error` variant.
    #[allow(clippy::must_use_candidate)]
    pub fn from_block_reason(symbol: &Symbol) -> Option<Self> {
        let env = soroban_sdk::Env::default();
        let all_errors = [
            Self::Unauthorized,
            Self::AlreadyInitialized,
            Self::NotInitialized,
            Self::InvalidConfig,
            Self::InvalidAmount,
            Self::AdminFrozen,
            Self::HeartbeatExpired,
            Self::NoPolicy,
            Self::Paused,
            Self::OutsideActiveWindow,
            Self::AssetNotAllowed,
            Self::RecipientNotAllowed,
            Self::RecipientBlocked,
            Self::PerTxCapExceeded,
            Self::WindowCapExceeded,
            Self::ProtocolNotAllowed,
            Self::FunctionNotAllowed,
            Self::UnknownContract,
            Self::SelfFunctionNotAllowed,
            Self::CreateContractNotAllowed,
            Self::ProtocolCallRateExceeded,
        ];
        all_errors
            .into_iter()
            .find(|&err| symbol == &Symbol::new(&env, err.reason()))
    }
}

/// Advisory result for a targeted asset transfer. All fields are calculated
/// from the current policy and window snapshot; this type never represents a
/// storage mutation.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
/// Advisory outcome and cap headroom for a targeted transfer check.
pub struct CheckDetail {
    /// Policy decision for the requested transfer.
    pub result: CheckResult,
    /// Remaining global or recipient window allowance, if enabled.
    pub remaining_window: Option<i128>,
    /// Configured per-transfer cap, if enabled.
    pub per_tx_cap: Option<i128>,
    /// Effective per-transfer cap for this recipient, if enabled.
    pub effective_per_tx_cap: Option<i128>,
    /// Effective rolling cap for this recipient, if enabled.
    pub effective_window_cap: Option<i128>,
}

// Storage layout (SPEC §3). `Initialized`/`Admin`/`AgentPubkey` live in
// instance storage (auto-TTL on every invocation); the rest live in
// persistent storage with TTL extensions on writes and thresholded refreshes
// on reads.
#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    /// Instance: one-time flag for `initialize`.
    Initialized,
    /// Instance: policy admin; set once at `initialize`.
    Admin,
    /// Instance: the registered agent's Ed25519 public key (32 bytes).
    AgentPubkey,
    /// Persistent: current policy (`None` = default-deny).
    Policy,
    /// Persistent: rolling spend ledger for asset transfers.
    Window,
    /// Persistent: unix seconds of last agent heartbeat (0 = never).
    LastHeartbeat,
    /// Persistent: admin-initiated freeze flag.
    AdminFrozen,
    /// Persistent: incrementing counter for policy changes.
    PolicyRevision,
}

/// Stable contract errors and decision reasons exposed by the ABI.
#[allow(missing_docs)] // individual ABI variants are described below
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // Generic / lifecycle (1..=9)
    /// Required signer did not authorize the operation.
    Unauthorized = 1,
    /// Initialization has already been completed.
    AlreadyInitialized = 2,
    /// Required contract state has not been initialized.
    NotInitialized = 3,
    /// Policy configuration violates a validation rule.
    InvalidConfig = 4,
    /// Requested transfer amount is invalid.
    InvalidAmount = 5,
    // Account-level gates (10..=19)
    /// Admin emergency freeze is active.
    AdminFrozen = 10,
    /// Agent heartbeat grace period has elapsed.
    HeartbeatExpired = 11,
    /// No policy is installed (default-deny).
    NoPolicy = 12,
    /// Policy is administratively paused.
    Paused = 13,
    /// Current ledger time is outside the configured active interval.
    OutsideActiveWindow = 14,
    // Per-call decisions (20..=29)
    /// Transfer asset is absent from the asset allowlist.
    AssetNotAllowed = 20,
    /// Transfer destination is absent from the recipient allowlist.
    RecipientNotAllowed = 21,
    /// Transfer exceeds its per-call cap.
    PerTxCapExceeded = 22,
    /// Transfer exceeds a rolling-window cap.
    WindowCapExceeded = 23,
    /// Called protocol contract is not allowlisted.
    ProtocolNotAllowed = 24,
    /// Called function is not allowlisted for its protocol.
    FunctionNotAllowed = 25,
    /// Call targets an unknown contract.
    UnknownContract = 26,
    /// Account self-call is not permitted by the fixed self-call policy.
    SelfFunctionNotAllowed = 27,
    /// Account-authorized contract creation is disabled.
    CreateContractNotAllowed = 28,
    /// Transfer destination is explicitly blocked.
    RecipientBlocked = 29,
    /// Rolling protocol-call count would exceed its configured limit.
    ProtocolCallRateExceeded = 30,
}

impl Error {
    /// Stable, human- and telemetry-readable reason name (no env needed).
    #[allow(clippy::must_use_candidate)]
    pub fn reason(self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::AlreadyInitialized => "already_initialized",
            Self::NotInitialized => "not_initialized",
            Self::InvalidConfig => "invalid_config",
            Self::InvalidAmount => "invalid_amount",
            Self::AdminFrozen => "admin_frozen",
            Self::HeartbeatExpired => "heartbeat_expired",
            Self::NoPolicy => "no_policy",
            Self::Paused => "paused",
            Self::OutsideActiveWindow => "outside_active_window",
            Self::AssetNotAllowed => "asset_not_allowed",
            Self::RecipientNotAllowed => "recipient_not_allowed",
            Self::RecipientBlocked => "recipient_blocked",
            Self::PerTxCapExceeded => "per_tx_cap_exceeded",
            Self::WindowCapExceeded => "window_cap_exceeded",
            Self::ProtocolNotAllowed => "protocol_not_allowed",
            Self::FunctionNotAllowed => "function_not_allowed",
            Self::UnknownContract => "unknown_contract",
            Self::SelfFunctionNotAllowed => "self_function_not_allowed",
            Self::CreateContractNotAllowed => "create_contract_not_allowed",
            Self::ProtocolCallRateExceeded => "protocol_call_rate_exceeded",
        }
    }
}
