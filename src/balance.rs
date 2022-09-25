use anyhow::{bail, Error};
use rust_decimal::Decimal;

use std::collections::HashMap;

use crate::ids::TxId;

/// Things we need to record incase they are disputed
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecordType {
    Deposit,
    Withdrawal,
}

/// Record of a transaction in case of dispute
#[derive(Debug, PartialEq)]
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

    // pub fn dispute(&mut self, tx: TxId) -> Result<(), Error> {
    //     if self.locked {
    //         return Ok(());
    //     }
    //     // TODO
    //     Ok(())
    // }

    // pub fn resolve(&mut self, _tx: TxId) -> Result<(), Error> {
    //     if self.locked {
    //         return Ok(());
    //     }
    //     // TODO
    //     Ok(())
    // }

    // pub fn chargeback(&mut self, _tx: TxId) -> Result<(), Error> {
    //     if self.locked {
    //         return Ok(());
    //     }
    //     // TODO
    //     Ok(())
    // }
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

    Ok(())
}
