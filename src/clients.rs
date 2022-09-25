use anyhow::{bail, Error};

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::balance::Balance;
use crate::ids::ClientId;
use crate::transaction::{TranType, Transaction};

/// Represents a collection of clients and allows us to process a transaction
#[derive(Debug, Default)]
pub struct Clients {
    pub balance_map: HashMap<ClientId, Balance>,
}

impl Clients {
    pub fn process(&mut self, t: Transaction) -> Result<(), Error> {
        let e = self.balance_map.entry(t.client);
        match (t.tran_type, e, t.amount) {
            (TranType::Deposit, e, Some(amount)) => e.or_default().deposit(t.tx, amount),
            (TranType::Withdrawal, e, Some(amount)) => e.or_default().withdraw(t.tx, amount),
            (TranType::Deposit, _, None) | (TranType::Withdrawal, _, None) => {
                bail!("Invalid transaction, missing amount for {:?}", t)
            }

            (TranType::Dispute, Entry::Occupied(mut e), _) => e.get_mut().dispute(t.tx),
            (TranType::Resolve, Entry::Occupied(mut e), _) => e.get_mut().resolve(t.tx),
            (TranType::Chargeback, Entry::Occupied(mut e), _) => e.get_mut().chargeback(t.tx),

            // partner error, the client for dispute doesn't exist, ignore
            (
                TranType::Dispute | TranType::Resolve | TranType::Chargeback,
                Entry::Vacant(_),
                None,
            ) => Ok(()),

            (_, _, Some(_)) => bail!("Invalid transaction, was not expeciting amount for {:?}", t),
        }
    }
}

impl Display for Clients {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // get a stable order for the clients so we can compare test data
        let mut keys: Vec<ClientId> = self.balance_map.keys().cloned().collect();
        keys.sort();
        for client in keys {
            let balance = self.balance_map.get(&client).unwrap();
            writeln!(f, "{},{}", client.id(), balance)?
        }
        Ok(())
    }
}

#[test]
fn test_process() -> Result<(), Error> {
    use crate::ids::TxId;
    use rust_decimal_macros::dec;

    let mut clients = Clients::default();
    clients.process(Transaction::new(
        TranType::Deposit,
        ClientId(1),
        TxId(1),
        Some(dec!(1.00)),
    ))?;
    assert!(clients.balance_map.get(&ClientId(1)).is_some());

    let t = Transaction::new(TranType::Deposit, ClientId(2), TxId(2), Some(dec!(1.00)));
    clients.process(t)?;
    assert!(clients.balance_map.get(&ClientId(2)).is_some());

    let t = Transaction::new(TranType::Withdrawal, ClientId(2), TxId(3), Some(dec!(1.00)));
    clients.process(t)?;
    assert!(clients.balance_map.get(&ClientId(2)).is_some());

    // Unknown client cases. partner error, ignore and check no client record is created
    let t = Transaction::new(TranType::Dispute, ClientId(99), TxId(2), None);
    assert!(clients.process(t).is_ok());
    assert!(clients.balance_map.get(&ClientId(99)).is_none());

    let t = Transaction::new(TranType::Resolve, ClientId(99), TxId(2), None);
    assert!(clients.process(t).is_ok());
    assert!(clients.balance_map.get(&ClientId(99)).is_none());

    let t = Transaction::new(TranType::Chargeback, ClientId(99), TxId(2), None);
    assert!(clients.process(t).is_ok());
    assert!(clients.balance_map.get(&ClientId(99)).is_none());

    let d = clients.to_string();
    let expected = "1,1.00,0,1.00,false
2,0.00,0,0,false
";
    assert_eq!(d, expected);

    Ok(())
}
