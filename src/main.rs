use anyhow::{bail, Error};
use clap::Parser;
use csv::{ReaderBuilder, Trim};

use std::collections::HashSet;

mod balance;
mod clients;
mod ids;
mod transaction;

#[derive(Parser)]
#[clap(name = "paytoy", about = "Simple example payments engine")]
struct Args {
    /// Input CSV file with header row: type, client, tx, amount
    #[clap(required = true)]
    input: String,
}

pub fn print_headers() {
    println!("client,available,held,total,locked");
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let mut clients = clients::Clients::default();

    let mut rdr = ReaderBuilder::new().trim(Trim::All).from_path(args.input)?;

    let valid_headers = HashSet::from(["type", "client", "tx", "amount"]);
    for h in rdr.headers()? {
        if !valid_headers.contains(h) {
            bail!("Invalid header {}", h);
        }
    }

    for result in rdr.deserialize() {
        let t: transaction::Transaction = result?;
        clients.process(t)?;
    }

    print_headers();
    print!("{}", clients);
    Ok(())
}
