use {
    crate::append_vec::{StoredAccountMeta, StoredMeta},
    solana_sdk::{account::AccountSharedData, clock::Slot},
    std::sync::{Arc, RwLock},
};

pub trait AccountsUpdateNotifierInterface: std::fmt::Debug {
    /// Notified when an account is updated at runtime, due to transaction activities
    fn notify_account_update(&self, slot: Slot, meta: &StoredMeta, account: &AccountSharedData);

    /// Notified when the AccountsDb is initialized at start when restored
    /// from a snapshot.
    fn notify_account_restore_from_snapshot(&self, slot: Slot, account: &StoredAccountMeta);

    /// Notified when all accounts have been notified when restoring from a snapshot.
    fn notify_end_of_restore_from_snapshot(&self);
}

pub type AccountsUpdateNotifier = Arc<RwLock<dyn AccountsUpdateNotifierInterface + Sync + Send>>;
