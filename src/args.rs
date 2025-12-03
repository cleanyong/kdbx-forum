use std::path::PathBuf;

use clap::Parser;

/// CLI arguments for kdbx-forum.
#[derive(Parser, Debug)]
#[command(
    name = "kdbx-forum",
    about = "Serve a read-only mini forum backed by a KeePass KDBX database"
)]
pub struct Args {
    /// Path to the .kdbx database
    #[arg(short, long)]
    pub database: PathBuf,

    /// Master password (if omitted, will be prompted interactively)
    #[arg(short = 'P', long)]
    pub password: Option<String>,

    /// Optional key file for the database
    #[arg(short = 'f', long)]
    pub keyfile: Option<PathBuf>,

    /// Address to listen on, e.g. 127.0.0.1:3000
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub listen: String,
}

