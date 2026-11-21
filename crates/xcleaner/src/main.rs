//! XCleaner GUI (GTK4 + libadwaita). `xcleaner --scan` prints a report instead.

mod ui;

use xcleaner_core::{format_size, scan};

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") => {
            println!("XCleaner {}", env!("CARGO_PKG_VERSION"));
            std::process::ExitCode::SUCCESS
        }
        Some("--scan") => cli_scan(),
        Some("--help") | Some("-h") => {
            println!("XCleaner {}\n\n  xcleaner          okno programu\n  xcleaner --scan   raport w terminalu (nic nie usuwa)", env!("CARGO_PKG_VERSION"));
            std::process::ExitCode::SUCCESS
        }
        _ => {
            if xcleaner_core::is_root() {
                eprintln!(
                    "XCleaner nie działa jako root w tej wersji (czyści tylko katalog domowy)."
                );
                return std::process::ExitCode::FAILURE;
            }
            ui::run()
        }
    }
}

fn cli_scan() -> std::process::ExitCode {
    let Some(layout) = scan::Layout::from_env() else {
        eprintln!("Brak poprawnego HOME");
        return std::process::ExitCode::FAILURE;
    };
    let res = match scan::scan(&layout) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Błąd skanowania: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut found = res.found.clone();
    found.sort_by_key(|f| std::cmp::Reverse(f.usage.bytes));
    for f in &found {
        let mark = if f.blocked.is_some() {
            "!"
        } else if f.item.default_selected() {
            "x"
        } else {
            " "
        };
        println!(
            "[{mark}] {:>10}  {}  ({}, {})",
            format_size(f.usage.bytes),
            f.item.name,
            f.item.risk.label(),
            f.item.regen.label()
        );
        if let Some(b) = &f.blocked {
            println!("               └ {b}");
        }
        for e in &f.errors {
            println!("               └ błąd: {e}");
        }
    }
    let selected: u64 = found
        .iter()
        .filter(|f| f.blocked.is_none() && f.item.default_selected())
        .map(|f| f.usage.bytes)
        .sum();
    println!(
        "\nRazem: {} · domyślnie zaznaczone: {} · skan {} ms",
        format_size(res.total()),
        format_size(selected),
        res.elapsed.as_millis()
    );
    std::process::ExitCode::SUCCESS
}
