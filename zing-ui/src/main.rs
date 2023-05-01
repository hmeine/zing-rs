use clap::Parser;

#[derive(Parser)]
struct Cli {
    login_id: String,
    table_id: String,
    #[arg(default_value = "http://localhost:3000")]
    base_url: String,
}

fn main() {
    let args = Cli::parse();
    zing_ui_lib::start_remote_game(args.login_id, args.table_id, args.base_url)
}
