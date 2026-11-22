//! Main window: scan → grouped checklist → one confirmation → clean.

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use std::{cell::RefCell, rc::Rc};
use xcleaner_core::{
    format_size,
    keep::KeepList,
    pl,
    rules::{Category, Item},
    scan::{self, Found, Layout},
};

const APP_ID: &str = "io.github.yhymid.XCleaner";

struct Row {
    found: Found,
    check: gtk::CheckButton,
}

struct State {
    layout: Layout,
    rows: RefCell<Vec<Row>>,
    content: adw::ToolbarView,
    scan_button: gtk::Button,
    clean_button: gtk::Button,
    summary: gtk::Label,
    toasts: adw::ToastOverlay,
    window: adw::ApplicationWindow,
    /// "Zdarzenia": session log for the user and for debugging.
    events: RefCell<Vec<String>>,
    restore_button: gtk::Button,
}

fn event(state: &State, msg: impl AsRef<str>) {
    let t = glib::DateTime::now_local()
        .ok()
        .and_then(|d| d.format("%H:%M:%S").ok())
        .map(|g| g.to_string())
        .unwrap_or_default();
    for (i, line) in msg.as_ref().lines().enumerate() {
        let prefix = if i == 0 {
            format!("`{t}` ")
        } else {
            "    ".to_string()
        };
        state.events.borrow_mut().push(format!("{prefix}{line}"));
    }
}

pub fn run() -> std::process::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build);
    // GTK must not see our own CLI flags.
    let code = app.run_with_args::<&str>(&[]);
    std::process::ExitCode::from(code.get())
}

fn build(app: &adw::Application) {
    let Some(layout) = Layout::from_env() else {
        eprintln!("Brak poprawnego HOME");
        return;
    };
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("XCleaner")
        .default_width(820)
        .default_height(720)
        .build();

    let header = adw::HeaderBar::new();
    let scan_button = gtk::Button::with_label("Skanuj");
    scan_button.add_css_class("suggested-action");
    header.pack_start(&scan_button);
    let usage_button = gtk::Button::from_icon_name("drive-harddisk-symbolic");
    usage_button.set_tooltip_text(Some("Co zajmuje miejsce"));
    header.pack_end(&usage_button);
    let events_button = gtk::Button::from_icon_name("document-open-recent-symbolic");
    events_button.set_tooltip_text(Some("Zdarzenia"));
    header.pack_end(&events_button);
    let restore_button = gtk::Button::with_label("Przywróć ukryte");
    restore_button.set_visible(false);
    header.pack_end(&restore_button);

    let content = adw::ToolbarView::new();
    content.add_top_bar(&header);
    content.set_content(Some(&status_page(
        "edit-clear-all-symbolic",
        "XCleaner",
        "Skanuj, żeby znaleźć pamięć podręczną, logi i inne śmieci w katalogu domowym.\nNic nie zostanie usunięte bez Twojego potwierdzenia.",
    )));

    let summary = gtk::Label::new(None);
    summary.set_hexpand(true);
    summary.set_xalign(0.0);
    let clean_button = gtk::Button::with_label("Wyczyść zaznaczone");
    clean_button.add_css_class("destructive-action");
    clean_button.set_sensitive(false);
    let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    bottom.set_margin_top(8);
    bottom.set_margin_bottom(8);
    bottom.set_margin_start(12);
    bottom.set_margin_end(12);
    bottom.append(&summary);
    bottom.append(&clean_button);
    content.add_bottom_bar(&bottom);

    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&content));
    window.set_content(Some(&toasts));

    let state = Rc::new(State {
        layout,
        rows: RefCell::new(Vec::new()),
        content,
        scan_button: scan_button.clone(),
        clean_button: clean_button.clone(),
        summary,
        toasts,
        window: window.clone(),
        events: RefCell::new(Vec::new()),
        restore_button: restore_button.clone(),
    });
    event(
        &state,
        format!("XCleaner {} uruchomiony", env!("CARGO_PKG_VERSION")),
    );
    let s = state.clone();
    events_button.connect_clicked(move |_| show_events(&s));
    let s = state.clone();
    usage_button.connect_clicked(move |_| show_usage(&s));
    let s = state.clone();
    restore_button.connect_clicked(move |_| match KeepList::clear() {
        Ok(()) => {
            event(&s, "Wyczyszczono listę ukrytych pozycji");
            start_scan(&s);
        }
        Err(e) => s.toasts.add_toast(adw::Toast::new(&format!("Błąd: {e}"))),
    });

    let s = state.clone();
    scan_button.connect_clicked(move |_| start_scan(&s));
    let s = state.clone();
    clean_button.connect_clicked(move |_| confirm_clean(&s));
    window.present();
    start_scan(&state);
    // QA hook: open a view directly (used for screenshots in development).
    match std::env::var("XCLEANER_OPEN").as_deref() {
        Ok("usage") => show_usage(&state),
        Ok("events") => show_events(&state),
        _ => {}
    }
}

