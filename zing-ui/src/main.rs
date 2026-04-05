use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
struct Cli {
    login_id: String,
    table_id: String,
    #[arg(default_value = "http://localhost:8000")]
    base_url: String,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Cli::parse();
    zing_ui_lib::start_remote_game(args.login_id, args.table_id, args.base_url)
}
