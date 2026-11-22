//! XCleaner GUI (GTK4 + libadwaita). `xcleaner --scan` prints a report instead.

mod ui;

use xcleaner_core::{format_size, scan};

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--no-files-db") {
        // Read by xcleaner-core before any thread starts.
        std::env::set_var("XCLEANER_NO_FILES_DB", "1");
    }
    let first = args
        .iter()
        .find(|a| *a != "--no-files-db")
        .map(String::as_str);
    match first {
        Some("--version") => {
            println!("XCleaner {}", env!("CARGO_PKG_VERSION"));
            std::process::ExitCode::SUCCESS
        }
        Some(xcleaner_core::term::WRAP_ARG) => {
            let rest: Vec<String> = args.iter().skip(1).cloned().collect();
            std::process::ExitCode::from(xcleaner_core::term::run_wrapper(&rest).clamp(0, 255) as u8)
        }
        Some(xcleaner_core::root::ROOT_ARG) => {
            let rest: Vec<String> = args
                .iter()
                .skip_while(|a| *a != xcleaner_core::root::ROOT_ARG)
                .skip(1)
                .cloned()
                .collect();
            std::process::ExitCode::from(xcleaner_core::root::run_root(&rest) as u8)
        }
        Some("--usage") | Some("--scan") => {
            xcleaner_core::default_sigpipe();
            if first == Some("--usage") {
                cli_usage(args.iter().skip_while(|a| *a != "--usage").nth(1))
            } else {
                cli_scan(args.iter().any(|a| a == "--details"))
            }
        }
        Some("--help") | Some("-h") => {
            println!("XCleaner {}\n\n  xcleaner          okno programu\n  xcleaner --scan [--details]   raport w terminalu (nic nie usuwa)\n  xcleaner --usage [katalog]    co zajmuje miejsce w HOME (tylko odczyt)\n  --no-files-db                 bez `pacman -Fl` (stare wykrywanie resztek)", env!("CARGO_PKG_VERSION"));
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

fn cli_usage(sub: Option<&String>) -> std::process::ExitCode {
    let Some(layout) = scan::Layout::from_env() else {
        return std::process::ExitCode::FAILURE;
    };
    let rel = std::path::PathBuf::from(sub.map(String::as_str).unwrap_or(""));
    match xcleaner_core::usage::breakdown(&layout.home, &rel) {
        Ok(b) => {
            for e in b.entries.iter().take(25) {
                let note = e
                    .note
                    .as_deref()
                    .map(|n| format!("  ({n})"))
                    .unwrap_or_default();
                println!(
                    "{:>10}  {}{}{note}",
                    format_size(e.usage.bytes),
                    e.name,
                    if e.is_dir { "/" } else { "" }
                );
            }
            println!(
                "\nRazem ~/{}: {} · {} ms",
                b.rel.display(),
                format_size(b.total),
                b.elapsed.as_millis()
            );
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Błąd: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn cli_scan(details: bool) -> std::process::ExitCode {
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
        if details {
            for l in f.item.description.lines() {
                println!("               │ {l}");
            }
            for (t, a) in &f.item.targets {
                println!(
                    "               · {}",
                    xcleaner_core::rules::target_label(t, a)
                );
            }
        }
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
    if res.hidden > 0 {
        println!("Ukryte przez listę wyjątków: {}", res.hidden);
    }
    std::process::ExitCode::SUCCESS
}