fn status_page(icon: &str, title: &str, desc: &str) -> adw::StatusPage {
    adw::StatusPage::builder()
        .icon_name(icon)
        .title(title)
        .description(desc)
        .build()
}

/// Open a directory in the default file manager. A launch context without
/// the AppImage environment is used: file managers that are not D-Bus
/// activated (Thunar, Nemo) would otherwise load our bundled GTK modules.
fn open_in_file_manager(win: &impl IsA<gtk::Widget>, dir: &std::path::Path) {
    use gtk::gio::prelude::*;
    let ctx = win.as_ref().display().app_launch_context();
    for v in xcleaner_core::FOREIGN_ENV {
        ctx.unsetenv(v);
    }
    match xcleaner_core::host_xdg_data_dirs() {
        Some(d) if d.is_empty() => ctx.unsetenv("XDG_DATA_DIRS"),
        Some(d) => ctx.setenv("XDG_DATA_DIRS", d),
        None => {}
    }
    let uri = gtk::gio::File::for_path(dir).uri();
    gtk::gio::AppInfo::launch_default_for_uri_async(
        &uri,
        Some(&ctx),
        None::<&gtk::gio::Cancellable>,
        |_| {},
    );
}

fn busy_page(title: &str) -> gtk::Widget {
    let spinner = gtk::Spinner::new();
    spinner.start();
    spinner.set_size_request(48, 48);
    let page = adw::StatusPage::builder().title(title).build();
    page.set_child(Some(&spinner));
    page.upcast()
}

fn start_scan(state: &Rc<State>) {
    state.scan_button.set_sensitive(false);
    state.clean_button.set_sensitive(false);
    state.summary.set_text("");
    state.content.set_content(Some(&busy_page("Skanowanie…")));
    let layout = state.layout.clone();
    let handle = gtk::gio::spawn_blocking(move || scan::scan(&layout));
    let state = state.clone();
    glib::spawn_future_local(async move {
        let result = handle.await;
        state.scan_button.set_sensitive(true);
        match result {
            Ok(Ok(res)) => show_results(&state, res),
            Ok(Err(e)) => {
                event(&state, format!("Skanowanie nie powiodło się: {e}"));
                state.content.set_content(Some(&status_page(
                    "dialog-error-symbolic",
                    "Skanowanie nie powiodło się",
                    &e.to_string(),
                )));
            }
            Err(_) => state.content.set_content(Some(&status_page(
                "dialog-error-symbolic",
                "Skanowanie przerwane",
                "",
            ))),
        }
    });
}

