use zing_rs::{table::Table, zing_game::ZingGame};


fn main() {
    let table = Table{ players: vec![
        zing_rs::table::Player{ name: "Hans".into() },
        zing_rs::table::Player{ name: "Darko".into() }
    ]};
    let game = ZingGame::new_from_table(table);
}
