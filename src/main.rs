mod account;
mod amount;
mod processor;
mod serde_str;

use amount::Amount;
use serde::{self, Deserialize, Serialize};
use serde_str::StringSerialized;
use tokio::sync::oneshot;

#[derive(Debug, Deserialize)]
struct Input {
    #[serde(alias = "type")]
    type_: String,
    client: u16,
    tx: u32,
    amount: Option<StringSerialized<Amount>>,
}

// csv doesn't support internally tagged unions :( https://github.com/BurntSushi/rust-csv/issues/211
impl TryFrom<Input> for processor::Message {
    type Error = String;

    fn try_from(i: Input) -> Result<Self, Self::Error> {
        match i.type_.as_str() {
            "deposit" => Ok(processor::Message::Deposit {
                client: i.client,
                tx: i.tx,
                amount: i.amount.ok_or("missing amount for deposit")?.0 .0,
                // .and_then(parse_amount)?,
            }),
            "withdrawal" => Ok(processor::Message::Withdrawal {
                client: i.client,
                tx: i.tx,
                amount: i.amount.ok_or("missing amount for withdrawal")?.0 .0,
                // .and_then(parse_amount)?,
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
            _ => Err(format!("invalid input type: {}", i.type_)),
        }
    }
}

#[derive(Debug, Serialize)]
struct Output {
    client: u16,
    available: StringSerialized<Amount>,
    held: StringSerialized<Amount>,
    total: StringSerialized<Amount>,
    locked: bool,
}

#[tokio::main]
async fn main() {
    let (tx_msg, mut rx_err) = processor::run().await;

    tokio::spawn(async move {
        while let Some(res) = rx_err.recv().await {
            eprintln!("{:?}", res);
        }
    });

    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path("foo.csv")
        .unwrap(); // fixme
    for result in rdr.deserialize() {
        // println!(">>> {:?}", result);
        let record: Input = result.unwrap(); // fixme
        let msg: processor::Message = record.try_into().unwrap(); // fixme
        tx_msg.send(msg).await.unwrap(); // fixme
    }

    let (tx, rx) = oneshot::channel();
    let _ = tx_msg.send(processor::Message::GetState { tx }).await;
    if let Ok(state) = rx.await {
        let mut wtr = csv::Writer::from_writer(std::io::stdout());
        for s in state {
            let processor::State {
                client,
                available,
                held,
                total,
                locked,
            } = s;
            wtr.serialize(Output {
                client,
                available: StringSerialized(Amount(available)),
                held: StringSerialized(Amount(held)),
                total: StringSerialized(Amount(total)),
                locked,
            })
            .unwrap(); // fixme
        }
        wtr.flush().unwrap(); // fixme
                              // println!("state = {:?}", s);
    } else {
        eprintln!("the sender dropped");
    }
}
