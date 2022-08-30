use std::collections::{
    btree_map::{BTreeMap, Entry},
    BTreeSet,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    TransactionAlreadyExists(u32),
    TransactionUnknown(u32),
    TransactionUndisputed(u32),
    TransactionAlreadyDisputed(u32),
    InsufficientFunds,
    NegativeAmount(i64),
    Locked,
}

impl std::error::Error for Error {}

pub type Result = std::result::Result<(), Error>;

// Could alternatively use ThisError crate
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error!") // fixme
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Account {
    /**
     * The total funds that are available for trading, staking, withdrawal, etc.
     *
     * Could be calculated on-demand from the log but stored here for efficient retrieval.
     */
    available: i64,
    /**
     * The total funds that are held for dispute.
     *
     * Could be calculated on-demand from the log but stored here for efficient retrieval.
     */
    held: i64,
    /**
     * Whether the account is locked.
     */
    locked: bool,
    /**
     * The log of deposits and withdrawels. We use i64 throughout in order to avoid conversions.
     *
     * This should eventually become bound either by size or some kind of age threshold.
     */
    log: BTreeMap<u32, i64>,
    /**
     * Set of disputed transactions. Could also be an attribute inside the transaction log but as
     * the number of disputes should stay small we don't waste space on every log entry.
     */
    disputes: BTreeSet<u32>,
}

impl Account {
    pub fn new() -> Account {
        Account {
            available: 0,
            held: 0,
            locked: false,
            log: BTreeMap::new(),
            disputes: BTreeSet::new(),
        }
    }

    /**
     * The total funds that are available or held.
     */
    pub fn total(&self) -> i64 {
        self.available + self.held
    }

    fn tx(&mut self, tx: u32, amount: i64) -> Result {
        match self.log.entry(tx) {
            Entry::Occupied(_) => Err(Error::TransactionAlreadyExists(tx)),
            Entry::Vacant(entry) => {
                entry.insert(amount);
                self.available += amount;
                Ok(())
            }
        }
    }

    /**
     * A deposit is a credit to the client's asset account, meaning it should increase the
     * available and total funds of the client account.
     *
     * Although the amount type is signed we only allow positive values.
     */
    pub fn deposit(&mut self, tx: u32, amount: i64) -> Result {
        if self.locked {
            return Err(Error::Locked);
        }
        if amount < 0 {
            return Err(Error::NegativeAmount(amount));
        }
        self.tx(tx, amount)
    }

    /**
     * A withdraw is a debit to the client's asset account, meaning it should decrease the
     * available and total funds of the client account.
     *
     * Although the amount type is signed we only allow positive values.
     */
    pub fn withdraw(&mut self, tx: u32, amount: i64) -> Result {
        if self.locked {
            return Err(Error::Locked);
        }
        if amount < 0 {
            // If a client does not have sufficient available funds the withdrawal should fail and
            // the total amount of funds should not change
            return Err(Error::NegativeAmount(amount));
        }
        if self.available < amount {
            return Err(Error::InsufficientFunds);
        }
        self.tx(tx, -amount)
    }

    /**
     * A dispute represents a client's claim that a transaction was erroneous and should be
     * reversed. The transaction shouldn't be reversed yet but the associated funds should be held.
     */
    pub fn dispute(&mut self, tx: u32) -> Result {
        if self.locked {
            return Err(Error::Locked);
        }
        match self.log.entry(tx) {
            Entry::Vacant(_) => Err(Error::TransactionUnknown(tx)),
            Entry::Occupied(entry) => {
                if self.disputes.contains(&tx) {
                    Err(Error::TransactionAlreadyDisputed(tx))
                } else {
                    self.disputes.insert(tx);
                    let amount = entry.get();
                    // available funds should decrease by the amount disputed
                    self.available -= amount;
                    // held funds should increase by the amount disputed
                    self.held += amount;
                    Ok(())
                }
            }
        }
    }

    /**
     * A resolve represents a resolution to a dispute, releasing the associated held funds.
     */
    pub fn resolve(&mut self, tx: u32) -> Result {
        if self.locked {
            return Err(Error::Locked);
        }
        match self.log.entry(tx) {
            Entry::Vacant(_) => Err(Error::TransactionUnknown(tx)),
            Entry::Occupied(entry) => {
                if !self.disputes.contains(&tx) {
                    Err(Error::TransactionUndisputed(tx))
                } else {
                    // Funds that were previously disputed are no longer disputed.
                    self.disputes.remove(&tx);
                    let amount = entry.get();
                    // available funds should increase by the amount no longer disputed
                    self.available += amount;
                    // held funds should decrease by the amount no longer disputed
                    self.held -= amount;
                    Ok(())
                }
            }
        }
    }