fn show_results(state: &Rc<State>, res: scan::ScanResult) {
    state.restore_button.set_visible(res.hidden > 0);
    state
        .restore_button
        .set_label(&format!("Przywróć ukryte ({})", res.hidden));
    let blocked = res.found.iter().filter(|f| f.blocked.is_some()).count();
    let errors: Vec<String> = res
        .found
        .iter()
        .flat_map(|f| f.errors.iter().cloned())
        .collect();
    let mut msg = format!(
        "Skan: {} · {} · {} ms · zablokowane (program działa): {blocked} · ukryte: {}",
        pl(res.found.len() as u64, "pozycja", "pozycje", "pozycji"),
        format_size(res_total(&res.found)),
        res.elapsed.as_millis(),
        res.hidden
    );
    for e in &errors {
        msg.push_str(&format!("\nbłąd odczytu: {e}"));
    }
    event(state, msg);
    let mut found = res.found;
    if found.is_empty() {
        state.rows.borrow_mut().clear();
        state.content.set_content(Some(&status_page(
            "emblem-ok-symbolic",
            "Czysto",
            "Nie znaleziono nic do wyczyszczenia.",
        )));
        update_summary(state);
        return;
    }
    found.sort_by_key(|f| std::cmp::Reverse(f.usage.bytes));
    let page = adw::PreferencesPage::new();
    let mut rows = Vec::new();
    for cat in Category::ALL {
        let in_cat: Vec<&Found> = found.iter().filter(|f| f.item.category == cat).collect();
        if in_cat.is_empty() {
            continue;
        }
        let total: u64 = in_cat.iter().map(|f| f.usage.bytes).sum();
        let group = adw::PreferencesGroup::builder()
            .title(cat.label())
            .description(format_size(total))
            .build();
        for f in in_cat {
            let (row, check) = item_row(f);
            let info = gtk::Button::from_icon_name("view-more-symbolic");
            info.set_valign(gtk::Align::Center);
            info.set_tooltip_text(Some("Szczegóły"));
            info.add_css_class("flat");
            let (s, fc) = (state.clone(), f.clone());
            info.connect_clicked(move |_| show_details(&s, &fc));
            row.add_suffix(&info);
            let s = state.clone();
            check.connect_toggled(move |_| update_summary(&s));
            group.add(&row);
            rows.push(Row {
                found: f.clone(),
                check,
            });
        }
        page.add(&group);
    }
    *state.rows.borrow_mut() = rows;
    state.content.set_content(Some(&page));
    update_summary(state);
    state.toasts.add_toast(adw::Toast::new(&format!(
        "Znaleziono {} w {} ms",
        format_size(res_total(&found)),
        res.elapsed.as_millis()
    )));
}

fn res_total(found: &[Found]) -> u64 {
    found.iter().map(|f| f.usage.bytes).sum()
}

fn item_row(f: &Found) -> (adw::ActionRow, gtk::CheckButton) {
    let it = &f.item;
    let mut subtitle = format!(
        "{}\n{} · {}",
        it.description,
        it.regen.label(),
        it.risk.label()
    );
    if let Some(b) = &f.blocked {
        subtitle = format!("{subtitle}\n⚠ {b}");
    }
    for e in f.errors.iter().take(2) {
        subtitle = format!("{subtitle}\nBłąd: {e}");
    }
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&it.name).as_str())
        .subtitle(glib::markup_escape_text(&subtitle).as_str())
        .subtitle_lines(4)
        .build();
    row.set_tooltip_text(Some(&target_list(it)));
    let check = gtk::CheckButton::new();
    check.set_valign(gtk::Align::Center);
    check.set_active(f.blocked.is_none() && it.default_selected());
    check.set_sensitive(f.blocked.is_none());
    row.add_prefix(&check);
    row.set_activatable_widget(Some(&check));
    let size = gtk::Label::new(Some(&format_size(f.usage.bytes)));
    size.add_css_class("numeric");
    row.add_suffix(&size);
    (row, check)
}

fn target_list(it: &Item) -> String {
    it.targets
        .iter()
        .map(|(p, a)| xcleaner_core::rules::target_label(p, a))
        .collect::<Vec<_>>()
        .join("\n")
}

