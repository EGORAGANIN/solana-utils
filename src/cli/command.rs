use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use clap::CommandFactory;
use std::string::String;
use solana_clap_v3_utils::input_validators::normalize_to_url_if_moniker;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{Keypair, read_keypair};
use solana_sdk::transaction::Transaction;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(arg_required_else_help = true)]
    KeypairTransform {
        /// Format to transform for keypair
        #[arg(long, short, value_enum)]
        transform: Format,
        /// Filepath to *.json file contain keypair
        #[arg(short, long, value_name = "KEYPAIR")]
        path: Option<PathBuf>,
        /// Raw value from *.json file representation keypair
        #[arg(short, long, value_name = "RAW_VALUE", required_unless_present = "path")]
        value: Option<String>,
    },
    #[clap(arg_required_else_help = true)]
    TransactionSend {
        /// URL for Solana's JSON RPC or moniker (or their first letter): [mainnet-beta, testnet, devnet, localhost]
        #[arg(short, long)]
        url: String,
        /// Raw base64 encoded transaction
        #[arg(short, long)]
        transaction: String,
        /// Filepath to *.json file contain keypair for sign transaction
        #[arg(short, long, value_name = "KEYPAIR")]
        signer: Option<PathBuf>,
        #[arg(long, short, value_enum)]
        format: Option<Format>
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Format {
    Base58,
    Bytes,
}

impl Cli {
    pub fn start() {
        let cli: Cli = Cli::parse();

        match &cli.command {
            Some(Commands::KeypairTransform { transform, path, value }) => {
                let kp_raw_value = match (path, value) {
                    (Some(path), None) => read_keypair_file_as_str(path),
                    (None, Some(value)) => value.to_string(),
                    _ => return Self::command().print_help().unwrap()
                };
                let format = match transform {
                    Format::Base58 => Format::Bytes,
                    Format::Bytes => Format::Base58
                };
                let kp = create_keypair(kp_raw_value.as_str(), &format);
                print_transform_keypair(&kp, &transform);
            }
            Some(Commands::TransactionSend { url, transaction, signer, format }) => {
                let url = normalize_to_url_if_moniker(url);
                let client = RpcClient::new(url);
                println!("RpcClient={:?}", client.url());

                let decoded_tx = base64::decode(transaction).unwrap();
                let mut tx = bincode::deserialize::<Transaction>(&decoded_tx).unwrap();
                println!("Decoded tx={:?}", tx);
                if let Some(signer_path) = signer {
                    let kp_raw_value = read_keypair_file_as_str(signer_path);
                    let format = match format {
                        Some(f) => f,
                        None => &Format::Bytes
                    };
                    let kp = create_keypair(kp_raw_value.as_str(), &format);
                    tx.partial_sign(&[&kp], tx.message.recent_blockhash);
                    println!("Signed tx={:?}", tx);
                }

                println!("Send tx to blockchain");
                let tx_result = client.send_and_confirm_transaction(&tx);
                match tx_result {
                    Ok(signature) => println!("Tx executed SUCCESS, txSignature={:?}", signature),
                    Err(error) => println!("Tx executed FAILED, error={:?}", error)
                }
            }
            None => Self::command().print_help().unwrap(),
        }
    }
}

fn read_keypair_file_as_str(path: &PathBuf) -> String {
    let mut result = String::new();

    File::open(path)
        .expect("could not open keypair file")
        .read_to_string(&mut result)
        .expect("could not read keypair file");

    println!("Read keypair={:?}", result);
    return result;
}

fn create_keypair(kp_value: &str, format: &Format) -> Keypair {
    match format {
        Format::Base58 => Keypair::from_base58_string(&kp_value),
        Format::Bytes => {
            let mut kp_value = kp_value.as_bytes();
            read_keypair(&mut kp_value)
                .expect("could not create keypair from value")
        }
    }
}

fn print_transform_keypair(keypair: &Keypair, transform: &Format) {
    match transform {
        Format::Base58 => println!("Transformed keypair={:?}", keypair.to_base58_string()),
        Format::Bytes => println!("Transformed keypair={:?}", keypair.to_bytes())
    };
}