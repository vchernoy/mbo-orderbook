use anyhow::Result;
use clap::Parser;
use clap::ValueEnum;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

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
    let path = args.input;

    let addr: SocketAddr = args.bind.parse()?;
    let listener = TcpListener::bind(addr).await?;
    let filepath = Arc::new(path);

    println!("Listening on {}", addr);

    match args.mode {
        Mode::Buffered => {
            println!("Loading DBN file into memory: {:?}", filepath);

            let mut file = File::open(&*filepath).await?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await?;
            let data = Arc::new(buf);

            println!("File loaded ({} bytes). Listening on {}", data.len(), addr);

            loop {
                let (mut socket, peer) = listener.accept().await?;
                let data = Arc::clone(&data);

                tokio::spawn(async move {
                    println!("New client: {}", peer);

                    if let Err(e) = socket.write_all(&data).await {
                        eprintln!("Error sending to client: {e}");
                        return;
                    }

                    let _ = socket.shutdown().await;
                    println!("Finished streaming to {}", peer);
                });
            }
        }
        Mode::Streaming => {
            loop {
                let (mut socket, peer) = listener.accept().await?;
                let filepath = filepath.clone();

                tokio::spawn(async move {
                    println!("New client: {}", peer);
                    println!("Streaming DBN file: {:?}", filepath);

                    let mut file = match File::open(&*filepath).await {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error opening file: {e}");
                            return;
                        }
                    };

                    let mut buf = [0u8; 64 * 1024];
                    loop {
                        match file.read(&mut buf).await {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                if let Err(e) = socket.write_all(&buf[..n]).await {
                                    eprintln!("Error sending to client: {e}");
                                    return;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error reading file: {e}");
                                return;
                            }
                        }
                    }

                    let _ = socket.shutdown().await;
                    println!("Finished streaming to {}", peer);
                });
            }
        }
    }
}
