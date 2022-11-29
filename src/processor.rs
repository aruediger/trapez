use std::collections::btree_map::{BTreeMap, Entry};

use crate::account::{self, Account};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Transaction error for client {client}: `{err}`.")]
    Transaction { client: u16, err: account::Error },
    #[error("Client '{0}' not found.")]
    UnknownClient(u16),
    #[error("Error sending state result.")]
    Send(),
}

#[derive(Debug)]
pub struct State {
    pub client: u16,
    pub available: i64,
    pub held: i64,
    pub total: i64,
    pub locked: bool,
}

#[derive(Debug)]
pub enum Message {
    Deposit {
        client: u16,
        tx: u32,
        amount: i64,
    },
    Withdrawal {
        client: u16,
        tx: u32,
        amount: i64,
    },
    Dispute {
        client: u16,
        tx: u32,
    },
    Resolve {
        client: u16,
        tx: u32,
    },
    Chargeback {
        client: u16,
        tx: u32,
    },
    GetState {
        tx: oneshot::Sender<Vec<State>>, // Return a stream instead?
    },
}

struct Processor {
    accounts: BTreeMap<u16, Account>,
}

impl Processor {
    fn new() -> Processor {
        Self {
            accounts: BTreeMap::new(),
        }
    }

    fn tx<F>(&mut self, client: u16, create: bool, mut f: F) -> Result<(), Error>
    where
        F: FnMut(&mut Account) -> Result<(), account::Error>,
    {
        let account = match self.accounts.entry(client) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                if create {
                    entry.insert(Account::new())
                } else {
                    Err(Error::UnknownClient(client))?
                }
            }
        };
        f(account).map_err(|err| Error::Transaction { client, err })
    }

    async fn handle(&mut self, msg: Message, tx_err: &mpsc::Sender<Error>) {
        use Message::*;

        let res = match msg {
            Deposit { client, tx, amount } => self.tx(client, true, |a| a.deposit(tx, amount)),
            Withdrawal { client, tx, amount } => self.tx(client, false, |a| a.withdraw(tx, amount)),
            Dispute { client, tx } => self.tx(client, false, |a| a.dispute(tx)),
            Resolve { client, tx } => self.tx(client, false, |a| a.resolve(tx)),
            Chargeback { client, tx } => self.tx(client, false, |a| a.chargeback(tx)),
            GetState { tx } => tx.send(self.state()).map_err(|_| Error::Send()),
        };
        if let Err(err) = res {
            let _ = tx_err.send(err).await;
        }
    }

    fn state(&self) -> Vec<State> {
        self.accounts
            .iter()
            .map(|(client, account)| State {
                client: *client,
                available: account.available,
                held: account.held,
                total: account.total(),
                locked: account.locked,
            })
            .collect()
    }
}

pub async fn run() -> (mpsc::Sender<Message>, mpsc::Receiver<Error>) {
    let (tx_msg, mut rx_msg) = mpsc::channel(100);
    let (tx_err, rx_err) = mpsc::channel(100);

    tokio::spawn(async move {
        let mut processor = Processor::new();
        while let Some(msg) = rx_msg.recv().await {
            processor.handle(msg, &tx_err).await;
        }
    });

    (tx_msg, rx_err)
}
