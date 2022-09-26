use anyhow::{bail, Error};
use rust_decimal::Decimal;

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::ids::TxId;

/// Things we need to record incase they are disputed
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordType {
    Deposit,
    Withdrawal,
}

/// Record of a transaction in case of dispute
#[derive(Debug, Eq, PartialEq)]
pub struct TranRecord {
    rec_type: RecordType,
    amount: Decimal,
    disputed: bool,
}

impl TranRecord {
    pub fn new(rec_type: RecordType, amount: Decimal) -> Self {
        Self {
            rec_type,
            amount,
            disputed: false,
        }
    }
}

/// Holds the balances for one client asset
#[derive(Debug, Default)]
pub struct Balance {
    available: Decimal,
    held: Decimal,
    locked: bool,
    trans: HashMap<TxId, TranRecord>,
}

impl Balance {
    pub fn deposit(&mut self, tx: TxId, amount: Decimal) -> Result<(), Error> {
        if amount <= Decimal::ZERO {
            bail!("invalid amount {}", amount);
        }
        if !self.locked {
            let old = self
                .trans
                .insert(tx, TranRecord::new(RecordType::Deposit, amount));
            if let Some(old) = old {
                self.trans.insert(tx, old);
                bail!("Duplicate transaction {:?}", tx);
            }
            self.available += amount;
        }
        Ok(())
    }

    pub fn withdraw(&mut self, tx: TxId, amount: Decimal) -> Result<(), Error> {
        if amount <= Decimal::ZERO {
            bail!("invalid amount {}", amount);
        }
        if !self.locked && self.available >= amount {
            let old = self
                .trans
                .insert(tx, TranRecord::new(RecordType::Withdrawal, amount));
            if let Some(old) = old {
                self.trans.insert(tx, old);
                bail!("Duplicate transaction {:?}", tx);
            }
            self.available -= amount;
        }
        Ok(())
    }

    pub fn dispute(&mut self, tx: TxId) -> Result<(), Error> {
        if self.locked {
            return Ok(());
        }
        let record = self.trans.get_mut(&tx);
        if let Some(record) = record {
            match (record.rec_type, record.disputed) {
                (RecordType::Deposit, false) => {
                    self.available -= record.amount;
                    self.held += record.amount;
                    record.disputed = true;
                }
                (RecordType::Withdrawal, false) => {
                    self.held -= record.amount;
                    record.disputed = true;
                }
                // Already disputed
                (_, true) => (),
            }
            Ok(())
        } else {
            // Unknown TxId, assume payment partner error
            Ok(())
        }
    }

    pub fn resolve(&mut self, tx: TxId) -> Result<(), Error> {
        if self.locked {
            return Ok(());
        }
        let record = self.trans.get_mut(&tx);
        if let Some(record) = record {
            match (record.rec_type, record.disputed) {
                (RecordType::Deposit, true) => {
                    self.available += record.amount;
                    self.held -= record.amount;
                    record.disputed = false;
                }
                (RecordType::Withdrawal, true) => {
                    self.held += record.amount;
                    record.disputed = false;
                }
                // Not disputed, ignore
                (_, false) => (),
            }
            Ok(())
        } else {
            // Unknown TxId, assume payment partner error
            Ok(())
        }
    }

    pub fn chargeback(&mut self, tx: TxId) -> Result<(), Error> {
        if self.locked {
            return Ok(());
        }
        let record = self.trans.get_mut(&tx);
        if let Some(record) = record {
            match (record.rec_type, record.disputed) {
                (RecordType::Deposit, true) => {
                    self.held -= record.amount;
                    record.disputed = false;
                    self.locked = true;
                }
                (RecordType::Withdrawal, true) => {
                    self.available += record.amount;
                    self.held += record.amount;
                    record.disputed = false;
                    self.locked = true;
                }
                // Not disputed, ignore
                (_, false) => (),
            }
            Ok(())
        } else {
            // Unknown TxId, assume payment partner error
            Ok(())
        }
    }
}

impl Display for Balance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            self.available,
            self.held,
            self.available + self.held,
            self.locked
        )
    }
}

#[test]
fn test_dispute_deposit() -> Result<(), Error> {
    use rust_decimal_macros::dec;
    let mut balance = Balance::default();

    balance.deposit(TxId(1), dec!(10.0))?;
    balance.withdraw(TxId(2), dec!(7.0))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    balance.dispute(TxId(1))?;
    assert_eq!(balance.available, dec!(-7.0));
    assert_eq!(balance.held, dec!(10.0));
    assert_eq!(balance.locked, false);

    balance.resolve(TxId(1))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    // second resolve should have no effect
    balance.resolve(TxId(1))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    Ok(())
}

