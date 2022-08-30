mod account;
mod processor;

use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    let (tx_msg, mut rx_err) = processor::run().await;

    tokio::spawn(async move {
        while let Some(res) = rx_err.recv().await {
            println!("error = {:?}", res);
        }
    });

    assert!(tx_msg
        .send(processor::Message::Deposit {
            client: 0,
            tx: 0,
            amount: 1,
        })
        .await
        .is_ok());
    assert!(tx_msg
        .send(processor::Message::Deposit {
            client: 0,
            tx: 1,
            amount: 1,
        })
        .await
        .is_ok());
    assert!(tx_msg
        .send(processor::Message::Deposit {
            client: 1,
            tx: 0,
            amount: 1,
        })
        .await
        .is_ok());
    assert!(tx_msg
        .send(processor::Message::Deposit {
            client: 1,
            tx: 0,
            amount: 1,
        })
        .await
        .is_ok());
    assert!(tx_msg
        .send(processor::Message::Deposit {
            client: 1,
            tx: 1,
            amount: 1,
        })
        .await
        .is_ok());

    let (tx, rx) = oneshot::channel();
    let _ = tx_msg.send(processor::Message::GetState { tx }).await;
    match rx.await {
        Ok(v) => println!("state = {:?}", v),
        Err(_) => println!("the sender dropped"),
    }
}
