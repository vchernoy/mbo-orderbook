use std::path::PathBuf;
//use std::{
//    collections::{BTreeMap, HashMap, VecDeque},
//    fmt::Display,
//};

use clap::Parser;

use databento::dbn::{decode::AsyncDbnDecoder, Action, MboMsg, Side};

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

/// Pretty-print a single MBO record.
fn print_pretty(idx: usize, mbo: &MboMsg) {
    // Decode action / side from raw bytes (i8 → u8 → enum)
    let action = Action::try_from(mbo.action as u8).unwrap_or(Action::None);
    let side = Side::try_from(mbo.side as u8).unwrap_or(Side::None);

    // Databento doc: prices are 1e-9 fixed precision units
    let price = mbo.price as f64 / 1_000_000_000.0;

    println!(
        "#{:<6} ts_event={} instr_id={} oid={} px={:.2} qty={:<4} side={:?} action={:?}",
        idx, mbo.hd.ts_event, mbo.hd.instrument_id, mbo.order_id, price, mbo.size, side, action,
    );
}
