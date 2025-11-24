use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use databento::dbn::{
    decode::{AsyncDbnDecoder, DbnMetadata},
    Action, MboMsg, Side,
};
use tokio::net::TcpStream;

/// Connect to mbo-streammer, read DBN MBO data, decode, and print records.
#[derive(Parser, Debug)]
#[command(
    name = "mbo-consumer",
    version,
    about = "Connects to mbo-streammer and prints MBO records",
    long_about = None
)]
struct Args {
    /// Address of mbo-streammer, e.g. 127.0.0.1:5000
    #[arg(
        long,
        short = 'a',
        value_name = "ADDR",
        default_value = "127.0.0.1:5000"
    )]
    addr: String,

    /// Maximum number of records to print (0 = no limit)
    #[arg(long, short, default_value_t = 0)]
    limit: usize,

    /// Pretty-print records instead of raw Debug
    #[arg(long)]
    pretty: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let addr: SocketAddr = args.addr.parse()?;
    println!("Connecting to mbo-streamer at {}", addr);

    let stream = TcpStream::connect(addr).await?;
    println!("Connected, starting to read DBN stream…");

    // AsyncDbnDecoder can take any AsyncRead (like TcpStream)
    let mut decoder = AsyncDbnDecoder::new(stream).await?;

    // We can inspect metadata if desired
    let metadata = decoder.metadata().clone();
    println!(
        "Received metadata: schema={:?}, dataset={}",
        metadata.schema, metadata.dataset
    );

    let mut rec_idx: usize = 0;

    // Main read loop
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

    println!("Stream ended, total records: {}", rec_idx);
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
