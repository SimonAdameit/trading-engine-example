use anyhow::{ensure, Context, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{AddAssign, SubAssign};
use TransactionType::*;

#[derive(Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
#[serde(transparent)]
pub struct ClientId(u16);

#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Hash, Debug)]
#[serde(transparent)]
pub struct TransactionId(u32);

#[derive(Serialize, Deserialize, Eq, PartialEq, PartialOrd, Copy, Clone, Debug)]
#[serde(transparent)]
pub struct Amount(#[serde(with = "rust_decimal::serde::str")] Decimal);

impl Amount {
    const ZERO: Self = Amount(Decimal::ZERO);

    fn normalize(&self) -> Self {
        Self(self.0.normalize())
    }
}

impl AddAssign<&Self> for Amount {
    fn add_assign(&mut self, rhs: &Self) {
        self.0.add_assign(rhs.0)
    }
}

impl SubAssign<&Self> for Amount {
    fn sub_assign(&mut self, rhs: &Self) {
        self.0.sub_assign(rhs.0)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Option<Amount>,
}

pub struct Account {
    client: ClientId,
    available: Amount,
    held: Amount,
    total: Amount,
    locked: bool,
    transactions: HashMap<TransactionId, AccountTransaction>,
}

#[derive(Serialize, Eq, PartialEq, Clone, Debug)]
pub struct AccountInfo {
    pub client: ClientId,
    pub available: Amount,
    pub held: Amount,
    pub total: Amount,
    pub locked: bool,
}

struct AccountTransaction {
    transaction: Transaction,
    state: TransactionState,
}

enum TransactionState {
    Failed,
    Executed { dispute: DisputeState },
}
impl TransactionState {
    fn executed() -> Self {
        Self::Executed {
            dispute: DisputeState::Undisputed,
        }
    }
}

enum DisputeState {
    Undisputed,
    Disputed,
    Resolved,
    Chargeback,
}

impl Account {
    pub fn new(client: ClientId) -> Account {
        Self {
            client,
            available: Amount::ZERO,
            held: Amount::ZERO,
            total: Amount::ZERO,
            locked: false,
            transactions: HashMap::new(),
        }
    }

    pub fn info(&self) -> AccountInfo {
        AccountInfo {
            client: self.client,
            available: self.available.normalize(),
            held: self.held.normalize(),
            total: self.total.normalize(),
            locked: self.locked,
        }
    }

    pub fn handle(&mut self, transaction: Transaction) -> Result<()> {
        ensure!(self.client == transaction.client, "transaction is for this account");
        if !self.locked {
            match transaction.transaction_type {
                Deposit => self.deposit(transaction)?,
                Withdrawal => self.withdrawal(transaction)?,
                Dispute => self.dispute(transaction)?,
                Resolve => self.resolve(transaction)?,
                Chargeback => self.chargeback(transaction)?,
            }
        }
        Ok(())
    }

    fn deposit(&mut self, transaction: Transaction) -> Result<()> {
        let amount = &transaction.amount.context("Deposit requires amount")?;
        let tx = transaction.tx;
        self.available += amount;
        self.total += amount;
        let state = TransactionState::executed();
        self.transactions.insert(tx, AccountTransaction { transaction, state });
        Ok(())
    }

    fn withdrawal(&mut self, transaction: Transaction) -> Result<()> {
        let amount = &transaction.amount.context("Withdrawal requires amount")?;
        let tx = transaction.tx;
        let state = if &self.available >= amount {
            self.available -= amount;
            self.total -= amount;
            TransactionState::executed()
        } else {
            TransactionState::Failed
        };
        self.transactions.insert(tx, AccountTransaction { transaction, state });
        Ok(())
    }

    fn dispute(&mut self, transaction: Transaction) -> Result<()> {
        if let Some(AccountTransaction {
            transaction:
                Transaction {
                    transaction_type: Deposit,
                    amount: Some(amount),
                    ..
                },
            state:
                TransactionState::Executed {
                    dispute: dispute @ (DisputeState::Undisputed | DisputeState::Resolved),
                },
        }) = self.transactions.get_mut(&transaction.tx)
        {
            *dispute = DisputeState::Disputed;
            self.available -= amount;
            self.held += amount;
        }
        Ok(())
    }

    fn resolve(&mut self, transaction: Transaction) -> Result<()> {
        if let Some(AccountTransaction {
            transaction:
                Transaction {
                    transaction_type: Deposit,
                    amount: Some(amount),
                    ..
                },
            state: TransactionState::Executed {
                dispute: dispute @ DisputeState::Disputed,
            },
        }) = self.transactions.get_mut(&transaction.tx)
        {
            *dispute = DisputeState::Resolved;
            self.available += amount;
            self.held -= amount;
        }
        Ok(())
    }

    fn chargeback(&mut self, transaction: Transaction) -> Result<()> {
        if let Some(AccountTransaction {
            transaction:
                Transaction {
                    transaction_type: Deposit,
                    amount: Some(amount),
                    ..
                },
            state: TransactionState::Executed {
                dispute: dispute @ DisputeState::Disputed,
            },
        }) = self.transactions.get_mut(&transaction.tx)
        {
            *dispute = DisputeState::Chargeback;
            self.locked = true;
            self.held -= amount;
            self.total -= amount;
        }
        Ok(())
    }
}
