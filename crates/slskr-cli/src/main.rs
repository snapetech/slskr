#[tokio::main]
async fn main() {
    if let Err(error) = slskr_cli::run_from_env().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
