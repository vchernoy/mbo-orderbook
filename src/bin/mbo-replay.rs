use ::mbo_orderbook::common::print_pretty;
use std::path::PathBuf;

use clap::Parser;

use databento::dbn::{decode::AsyncDbnDecoder, MboMsg};

/// Replay MBO records from a DBN file.
#[derive(Parser, Debug)]
#[command(
    name = "mbo-replay",
    version,
    about = "Replay MBO market data (DBN) and print records",
    long_about = None
)]
struct Args {
    /// Path to the input DBN file
    #[arg(value_name = "DBN_FILE")]
    input: PathBuf,

    /// Maximum number of records to print (0 = no limit)
    #[arg(long, short, default_value_t = 0)]
    limit: usize,

    /// Pretty-print records instead of raw Debug
    #[arg(long)]
    pretty: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path = args.input;
    println!("Reading DBN file: {:?}", path);

    let mut decoder = AsyncDbnDecoder::from_file(path).await?;

    let mut rec_idx = 0;
    while let Some(mbo) = decoder.decode_record::<MboMsg>().await? {
        rec_idx += 1;
        if args.pretty {
            print_pretty(rec_idx, mbo);
        } else {
            println!("{rec_idx}: {:?}", mbo);
        }

        if args.limit > 0 && rec_idx >= args.limit {
            break;
        }
    }

    Ok(())
}
