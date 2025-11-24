use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use clap::Parser;
use clap::ValueEnum;
use databento::dbn::encode::AsyncEncodeRecord;
use databento::dbn::{
    decode::{AsyncDbnDecoder, DbnMetadata},
    encode::AsyncDbnEncoder,
    MboMsg, Metadata,
};
use tokio::{io::BufWriter, net::TcpListener, net::TcpStream};

/// Stream DBN MBO records to any TCP client that connects.
#[derive(Parser, Debug)]
#[command(
    name = "mbo-streammer",
    version,
    about = "Replay a DBN MBO file to any TCP client as a DBN stream",
    long_about = None
)]
struct Args {
    /// Path to the input DBN file with MBO records
    #[arg(value_name = "DBN_FILE")]
    input: PathBuf,

    /// Address to bind the TCP server to, e.g. 0.0.0.0:5000
    #[arg(long, short, default_value = "0.0.0.0:5000")]
    bind: String,

    /// Mode: buffered (load into memory once) or streaming (re-read file per client)
    #[arg(long, short, value_enum, default_value_t = Mode::Buffered)]
    mode: Mode,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Mode {
    Buffered,
    Streaming,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // let path = Arc::new(args.input);

    match args.mode {
        Mode::Buffered => {
            // 1) Load DBN file into memory: metadata + all MboMsg records
            let path = &args.input;
            println!("Loading DBN file: {:?}", path);

            let (metadata, records) = load_dbn_mbo_file(path).await?;
            let metadata = Arc::new(metadata);
            let records = Arc::new(records);

            println!(
                "Loaded {} MBO records. Starting TCP server on {}",
                records.len(),
                args.bind
            );

            // 2) Start TCP listener
            let addr: SocketAddr = args.bind.parse()?;
            let listener = TcpListener::bind(addr).await?;

            loop {
                let (socket, peer) = listener.accept().await?;
                println!("New client connected: {}", peer);

                let metadata = Arc::clone(&metadata);
                let records = Arc::clone(&records);

                tokio::spawn(async move {
                    if let Err(err) = handle_client(socket, &metadata, &records).await {
                        eprintln!("Error serving {}: {err}", peer);
                    } else {
                        println!("Finished streaming to {}", peer);
                    }
                });
            }
        }
        Mode::Streaming => {
            let path = Arc::new(args.input);
            let addr: SocketAddr = args.bind.parse()?;

            println!("Listening on {}", addr);
            println!("Streaming DBN file: {:?}", path);

            let listener = TcpListener::bind(addr).await?;

            loop {
                let (socket, peer) = listener.accept().await?;
                println!("New client connected: {}", peer);

                let path = Arc::clone(&path);

                tokio::spawn(async move {
                    if let Err(err) = handle_client_async(socket, &path).await {
                        eprintln!("Error serving {}: {err}", peer);
                    } else {
                        println!("Finished streaming to {}", peer);
                    }
                });
            }
        }
    }
}

/// Load metadata + all MBO records from a DBN file into memory.
async fn load_dbn_mbo_file(path: &Path) -> Result<(Metadata, Vec<MboMsg>)> {
    let mut decoder = AsyncDbnDecoder::from_file(path).await?;

    // Metadata is parsed first and available via decoder.metadata()
    let metadata = decoder.metadata().clone();

    let mut records = Vec::new();
    let mut rec_idx = 0;
    while let Some(mbo) = decoder.decode_record::<MboMsg>().await? {
        rec_idx += 1;
        // println!("{rec_idx}: {:?}", mbo);
        records.push(mbo.clone());

        if rec_idx % 100_000 == 0 {
            println!("  loaded {} records…", rec_idx);
        }
    }
    let count = records.len();
    println!("Finished loading: {} records total.", count);

    Ok((metadata, records))
}

/// Handle a single TCP client: send metadata + all records as DBN.
async fn handle_client(socket: TcpStream, metadata: &Metadata, records: &[MboMsg]) -> Result<()> {
    // Creates encoder and writes metadata immediately
    let mut encoder = AsyncDbnEncoder::new(BufWriter::new(socket), metadata).await?;

    for mbo in records {
        encoder.encode_record(mbo).await?;
    }

    encoder.flush().await?;
    encoder.shutdown().await?; // if you want to close the writer nicely

    Ok(())
}

/// For each client:
/// - open the DBN file
/// - decode metadata + MboMsg records
/// - encode them to the socket as DBN
async fn handle_client_async(socket: TcpStream, path: &Path) -> Result<()> {
    // 1) Open DBN file and create decoder
    let mut decoder = AsyncDbnDecoder::from_file(path).await?;

    // 2) Get metadata
    let metadata = decoder.metadata().clone();

    // 3) Create encoder on the socket; this writes metadata immediately
    let mut encoder = AsyncDbnEncoder::new(BufWriter::new(socket), &metadata).await?;

    // 4) Stream all records: decode from file, encode to client
    while let Some(mbo) = decoder.decode_record::<MboMsg>().await? {
        encoder.encode_record(mbo).await?;
    }

    // 5) Flush (and optionally shutdown)
    encoder.flush().await?;
    // encoder.shutdown().await?; // if the API provides it, nice to call

    Ok(())
}
