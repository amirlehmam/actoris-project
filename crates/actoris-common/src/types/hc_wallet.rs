//! HC Wallet - Compute credit management (PFLOP-hours)
//!
//! HC (Harness Credits) represent compute capacity measured in PFLOP-hours.
//! Key characteristics:
//! - 30-day rolling expiration (encourages velocity)
//! - Can be locked in escrow (Delegate primitive)
//! - Tracks available vs locked balance
//! - Version field for optimistic concurrency

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default expiry period in days
pub const EXPIRY_DAYS: i64 = 30;

/// Minimum balance for operations
pub const MIN_BALANCE: Decimal = Decimal::ZERO;

/// Wallet operation errors
#[derive(Debug, Error, Clone, PartialEq)]
pub enum WalletError {
    #[error("Insufficient available balance: required {required}, available {available}")]
    InsufficientBalance {
        required: Decimal,
        available: Decimal,
    },

    #[error("Insufficient locked balance: required {required}, locked {locked}")]
    InsufficientLocked { required: Decimal, locked: Decimal },

    #[error("Amount must be positive")]
    InvalidAmount,

    #[error("Wallet has expired")]
    Expired,

    #[error("Version conflict: expected {expected}, found {found}")]
    VersionConflict { expected: u64, found: u64 },
}

/// HC Wallet for compute credit management
///
/// Each entity in Actoris has an HC wallet that tracks their compute credits.
/// Credits are used to pay for actions and can be earned by providing services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcWallet {
    /// Owner's UnifiedID DID
    pub owner_did: String,

    /// Available balance (PFLOP-hours) - can be spent
    pub available: Decimal,

    /// Locked balance (PFLOP-hours) - in escrow for pending operations
    pub locked: Decimal,

    /// Expiration timestamp (Unix milliseconds)
    /// Balance expires 30 days after last credit
    pub expires_at: i64,

    /// Version for optimistic concurrency control
    pub version: u64,

    /// Timestamp of last modification
    pub updated_at: i64,
}

impl HcWallet {
    /// Create a new empty wallet for an entity
    pub fn new(owner_did: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            owner_did,
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
            expires_at: now + (EXPIRY_DAYS * 24 * 60 * 60 * 1000),
            version: 0,
            updated_at: now,
        }
    }

    /// Create a wallet with initial balance
    pub fn with_balance(owner_did: String, initial_balance: Decimal) -> Self {
        let mut wallet = Self::new(owner_did);
        wallet.available = initial_balance;
        wallet
    }

    /// Total balance (available + locked)
    #[inline]
    pub fn total(&self) -> Decimal {
        self.available + self.locked
    }

    /// Check if wallet has expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp_millis() > self.expires_at
    }

    /// Credit HC to the wallet (extend expiry)
    pub fn credit(&mut self, amount: Decimal) -> Result<(), WalletError> {
        if amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        self.available += amount;
        self.refresh_expiry();
        self.touch();
        Ok(())
    }

    /// Debit HC from available balance
    pub fn debit(&mut self, amount: Decimal) -> Result<(), WalletError> {
        if amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        if self.is_expired() {
            return Err(WalletError::Expired);
        }

        if self.available < amount {
            return Err(WalletError::InsufficientBalance {
                required: amount,
                available: self.available,
            });
        }

        self.available -= amount;
        self.touch();
        Ok(())
    }

    /// Lock HC for escrow (Delegate primitive)
    ///
    /// Moves funds from available to locked, making them unavailable
    /// for regular spending but reserved for a specific operation.
    pub fn lock(&mut self, amount: Decimal) -> Result<(), WalletError> {
        if amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        if self.is_expired() {
            return Err(WalletError::Expired);
        }

        if self.available < amount {
            return Err(WalletError::InsufficientBalance {
                required: amount,
                available: self.available,
            });
        }

        self.available -= amount;
        self.locked += amount;
        self.touch();
        Ok(())
    }

    /// Release HC from escrow back to available
    ///
    /// Called when an escrowed operation is cancelled or the agent
    /// successfully completes the task.
    pub fn release(&mut self, amount: Decimal) -> Result<(), WalletError> {
        if amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        if self.locked < amount {
            return Err(WalletError::InsufficientLocked {
                required: amount,
                locked: self.locked,
            });
        }

        self.locked -= amount;
        self.available += amount;
        self.touch();
        Ok(())
    }

    /// Transfer locked HC to another wallet (escrow completion)
    ///
    /// Called when an escrowed operation completes successfully,
    /// transferring the locked funds to the service provider.
    pub fn transfer_locked(
        &mut self,
        amount: Decimal,
        recipient: &mut HcWallet,
    ) -> Result<(), WalletError> {
        if amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        if self.locked < amount {
            return Err(WalletError::InsufficientLocked {
                required: amount,
                locked: self.locked,
            });
        }

        self.locked -= amount;
        recipient.available += amount;
        recipient.refresh_expiry();

        self.touch();
        recipient.touch();
        Ok(())
    }

    /// Forfeit locked HC (dispute resolution - funds burned or redistributed)
    pub fn forfeit_locked(&mut self, amount: Decimal) -> Result<Decimal, WalletError> {
        if amount <= Decimal::ZERO {
            return Err(WalletError::InvalidAmount);
        }

        if self.locked < amount {
            return Err(WalletError::InsufficientLocked {
                required: amount,
                locked: self.locked,
            });
        }

        self.locked -= amount;
        self.touch();
        Ok(amount) // Returns forfeited amount for redistribution
    }

    /// Refresh expiry timestamp (30 days from now)
    fn refresh_expiry(&mut self) {
        let now = chrono::Utc::now().timestamp_millis();
        self.expires_at = now + (EXPIRY_DAYS * 24 * 60 * 60 * 1000);
    }

    /// Update version and timestamp
    fn touch(&mut self) {
        self.version += 1;
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }

    /// Check and update with optimistic concurrency
    pub fn check_version(&self, expected: u64) -> Result<(), WalletError> {
        if self.version != expected {
            return Err(WalletError::VersionConflict {
                expected,
                found: self.version,
            });
        }
        Ok(())
    }

    /// Calculate remaining time until expiry in days
    pub fn days_until_expiry(&self) -> i64 {
        let now = chrono::Utc::now().timestamp_millis();
        let remaining_ms = self.expires_at - now;
        remaining_ms / (24 * 60 * 60 * 1000)
    }
}