fn selected(state: &State) -> Vec<Found> {
    state
        .rows
        .borrow()
        .iter()
        .filter(|r| r.check.is_active() && r.check.is_sensitive())
        .map(|r| r.found.clone())
        .collect()
}

fn update_summary(state: &State) {
    let sel = selected(state);
    let bytes: u64 = sel.iter().map(|f| f.usage.bytes).sum();
    state.clean_button.set_sensitive(!sel.is_empty());
    state.summary.set_text(&if sel.is_empty() {
        "Nic nie zaznaczono".to_string()
    } else {
        format!(
            "Zaznaczono {} · {}",
            pl(sel.len() as u64, "pozycję", "pozycje", "pozycji"),
            format_size(bytes)
        )
    });
}

fn confirm_clean(state: &Rc<State>) {
    let sel = selected(state);
    if sel.is_empty() {
        return;
    }
    let bytes: u64 = sel.iter().map(|f| f.usage.bytes).sum();
    let names: Vec<String> = sel
        .iter()
        .take(12)
        .map(|f| format!("• {}", f.item.name))
        .collect();
    let more = if sel.len() > 12 {
        format!("\n… i {} więcej", sel.len() - 12)
    } else {
        String::new()
    };
    let leftovers = sel
        .iter()
        .filter(|f| f.item.category == Category::Leftovers)
        .count();
    let note = if leftovers > 0 {
        format!(
            "Usunięcia nie można cofnąć. {} usuwa całe katalogi programów, których już nie ma w systemie.",
            pl(leftovers as u64, "zaznaczona pozostałość", "zaznaczone pozostałości", "zaznaczonych pozostałości")
        )
    } else {
        "Usunięcia nie można cofnąć. Konfiguracja i dane osobiste nie są ruszane.".to_string()
    };
    let root = sel.iter().any(|f| f.item.category == Category::SystemRoot);
    let note = if root {
        format!("{note}\n\nDo czyszczenia systemu otworzy się terminal — wpisz tam hasło sudo.")
    } else {
        note
    };
    let dialog = adw::AlertDialog::new(
        Some(&format!("Usunąć {}?", format_size(bytes))),
        Some(&format!("{}{more}\n\n{note}", names.join("\n"))),
    );
    dialog.add_response("cancel", "Anuluj");
    dialog.add_response("clean", "Wyczyść");
    dialog.set_response_appearance("clean", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let s = state.clone();
    dialog.connect_response(None, move |_, resp| {
        if resp == "clean" {
            run_clean(&s, sel.iter().map(|f| f.item.clone()).collect());
        }
    });
    dialog.present(Some(&state.window));
}

fn run_clean(state: &Rc<State>, items: Vec<Item>) {
    state.scan_button.set_sensitive(false);
    state.clean_button.set_sensitive(false);
    state.content.set_content(Some(&busy_page("Czyszczenie…")));
    let layout = state.layout.clone();
    event(
        state,
        format!(
            "Czyszczenie: {}",
            items
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    let handle = gtk::gio::spawn_blocking(move || scan::clean(&layout, &items));
    let state = state.clone();
    glib::spawn_future_local(async move {
        let msg = match handle.await {
            Ok(Ok(rep)) => {
                let mut log = format!(
                    "Wynik: zwolniono {} · {} · {} · pominięte wpisy: {}",
                    format_size(rep.freed.bytes),
                    pl(rep.freed.files, "plik", "pliki", "plików"),
                    pl(rep.freed.dirs, "katalog", "katalogi", "katalogów"),
                    rep.skipped
                );
                for b in &rep.blocked {
                    log.push_str(&format!("\npominięto: {b}"));
                }
                for e in &rep.errors {
                    log.push_str(&format!("\nbłąd: {e}"));
                }
                event(&state, log);
                let mut m = format!("Zwolniono {}", format_size(rep.freed.bytes));
                if !rep.blocked.is_empty() {
                    m.push_str(&format!(
                        " · pominięto {} (program działa)",
                        rep.blocked.len()
                    ));
                }
                if !rep.errors.is_empty() || rep.skipped > 0 {
                    m.push_str(&format!(
                        " · {} nie usunięto",
                        rep.errors.len() as u64 + rep.skipped
                    ));
                }
                m
            }
            Ok(Err(e)) => format!("Błąd: {e}"),
            Err(_) => "Czyszczenie przerwane".into(),
        };
        state.toasts.add_toast(adw::Toast::new(&msg));
        start_scan(&state);
    });
}

fn show_details(state: &Rc<State>, f: &Found) {
    let it = &f.item;
    let page = adw::PreferencesPage::new();
    let about = adw::PreferencesGroup::builder()
        .title(glib::markup_escape_text(&it.name).as_str())
        .description(
            glib::markup_escape_text(&format!(
                "{}\n{} · {} · {}",
                it.description,
                it.regen.label(),
                it.risk.label(),
                format_size(f.usage.bytes)
            ))
            .as_str(),
        )
        .build();
    page.add(&about);
    let paths = adw::PreferencesGroup::builder().title("Ścieżki").build();
    for (rel, action) in it
        .targets
        .iter()
        .filter(|(_, a)| matches!(a, xcleaner_core::rules::Action::Root(_)))
    {
        paths.add(
            &adw::ActionRow::builder()
                .title(xcleaner_core::rules::target_label(rel, action))
                .subtitle("katalog systemowy — czyszczony przez sudo")
                .build(),
        );
    }
    for (rel, u) in &f.parts {
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&format!("~/{}", rel.display())).as_str())
            .subtitle(format!(
                "{} · {} · {}",
                format_size(u.bytes),
                pl(u.files, "plik", "pliki", "plików"),
                pl(u.dirs, "katalog", "katalogi", "katalogów")
            ))
            .build();
        let open = gtk::Button::from_icon_name("folder-open-symbolic");
        open.set_valign(gtk::Align::Center);
        open.add_css_class("flat");
        open.set_tooltip_text(Some("Otwórz w menedżerze plików"));
        let abs = state.layout.home.join(rel);
        let win = state.window.clone();
        open.connect_clicked(move |_| {
            let target = if abs.is_dir() {
                abs.clone()
            } else {
                abs.parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| abs.clone())
            };
            open_in_file_manager(&win, &target);
        });
        row.add_suffix(&open);
        paths.add(&row);
    }
    page.add(&paths);
    let actions = adw::PreferencesGroup::new();
    let hide = gtk::Button::with_label("Nie ruszaj tego — ukryj na stałe");
    hide.set_halign(gtk::Align::Center);
    hide.add_css_class("pill");
    actions.add(&hide);
    page.add(&actions);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&page));
    let dialog = adw::Dialog::builder()
        .title("Szczegóły")
        .content_width(640)
        .content_height(520)
        .child(&view)
        .build();
    let (s, id, d) = (state.clone(), it.id.clone(), dialog.clone());
    hide.connect_clicked(move |_| {
        match KeepList::add(&id) {
            Ok(()) => event(&s, format!("Ukryto na stałe: {id}")),
            Err(e) => event(&s, format!("Nie udało się zapisać listy ukrytych: {e}")),
        }
        d.close();
        start_scan(&s);
    });
    dialog.present(Some(&state.window));
}

