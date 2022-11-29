/**
 * The CLI interface for the transaction processor.
 *
 * Currently supported input is a CSV file name but additional sources can be added. (See comments.)
 *
 * Reading the CSV file continues despite any deserialization errors. The only fatal errors are when the
 * input file can't be read or forwarding messages to processor fails.
 */
use serde::{self, Deserialize, Serialize};
use std::fmt;
use tokio::sync::{
    mpsc::error::SendError,
    oneshot::{self, error::RecvError},
};

use crate::{amount, processor};

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Deserialization error: `{0}`.")]
    De(csv::Error),
    #[error("Serialization error: `{0}`.")]
    Ser(csv::Error),
    #[error("Input error: `{0}`.")]
    Input(String),
    #[error("Send error: `{0}`.")]
    Send(SendError<processor::Message>),
    #[error("Receive state error: `{0}`.")]
    RecvState(RecvError),
    #[error("IO error: `{0}`.")]
    Io(std::io::Error),
}

// Used by default when the main function returns Err.
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self, f)?;
        Ok(())
    }
}

// CSV structure of the input file
#[derive(Debug, Deserialize)]
struct Input {
    r#type: String,
    client: u16,
    tx: u32,
    #[serde(with = "amount")]
    amount: Option<i64>,
}

// The csv crate doesn't support internally tagged unions :( (https://github.com/BurntSushi/rust-csv/issues/211)
impl TryFrom<Input> for processor::Message {
    type Error = Error;

    fn try_from(i: Input) -> std::result::Result<Self, Self::Error> {
        match i.r#type.as_str() {
            "deposit" => Ok(processor::Message::Deposit {
                client: i.client,
                tx: i.tx,
                amount: i
                    .amount
                    .ok_or_else(|| Error::Input("missing amount for deposit".to_string()))?,
            }),
            "withdrawal" => Ok(processor::Message::Withdrawal {
                client: i.client,
                tx: i.tx,
                amount: i
                    .amount
                    .ok_or_else(|| Error::Input("missing amount for deposit".to_string()))?,
            }),
            "dispute" => Ok(processor::Message::Dispute {
                client: i.client,
                tx: i.tx,
            }),
            "resolve" => Ok(processor::Message::Resolve {
                client: i.client,
                tx: i.tx,
            }),
            "chargeback" => Ok(processor::Message::Chargeback {
                client: i.client,
                tx: i.tx,
            }),
            _ => Err(Error::Input(format!("invalid input type: '{}'", i.r#type))),
        }
    }
}

// CSV structure of the output file
#[derive(Debug, Serialize)]
struct Output {
    client: u16,
    #[serde(with = "amount")]
    available: i64,
    #[serde(with = "amount")]
    held: i64,
    #[serde(with = "amount")]
    total: i64,
    locked: bool,
}

fn read_csv<R: std::io::Read>(reader: R) -> impl Iterator<Item = processor::Message> {
    let reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    reader
        .into_deserialize::<Input>()
        .map(|res_input| res_input.map_err(Error::De).and_then(TryInto::try_into))
        .filter_map(|res_msg| res_msg.map_err(|e| eprintln!("{}", e)).ok())
}

pub async fn run<R: std::io::Read, W: std::io::Write>(reader: R, writer: W) -> Result<(), Error> {
    // Create the processor and the get send and receive handles for transaction messages
    // and errors.
    let (tx_msg, mut rx_err) = processor::run().await;

    tokio::spawn(async move {
        while let Some(res) = rx_err.recv().await {
            eprintln!("{}", res); // log transaction errors to stderr
        }
    });

    // Send transaction messages extracted from the CSV file to the transaction processor.
    // Additional sources can by added by replicating this pattern and running the message
    // producers in dedicated threads.
    let tx_csv = tx_msg.clone();
    for csv_msg in read_csv(reader) {
        tx_csv.send(csv_msg).await.map_err(Error::Send)?;
    }
    drop(tx_csv);

    // Finally request the state of the transaction processor.
    let (tx_state, rx_state) = oneshot::channel();
    tx_msg
        .send(processor::Message::GetState { tx: tx_state })
        .await
        .map_err(Error::Send)?;
    let state = rx_state.await.map_err(Error::RecvState)?;
    let mut wtr = csv::Writer::from_writer(writer);
    for s in state {
        if let Err(err) = wtr
            .serialize(Output {
                client: s.client,
                available: s.available,
                held: s.held,
                total: s.total,
                locked: s.locked,
            })
            .map_err(Error::Ser)
        {
            eprintln!("{}", err);
        }
    }
    wtr.flush().map_err(Error::Io)
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn samples() {
        let file = std::fs::File::open("data/in.csv").unwrap();
        let expected = std::fs::read_to_string("data/out.csv").unwrap();
        let mut buf = Vec::new();
        let _ = super::run(file, &mut buf).await;
        let actual = String::from_utf8(buf).unwrap();
        assert_eq!(actual, expected)
    }
}
