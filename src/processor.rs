use std::collections::btree_map::{BTreeMap, Entry};

use crate::account::{self, Account};
use tokio::sync::{mpsc, oneshot};

type Accounts = BTreeMap<u16, Account>;

#[derive(Debug)]
pub enum Error {
    TransactionError { client: u16, err: account::Error },
    IoError(),
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
        tx: oneshot::Sender<Vec<State>>, // todo: return a stream instead
    },
}

fn account(accounts: &mut Accounts, client: u16) -> &mut Account {
    // todo use `or_insert` once stable
    match accounts.entry(client) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(Account::new()),
    }
}

fn state(accounts: &Accounts) -> Vec<State> {
    accounts
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

async fn handle(msg: Message, accounts: &mut Accounts, tx_err: &mpsc::Sender<Error>) {
    use Message::*;
    let res = match msg {
        Deposit { client, tx, amount } => account(accounts, client)
            .deposit(tx, amount)
            .map_err(|err| Error::TransactionError { client, err }),
        Withdrawal { client, tx, amount } => account(accounts, client)
            .withdraw(tx, amount)
            .map_err(|err| Error::TransactionError { client, err }),
        Dispute { client, tx } => account(accounts, client)
            .dispute(tx)
            .map_err(|err| Error::TransactionError { client, err }),
        Resolve { client, tx } => account(accounts, client)
            .resolve(tx)
            .map_err(|err| Error::TransactionError { client, err }),
        Chargeback { client, tx } => account(accounts, client)
            .chargeback(tx)
            .map_err(|err| Error::TransactionError { client, err }),
        GetState { tx } => {
            if tx.send(state(accounts)).is_err() {
                Err(Error::IoError())
            } else {
                Ok(())
            }
        }
    };
    if let Err(err) = res {
        let _ = tx_err.send(err).await;
    }
}

pub async fn run() -> (mpsc::Sender<Message>, mpsc::Receiver<Error>) {
    let (tx_msg, mut rx_msg) = mpsc::channel(100);
    let (tx_err, rx_err) = mpsc::channel(100);

    let mut accounts = BTreeMap::new();

    tokio::spawn(async move {
        while let Some(msg) = rx_msg.recv().await {
            // println!("got = {:?}", msg);
            handle(msg, &mut accounts, &tx_err).await;
        }
    });

    (tx_msg, rx_err)
}