fn events_markdown(state: &State) -> String {
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                l.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        })
        .unwrap_or_default();
    let mut md = format!(
        "# XCleaner — zdarzenia\n\n- wersja: {}\n- GTK: {}.{}.{} · libadwaita {}.{}.{}\n- system: {os}\n- kernel: {}\n- sesja: {}\n- historia czyszczeń: ~/.local/state/xcleaner/history.log\n\n## Dziennik\n\n",
        env!("CARGO_PKG_VERSION"),
        gtk::major_version(),
        gtk::minor_version(),
        gtk::micro_version(),
        adw::major_version(),
        adw::minor_version(),
        adw::micro_version(),
        kernel.trim(),
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
    );
    for l in state.events.borrow().iter() {
        md.push_str(l);
        md.push('\n');
    }
    // Never leak the user name in a report meant to be shared.
    let home = state.layout.home.to_string_lossy().to_string();
    md.replace(&home, "~")
}

fn show_events(state: &Rc<State>) {
    let text = events_markdown(state);
    let buffer = gtk::TextBuffer::new(None);
    buffer.set_text(&text);
    let tv = gtk::TextView::with_buffer(&buffer);
    tv.set_editable(false);
    tv.set_monospace(true);
    tv.set_wrap_mode(gtk::WrapMode::WordChar);
    tv.set_margin_start(12);
    tv.set_margin_end(12);
    tv.set_margin_top(12);
    tv.set_margin_bottom(12);
    let scroll = gtk::ScrolledWindow::builder()
        .child(&tv)
        .vexpand(true)
        .build();
    let header = adw::HeaderBar::new();
    let copy = gtk::Button::with_label("Kopiuj");
    let save = gtk::Button::with_label("Zapisz .md");
    header.pack_start(&copy);
    header.pack_start(&save);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&scroll));
    let dialog = adw::Dialog::builder()
        .title("Zdarzenia")
        .content_width(760)
        .content_height(560)
        .child(&view)
        .build();
    let (t, s) = (text.clone(), state.clone());
    copy.connect_clicked(move |b| {
        b.clipboard().set_text(&t);
        s.toasts.add_toast(adw::Toast::new("Skopiowano do schowka"));
    });
    let (t, s) = (text, state.clone());
    save.connect_clicked(move |_| {
        let fd = gtk::FileDialog::builder()
            .title("Zapisz zdarzenia")
            .initial_name("xcleaner-zdarzenia.md")
            .build();
        let (t, s2) = (t.clone(), s.clone());
        fd.save(Some(&s.window), None::<&gtk::gio::Cancellable>, move |r| {
            if let Ok(file) = r {
                if let Some(p) = file.path() {
                    let msg = match std::fs::write(&p, &t) {
                        Ok(()) => format!("Zapisano {}", p.display()),
                        Err(e) => format!("Błąd zapisu: {e}"),
                    };
                    s2.toasts.add_toast(adw::Toast::new(&msg));
                }
            }
        });
    });
    dialog.present(Some(&state.window));
}

