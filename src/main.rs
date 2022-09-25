use clap::Parser;

mod balance;
mod ids;
mod transaction;

#[derive(Parser)]
#[clap(name = "paytoy", about = "Simple example payments engine")]
struct Args {
    /// Input CSV file with header row: type, client, tx, amount
    #[clap(required = true)]
    input: String,
}

fn main() {
    let _args = Args::parse();
}
