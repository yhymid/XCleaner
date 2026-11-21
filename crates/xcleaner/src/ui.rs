//! Main window: scan → grouped checklist → one confirmation → clean.

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use std::{cell::RefCell, rc::Rc};
use xcleaner_core::{
    format_size, pl,
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
    });

    let s = state.clone();
    scan_button.connect_clicked(move |_| start_scan(&s));
    let s = state.clone();
    clean_button.connect_clicked(move |_| confirm_clean(&s));
    window.present();
    start_scan(&state);
}

fn status_page(icon: &str, title: &str, desc: &str) -> adw::StatusPage {
    adw::StatusPage::builder()
        .icon_name(icon)
        .title(title)
        .description(desc)
        .build()
}

fn busy_page(title: &str) -> gtk::Widget {
    let spinner = adw::Spinner::new();
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
            Ok(Err(e)) => state.content.set_content(Some(&status_page(
                "dialog-error-symbolic",
                "Skanowanie nie powiodło się",
                &e.to_string(),
            ))),
            Err(_) => state.content.set_content(Some(&status_page(
                "dialog-error-symbolic",
                "Skanowanie przerwane",
                "",
            ))),
        }
    });
}

fn show_results(state: &Rc<State>, res: scan::ScanResult) {
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
        .map(|(p, _)| format!("~/{}", p.display()))
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
    let dialog = adw::AlertDialog::new(
        Some(&format!("Usunąć {}?", format_size(bytes))),
        Some(&format!(
            "{}{more}\n\nUsunięcia nie można cofnąć. Konfiguracja i dane osobiste nie są ruszane.",
            names.join("\n")
        )),
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
    let handle = gtk::gio::spawn_blocking(move || scan::clean(&layout, &items));
    let state = state.clone();
    glib::spawn_future_local(async move {
        let msg = match handle.await {
            Ok(Ok(rep)) => {
                for e in &rep.errors {
                    eprintln!("xcleaner: {e}");
                }
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