struct UsageView {
    home: std::path::PathBuf,
    view: adw::ToolbarView,
    title: adw::WindowTitle,
    back: gtk::Button,
    path: RefCell<Vec<std::path::PathBuf>>,
    window: adw::ApplicationWindow,
}

fn show_usage(state: &Rc<State>) {
    let header = adw::HeaderBar::new();
    let back = gtk::Button::from_icon_name("go-previous-symbolic");
    back.set_tooltip_text(Some("Wróć"));
    header.pack_start(&back);
    let title = adw::WindowTitle::new("Co zajmuje miejsce", "~");
    header.set_title_widget(Some(&title));
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    let dialog = adw::Dialog::builder()
        .title("Co zajmuje miejsce")
        .content_width(720)
        .content_height(640)
        .child(&view)
        .build();
    let uv = Rc::new(UsageView {
        home: state.layout.home.clone(),
        view,
        title,
        back: back.clone(),
        path: RefCell::new(vec![std::path::PathBuf::new()]),
        window: state.window.clone(),
    });
    let u = uv.clone();
    back.connect_clicked(move |_| {
        let target = {
            let mut p = u.path.borrow_mut();
            if p.len() > 1 {
                p.pop();
            }
            p.last().cloned().unwrap_or_default()
        };
        load_usage(&u, target);
    });
    load_usage(&uv, std::path::PathBuf::new());
    event(state, "Otwarto „Co zajmuje miejsce”");
    dialog.present(Some(&state.window));
}