    pub fn chargeback(&mut self, tx: u32) -> Result {
        if self.locked {
            return Err(Error::Locked);
        }
        match self.log.entry(tx) {
            Entry::Vacant(_) => Err(Error::TransactionUnknown(tx)),
            Entry::Occupied(entry) => {
                if !self.disputes.contains(&tx) {
                    Err(Error::TransactionUndisputed(tx))
                } else {
                    self.disputes.remove(&tx);
                    let amount = entry.get();
                    self.held -= amount;
                    self.locked = true;
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        let account = Account::new();
        assert_eq!(
            account,
            Account {
                available: 0,
                held: 0,
                locked: false,
                log: BTreeMap::new(),
                disputes: BTreeSet::new()
            }
        );
        assert_eq!(account.total(), 0)
    }

    #[test]
    fn deposit() {
        let mut account = Account::new();

        assert!(account.deposit(0, 5).is_ok());
        assert_eq!(
            &account,
            &Account {
                available: 5,
                held: 0,
                locked: false,
                log: [(0, 5)].into_iter().collect(),
                disputes: BTreeSet::new()
            }
        );
        assert_eq!(account.total(), 5);

        assert!(account.deposit(1, 3).is_ok());
        assert_eq!(
            &account,
            &Account {
                available: 8,
                held: 0,
                locked: false,
                log: [(0, 5), (1, 3)].into_iter().collect(),
                disputes: BTreeSet::new()
            }
        );
        assert_eq!(account.total(), 8);

        assert_eq!(
            account.deposit(0, 1).unwrap_err(),
            Error::TransactionAlreadyExists(0)
        );
        assert_eq!(
            account.deposit(2, -1).unwrap_err(),
            Error::NegativeAmount(-1)
        );
        assert_eq!(account.total(), 8)
    }

    #[test]
    fn withdraw() {
        let mut account = Account::new();

        account.deposit(0, 5).unwrap();
        assert_eq!(account.total(), 5);

        assert!(account.withdraw(1, 3).is_ok());
        assert_eq!(
            &account,
            &Account {
                available: 2,
                held: 0,
                locked: false,
                log: [(0, 5), (1, -3)].into_iter().collect(),
                disputes: BTreeSet::new()
            }
        );
        assert_eq!(account.total(), 2);

        assert_eq!(
            account.withdraw(0, 1).unwrap_err(),
            Error::TransactionAlreadyExists(0)
        );
        assert_eq!(
            account.withdraw(2, 3).unwrap_err(),
            Error::InsufficientFunds
        );

        assert_eq!(
            account.withdraw(2, -1).unwrap_err(),
            Error::NegativeAmount(-1)
        );
        assert_eq!(account.log.len(), 2);
        assert_eq!(account.total(), 2)
    }

    #[test]
    fn dispute() {
        let mut account = Account::new();

        account.deposit(0, 5).unwrap();
        account.withdraw(1, 3).unwrap();

        assert_eq!(account.total(), 2);

        assert!(account.dispute(0).is_ok());
        assert_eq!(
            &account,
            &Account {
                available: -3,
                held: 5,
                locked: false,
                log: [(0, 5), (1, -3)].into_iter().collect(),
                disputes: [0].into_iter().collect()
            }
        );
        assert_eq!(account.total(), 2);

        assert!(account.dispute(1).is_ok());
        assert_eq!(
            &account,
            &Account {
                available: 0,
                held: 2,
                locked: false,
                log: [(0, 5), (1, -3)].into_iter().collect(),
                disputes: [0, 1].into_iter().collect()
            }
        );
        assert_eq!(account.total(), 2);

        assert_eq!(
            account.dispute(2).unwrap_err(),
            Error::TransactionUnknown(2)
        );
        assert_eq!(
            account.dispute(0).unwrap_err(),
            Error::TransactionAlreadyDisputed(0)
        );

        assert_eq!(account.total(), 2)
    }

    #[test]
    fn resolve() {
        let mut account = Account::new();
        account.deposit(0, 5).unwrap();
        account.dispute(0).unwrap();

        assert!(account.resolve(0).is_ok());
        assert_eq!(
            &account,
            &Account {
                available: 5,
                held: 0,
                locked: false,
                log: [(0, 5)].into_iter().collect(),
                disputes: BTreeSet::new()
            }
        );
        assert_eq!(account.total(), 5);

        assert_eq!(
            account.resolve(1).unwrap_err(),
            Error::TransactionUnknown(1)
        );
        assert_eq!(
            account.resolve(0).unwrap_err(),
            Error::TransactionUndisputed(0)
        );
        assert_eq!(account.total(), 5);
    }

    #[test]
    fn chargeback() {
        let mut account = Account::new();

        assert_eq!(
            account.chargeback(0).unwrap_err(),
            Error::TransactionUnknown(0)
        );

        account.deposit(0, 5).unwrap();

        assert_eq!(
            account.chargeback(0).unwrap_err(),
            Error::TransactionUndisputed(0)
        );

        account.dispute(0).unwrap();

        assert!(account.chargeback(0).is_ok());
        assert_eq!(
            &account,
            &Account {
                available: 0,
                held: 0,
                locked: true,
                log: [(0, 5)].into_iter().collect(),
                disputes: BTreeSet::new()
            }
        );
        assert_eq!(account.total(), 0);

        assert_eq!(account.deposit(1, 1).unwrap_err(), Error::Locked);
        assert_eq!(account.withdraw(1, 1).unwrap_err(), Error::Locked);
        assert_eq!(account.dispute(1).unwrap_err(), Error::Locked);
        assert_eq!(account.resolve(1).unwrap_err(), Error::Locked);
        assert_eq!(account.chargeback(1).unwrap_err(), Error::Locked);
        assert_eq!(account.total(), 0);
    }
}