#[test]
fn test_dispute_withdrawal() -> Result<(), Error> {
    use rust_decimal_macros::dec;
    let mut balance = Balance::default();

    balance.deposit(TxId(1), dec!(10.0))?;
    assert_eq!(balance.available, dec!(10.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    // resolving an undisputed transaction should have no effect
    balance.resolve(TxId(2))?;
    assert_eq!(balance.available, dec!(10.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    balance.withdraw(TxId(2), dec!(7.0))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    balance.dispute(TxId(2))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(-7.0));
    assert_eq!(balance.locked, false);

    balance.resolve(TxId(2))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    // second resolve should have no effect
    balance.resolve(TxId(2))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    Ok(())
}

#[test]
fn test_chargeback_deposit() -> Result<(), Error> {
    use rust_decimal_macros::dec;
    let mut balance = Balance::default();

    balance.deposit(TxId(1), dec!(10.0))?;
    balance.withdraw(TxId(2), dec!(7.0))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    balance.dispute(TxId(1))?;
    assert_eq!(balance.available, dec!(-7.0));
    assert_eq!(balance.held, dec!(10.0));
    assert_eq!(balance.locked, false);

    balance.chargeback(TxId(1))?;
    assert_eq!(balance.available, dec!(-7.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, true);

    // second chargeback should have no effect
    balance.chargeback(TxId(1))?;
    assert_eq!(balance.available, dec!(-7.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, true);

    Ok(())
}

#[test]
fn test_chargeback_withdrawal() -> Result<(), Error> {
    use rust_decimal_macros::dec;
    let mut balance = Balance::default();

    balance.deposit(TxId(1), dec!(10.0))?;
    assert_eq!(balance.available, dec!(10.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    balance.withdraw(TxId(2), dec!(7.0))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, false);

    balance.dispute(TxId(2))?;
    assert_eq!(balance.available, dec!(3.0));
    assert_eq!(balance.held, dec!(-7.0));
    assert_eq!(balance.locked, false);

    balance.chargeback(TxId(2))?;
    assert_eq!(balance.available, dec!(10.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, true);

    // second chargeback should have no effect
    balance.chargeback(TxId(2))?;
    assert_eq!(balance.available, dec!(10.0));
    assert_eq!(balance.held, dec!(0.0));
    assert_eq!(balance.locked, true);

    Ok(())
}

#[test]
fn test_deposit_withdraw() -> Result<(), Error> {
    use rust_decimal_macros::dec;
    let mut balance = Balance::default();

    // try withdraw from empty balance
    balance.withdraw(TxId(1), dec!(5.00))?;
    assert_eq!(balance.available, dec!(0));
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    assert_eq!(balance.trans.get(&TxId(1)), None);

    // try withdraw of zero
    assert!(balance.withdraw(TxId(2), dec!(0)).is_err());
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    assert_eq!(balance.trans.get(&TxId(2)), None);

    // try deposit of zero
    assert!(balance.deposit(TxId(3), dec!(0)).is_err());
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    assert_eq!(balance.trans.get(&TxId(3)), None);

    // deposit in bounds
    balance.deposit(TxId(4), dec!(10.0))?;
    assert_eq!(balance.available, dec!(10.0));
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    assert_eq!(
        balance.trans.get(&TxId(4)),
        Some(&TranRecord::new(RecordType::Deposit, dec!(10.0)))
    );

    // withdraw more than available
    balance.withdraw(TxId(5), dec!(11.0))?;
    assert_eq!(balance.available, dec!(10.0));
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    assert_eq!(balance.trans.get(&TxId(5)), None);

    // withdraw in bounds
    balance.withdraw(TxId(6), dec!(3.0))?;
    assert_eq!(balance.available, dec!(7.0));
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    assert_eq!(
        balance.trans.get(&TxId(6)),
        Some(&TranRecord::new(RecordType::Withdrawal, dec!(3.0)))
    );

    // withdraw in dupe transaction, check its err
    assert!(balance.withdraw(TxId(6), dec!(3.0)).is_err());
    assert_eq!(balance.available, dec!(7.0));
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    // check no change in the transaction record
    assert_eq!(
        balance.trans.get(&TxId(6)),
        Some(&TranRecord::new(RecordType::Withdrawal, dec!(3.0)))
    );

    // deposit in dupe transaction id, check its err
    assert!(balance.deposit(TxId(6), dec!(1.0)).is_err());
    assert_eq!(balance.available, dec!(7.0));
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    // check no change in the transaction record
    assert_eq!(
        balance.trans.get(&TxId(6)),
        Some(&TranRecord::new(RecordType::Withdrawal, dec!(3.0)))
    );

    // withdraw all remaining funds
    balance.withdraw(TxId(7), dec!(7.0))?;
    assert_eq!(balance.available, dec!(0.0));
    assert_eq!(balance.held, dec!(0));
    assert_eq!(balance.locked, false);
    assert_eq!(
        balance.trans.get(&TxId(7)),
        Some(&TranRecord::new(RecordType::Withdrawal, dec!(7.0)))
    );

    Ok(())
}

// #[test]
// fn test_sizeof() {
//     // Uncomment this to get estimate of transaction storage cost
//     // its commented by default to avoid environment dependent failures if underlying crates update or rustc struct layout changes
//     use std::mem::size_of;
//     use std::collections::hash_map::Entry;
//     assert_eq!(32, size_of::<Entry<TxId, TranRecord>>());
// }