fn load_usage(uv: &Rc<UsageView>, rel: std::path::PathBuf) {
    uv.back.set_sensitive(uv.path.borrow().len() > 1);
    uv.title.set_subtitle(&format!("~/{}", rel.display()));
    uv.view.set_content(Some(&busy_page("Liczenie rozmiarów…")));
    let (home, r) = (uv.home.clone(), rel.clone());
    let handle = gtk::gio::spawn_blocking(move || xcleaner_core::usage::breakdown(&home, &r));
    let uv = uv.clone();
    glib::spawn_future_local(async move {
        let b = match handle.await {
            Ok(Ok(b)) => b,
            Ok(Err(e)) => {
                uv.view.set_content(Some(&status_page(
                    "dialog-error-symbolic",
                    "Nie można odczytać",
                    &e.to_string(),
                )));
                return;
            }
            Err(_) => return,
        };
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::builder()
            .title(glib::markup_escape_text(&format!("~/{}", b.rel.display())).as_str())
            .description(format!(
                "{} · {} · {} ms",
                format_size(b.total),
                pl(b.entries.len() as u64, "pozycja", "pozycje", "pozycji"),
                b.elapsed.as_millis()
            ))
            .build();
        for e in b.entries.iter().take(300) {
            let mut subtitle = format!(
                "{} · {}",
                pl(e.usage.files, "plik", "pliki", "plików"),
                pl(
                    e.usage.dirs.saturating_sub(e.is_dir as u64),
                    "katalog",
                    "katalogi",
                    "katalogów"
                )
            );
            if e.is_symlink {
                subtitle = "dowiązanie (nie jest liczone)".into();
            }
            if let Some(n) = &e.note {
                subtitle.push_str(&format!(" · {n}"));
            }
            let row = adw::ActionRow::builder()
                .title(
                    glib::markup_escape_text(&format!(
                        "{}{}",
                        e.name,
                        if e.is_dir { "/" } else { "" }
                    ))
                    .as_str(),
                )
                .subtitle(glib::markup_escape_text(&subtitle).as_str())
                .build();
            let bar = gtk::LevelBar::for_interval(0.0, 1.0);
            bar.set_value(if b.total > 0 {
                e.usage.bytes as f64 / b.total as f64
            } else {
                0.0
            });
            bar.set_size_request(90, -1);
            bar.set_valign(gtk::Align::Center);
            // Plain proportion, not a warning meter.
            bar.remove_offset_value(Some(gtk::LEVEL_BAR_OFFSET_LOW));
            bar.remove_offset_value(Some(gtk::LEVEL_BAR_OFFSET_HIGH));
            bar.remove_offset_value(Some(gtk::LEVEL_BAR_OFFSET_FULL));
            row.add_prefix(&bar);
            let size = gtk::Label::new(Some(&format_size(e.usage.bytes)));
            size.add_css_class("numeric");
            row.add_suffix(&size);
            let open = gtk::Button::from_icon_name("folder-open-symbolic");
            open.set_valign(gtk::Align::Center);
            open.add_css_class("flat");
            open.set_tooltip_text(Some("Otwórz w menedżerze plików"));
            let abs = uv.home.join(&e.rel);
            let win = uv.window.clone();
            open.connect_clicked(move |_| {
                let target = if abs.is_dir() {
                    abs.clone()
                } else {
                    abs.parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| abs.clone())
                };
                open_in_file_manager(&win, &target);
            });
            row.add_suffix(&open);
            if e.is_dir && !e.is_symlink {
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                row.set_activatable(true);
                let (u, r) = (uv.clone(), e.rel.clone());
                row.connect_activated(move |_| {
                    u.path.borrow_mut().push(r.clone());
                    load_usage(&u, r.clone());
                });
            }
            group.add(&row);
        }
        page.add(&group);
        uv.view.set_content(Some(&page));
    });
}
