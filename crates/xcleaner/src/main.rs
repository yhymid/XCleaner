//! XCleaner GUI (GTK4 + libadwaita). `xcleaner --scan` prints a report instead.

mod ui;

use xcleaner_core::{
    format_size,
    i18n::{self, Lang},
    scan, tr, trf,
};

fn main() -> std::process::ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // Language: `--lang en|pl` (this run only), else $XCLEANER_LANG, else
    // the saved choice, else English.
    let mut lang = i18n::from_env_or_saved();
    if let Some(i) = args.iter().position(|a| a == "--lang") {
        match args.get(i + 1).and_then(|v| Lang::parse(v)) {
            Some(l) => lang = l,
            None => {
                eprintln!("--lang: en | pl");
                return std::process::ExitCode::from(2);
            }
        }
        args.drain(i..i + 2);
    }
    i18n::set(lang);
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
        Some("--debug-evidence") => {
            xcleaner_core::default_sigpipe();
            let Some(layout) = scan::Layout::from_env() else {
                return std::process::ExitCode::FAILURE;
            };
            print!("{}", xcleaner_core::leftovers::debug_dump(&layout.home));
            std::process::ExitCode::SUCCESS
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
            let v = env!("CARGO_PKG_VERSION");
            println!(
                "{}",
                trf!(
                    "XCleaner {v}\n\n  xcleaner                      the window\n  xcleaner --scan [--details]   report in the terminal (deletes nothing)\n  xcleaner --usage [dir]        what takes space in HOME (read-only)\n  --lang en|pl                  language for this run (the window saves its own choice)\n  --no-files-db                 without `pacman -Fl` (older leftover detection)",
                    "XCleaner {v}\n\n  xcleaner                      okno programu\n  xcleaner --scan [--details]   raport w terminalu (nic nie usuwa)\n  xcleaner --usage [katalog]    co zajmuje miejsce w HOME (tylko odczyt)\n  --lang en|pl                  język tego uruchomienia (okno zapisuje własny wybór)\n  --no-files-db                 bez `pacman -Fl` (stare wykrywanie resztek)"
                )
            );
            std::process::ExitCode::SUCCESS
        }
        _ => {
            if xcleaner_core::is_root() {
                eprintln!(
                    "{}",
                    tr!(
                        "Do not run XCleaner as root: start it as your user; system cleaning asks for the password in a terminal.",
                        "Nie uruchamiaj XCleanera jako root: uruchom go jako swój użytkownik; czyszczenie systemu zapyta o hasło w terminalu."
                    )
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
                "{}",
                trf!(
                    "\nTotal ~/{}: {} · {} ms",
                    "\nRazem ~/{}: {} · {} ms",
                    b.rel.display(),
                    format_size(b.total),
                    b.elapsed.as_millis()
                )
            );
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", trf!("Error: {e}", "Błąd: {e}"));
            std::process::ExitCode::FAILURE
        }
    }
}

fn cli_scan(details: bool) -> std::process::ExitCode {
    let Some(layout) = scan::Layout::from_env() else {
        eprintln!("{}", tr!("No valid HOME", "Brak poprawnego HOME"));
        return std::process::ExitCode::FAILURE;
    };
    let res = match scan::scan(&layout) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", trf!("Scan error: {e}", "Błąd skanowania: {e}"));
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
            println!("               └ {}", trf!("error: {e}", "błąd: {e}"));
        }
    }
    let selected: u64 = found
        .iter()
        .filter(|f| f.blocked.is_none() && f.item.default_selected())
        .map(|f| f.usage.bytes)
        .sum();
    println!(
        "{}",
        trf!(
            "\nTotal: {} · selected by default: {} · scan {} ms",
            "\nRazem: {} · domyślnie zaznaczone: {} · skan {} ms",
            format_size(res.total()),
            format_size(selected),
            res.elapsed.as_millis()
        )
    );
    if res.hidden > 0 {
        println!(
            "{}",
            trf!(
                "Hidden by the keep list: {}",
                "Ukryte przez listę wyjątków: {}",
                res.hidden
            )
        );
    }
    std::process::ExitCode::SUCCESS
}
