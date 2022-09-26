use anyhow::{bail, Error};
use clap::Parser;
use csv::{ReaderBuilder, Trim};

use futures::future::try_join_all;
use tokio::sync::mpsc;

use std::cmp::min;
use std::collections::HashSet;

mod balance;
mod clients;
mod ids;
mod transaction;

use crate::clients::Clients;
use crate::transaction::TranType;

const SHARD_QUEUE_MAX: usize = 1_000_000;

#[derive(Parser)]
#[clap(name = "paytoy", about = "Simple example payments engine")]
struct Args {
    /// Input CSV file with header row: type, client, tx, amount
    #[clap(required = true)]
    input: String,
}

fn print_headers() {
    println!("client,available,held,total,locked");
}

async fn process_csv(input: String) -> Result<Clients, Error> {
    let mut rdr = ReaderBuilder::new().trim(Trim::All).from_path(input)?;

    let valid_headers = HashSet::from(["type", "client", "tx", "amount"]);
    for h in rdr.headers()? {
        if !valid_headers.contains(h) {
            bail!("Invalid header {}", h);
        }
    }

    // size number of shards based on cpu count
    let num_shards: u16 = min(num_cpus::get(), u16::MAX as usize) as u16;

    let mut shard_futs = Vec::with_capacity(num_shards.into());

    let mut shard_handles = Vec::with_capacity(num_shards.into());
    {
        // Spawn the worker shards, channel per shard
        for _i in 0..num_shards {
            let (tx, mut rx) = mpsc::channel(SHARD_QUEUE_MAX);
            shard_handles.push(tx);
            shard_futs.push(tokio::spawn(async move {
                let mut shard = Clients::default();
                while let Some(t) = rx.recv().await {
                    shard.process(t)?;
                }
                Ok::<_, Error>(shard)
            }));
        }
    }

    // Read from the csv and send to the shards
    let mut seen_tx = HashSet::new();
    for result in rdr.deserialize() {
        let t: transaction::Transaction = result?;
        match t.tran_type {
            TranType::Deposit | TranType::Withdrawal => {
                if seen_tx.contains(&t.tx) {
                    bail!("Reused transaction {}", t.tx.id());
                }
                seen_tx.insert(t.tx);
            }
            _ => (),
        }
        let shard_id = t.client.id() % num_shards;
        shard_handles[shard_id as usize].send(t).await?;
    }

    // Close the channels
    shard_handles.clear();

    // collect the results
    let mut combined = Clients::default();
    for one_shard in try_join_all(shard_futs).await? {
        combined.combine(one_shard?)?;
    }

    Ok(combined)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    let clients = process_csv(args.input).await?;
    print_headers();
    print!("{}", clients);
    Ok(())
}
