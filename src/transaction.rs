use anyhow::{bail, Error};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Deserializer;

use crate::ids::{ClientId, TxId};

const MAX_DP: u32 = 4;

/// types of transaction we can process
#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum TranType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// The input transaction
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub tran_type: TranType,
    pub client: ClientId,
    pub tx: TxId,
    pub amount: Option<Decimal>,
}

impl Transaction {
    pub fn new(tran_type: TranType, client: ClientId, tx: TxId, amount: Option<Decimal>) -> Self {
        Self {
            client,
            tx,
            tran_type,
            amount,
        }
    }
}

/// Respect the decimal point limit
fn try_from_str(s: &str) -> Result<Option<Decimal>, Error> {
    let s = s.trim();
    Ok(if s.is_empty() {
        None
    } else {
        if s.chars().next() == Some('.') {
            bail!("leading decimal point not allowed: {}", s);
        }
        let d = Decimal::from_str_exact(s)?;
        if d.is_sign_negative() {
            bail!("negative amount: {}", s);
        } else if d == Decimal::ZERO {
            bail!("zero amount: {}", s);
        } else if d.fract().scale() > MAX_DP {
            bail!("too many decimal places: {}", s);
        }
        Some(d)
    })
}

/// Custom deserializer to enforce max decimal places
fn deserialize_amount<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<String> = Option::deserialize(deserializer)?;
    if let Some(v) = v.as_ref() {
        Ok(try_from_str(v).map_err(serde::de::Error::custom)?)
    } else {
        Ok(None)
    }
}

/// Custom deserializer to enforce invariants on inputs
impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Inner {
            pub client: ClientId,
            pub tx: TxId,
            #[serde(rename = "type")]
            pub tran_type: TranType,
            #[serde(deserialize_with = "deserialize_amount")]
            pub amount: Option<Decimal>,
        }

        // Deserialize the inner struct
        let inner = Inner::deserialize(deserializer)?;

        // Do the additional validation, if it fails return an error
        let amount = match (inner.tran_type, inner.amount) {
            (TranType::Deposit | TranType::Withdrawal, None) => Err(serde::de::Error::custom(
                "amount required for deposit and withdrawal",
            )),
            (TranType::Dispute | TranType::Resolve | TranType::Chargeback, Some(_)) => Err(
                serde::de::Error::custom("amount not allowed for dispute, resolve, or chargeback"),
            ),
            (TranType::Deposit | TranType::Withdrawal, Some(amount)) => Ok(Some(amount)),
            (TranType::Dispute | TranType::Resolve | TranType::Chargeback, None) => Ok(None),
        }?;

        // Return the actual contract
        Ok(Transaction::new(
            inner.tran_type,
            inner.client,
            inner.tx,
            amount,
        ))
    }
}

#[test]
fn test_from_str() -> Result<(), Error> {
    use rust_decimal_macros::dec;

    assert_eq!(try_from_str("")?, None);
    assert_eq!(try_from_str("1.1")?, Some(dec!(1.1)));
    assert_eq!(try_from_str(" 1.1 ")?, Some(dec!(1.1)));

    assert!(try_from_str("0.0").is_err());
    assert!(try_from_str("0").is_err());
    assert!(try_from_str("0.23456").is_err());
    assert!(try_from_str("0.234.56").is_err());
    assert!(try_from_str("0.2345.6").is_err());
    assert!(try_from_str(".2345").is_err());
    assert!(try_from_str("10.23456").is_err());
    assert!(try_from_str("foo").is_err());
    assert!(try_from_str("-1.2345").is_err());
    assert!(try_from_str("-1.23456").is_err());

    assert_eq!(try_from_str("1.2345")?, Some(dec!(1.2345)));
    assert_eq!(try_from_str("0.0001")?, Some(dec!(0.0001)));
    Ok(())
}

#[test]
fn test_deserialize_with_amount() -> Result<(), Error> {
    use csv::StringRecord;
    use rust_decimal_macros::dec;

    let expected = Transaction::new(TranType::Deposit, ClientId(1), TxId(2), Some(dec!(1.1)));

    let h = StringRecord::from(vec!["type", "client", "tx", "amount"]);
    let t = &StringRecord::from_iter("deposit,1,2,1.1".split(","))
        .deserialize::<Transaction>(Some(&h))?;
    assert_eq!(t, &expected);

    // Check header order doesn't matter
    let h = StringRecord::from(vec!["client", "type", "tx", "amount"]);
    let t = &StringRecord::from_iter("1,deposit,2,1.1".split(","))
        .deserialize::<Transaction>(Some(&h))?;
    assert_eq!(t, &expected);

    // Check amount appears ok for withdrawal
    let t = &StringRecord::from_iter("1,withdrawal,2,1.1".split(","))
        .deserialize::<Transaction>(Some(&h))?;
    assert_eq!(
        t,
        &Transaction {
            tran_type: TranType::Withdrawal,
            ..expected
        }
    );

    Ok(())
}

#[test]
fn test_deserialize_no_amount() -> Result<(), Error> {
    use csv::StringRecord;

    let expected = Transaction::new(TranType::Dispute, ClientId(1), TxId(2), None);

    let h = StringRecord::from(vec!["type", "client", "tx", "amount"]);
    let t =
        &StringRecord::from_iter("dispute,1,2,".split(",")).deserialize::<Transaction>(Some(&h))?;
    assert_eq!(t, &expected);

    let t =
        &StringRecord::from_iter("resolve,1,2,".split(",")).deserialize::<Transaction>(Some(&h))?;
    assert_eq!(
        t,
        &Transaction {
            tran_type: TranType::Resolve,
            ..expected
        }
    );

    let t = &StringRecord::from_iter("chargeback,1,2,".split(","))
        .deserialize::<Transaction>(Some(&h))?;
    assert_eq!(
        t,
        &Transaction {
            tran_type: TranType::Chargeback,
            ..expected
        }
    );

    Ok(())
}

#[test]
fn test_deserialize_err() -> Result<(), Error> {
    use csv::StringRecord;

    let h = StringRecord::from(vec!["type", "client", "tx", "amount", "random"]);

    assert!(&StringRecord::from_iter("deposit,1,2,1.1,1".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    let h = StringRecord::from(vec!["type", "client", "tx", "amount"]);

    assert!(&StringRecord::from_iter("depositd,1,2,1.1".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    assert!(&StringRecord::from_iter("deposit,1,2,".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    assert!(&StringRecord::from_iter("deposit,1,2".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    assert!(&StringRecord::from_iter("withdrawal,1,2".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    assert!(&StringRecord::from_iter("dispute,1,2,1.0".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    assert!(&StringRecord::from_iter("resolve,1,2,1.0".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    assert!(&StringRecord::from_iter("chargeback,1,2,1.0".split(","))
        .deserialize::<Transaction>(Some(&h))
        .is_err());

    Ok(())
}