impl std::fmt::Display for HcWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HcWallet(available={}, locked={}, expires_in={}d)",
            self.available,
            self.locked,
            self.days_until_expiry()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_wallet() {
        let wallet = HcWallet::new("did:key:test".to_string());
        assert_eq!(wallet.available, Decimal::ZERO);
        assert_eq!(wallet.locked, Decimal::ZERO);
        assert!(!wallet.is_expired());
    }

    #[test]
    fn test_credit_debit() {
        let mut wallet = HcWallet::new("did:key:test".to_string());

        wallet.credit(dec!(100)).unwrap();
        assert_eq!(wallet.available, dec!(100));

        wallet.debit(dec!(30)).unwrap();
        assert_eq!(wallet.available, dec!(70));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut wallet = HcWallet::new("did:key:test".to_string());
        wallet.credit(dec!(50)).unwrap();

        let result = wallet.debit(dec!(100));
        assert!(matches!(result, Err(WalletError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_lock_release() {
        let mut wallet = HcWallet::new("did:key:test".to_string());
        wallet.credit(dec!(100)).unwrap();

        wallet.lock(dec!(40)).unwrap();
        assert_eq!(wallet.available, dec!(60));
        assert_eq!(wallet.locked, dec!(40));

        wallet.release(dec!(20)).unwrap();
        assert_eq!(wallet.available, dec!(80));
        assert_eq!(wallet.locked, dec!(20));
    }

    #[test]
    fn test_transfer_locked() {
        let mut sender = HcWallet::with_balance("did:key:sender".to_string(), dec!(100));
        let mut recipient = HcWallet::new("did:key:recipient".to_string());

        sender.lock(dec!(50)).unwrap();
        sender.transfer_locked(dec!(50), &mut recipient).unwrap();

        assert_eq!(sender.available, dec!(50));
        assert_eq!(sender.locked, dec!(0));
        assert_eq!(recipient.available, dec!(50));
    }

    #[test]
    fn test_version_increment() {
        let mut wallet = HcWallet::new("did:key:test".to_string());
        let initial_version = wallet.version;

        wallet.credit(dec!(10)).unwrap();
        assert_eq!(wallet.version, initial_version + 1);

        wallet.debit(dec!(5)).unwrap();
        assert_eq!(wallet.version, initial_version + 2);
    }
}
