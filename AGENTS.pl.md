# Instrukcje dla agentów AI piszących kod (XCleaner)

[English](AGENTS.md) | **Polski**

Najpierw przeczytaj README, docs/ROADMAP.md i docs/research/.

## Decyzje właściciela (wiążące)
- Interfejs po angielsku (domyślnie) i po polsku, przełącznik EN/PL w oknie (właściciel 2026-11-22).
  Każdy tekst widoczny dla użytkownika ma obie wersje: `tr!("en", "pl")`, `trf!("en {x}", "pl {x}")`,
  `i18n::T::new(en, pl)` w statycznych tabelach, `i18n::count(n, [en1, enN], [pl1, pl2-4, pl5+])`.
  Nigdy nie dodawaj tekstu tylko w jednym języku. Kod i commity są po angielsku.
- Dokumentacja jest w parach: `X.md` (angielski) i `X.pl.md` (polski), z linkami do siebie
  na górze. Zmieniaj obie naraz.
- Precyzja zamiast agresji: każda reguła wskazuje dokładne ścieżki. Nigdy nie proponuj całego
  katalogu najwyższego poziomu (`~/.config`, `~/.local`, `~/.cache`, katalog Steam) — pilnuje tego `protect.rs`.
- Właściciel 2026-11-22: `~/.cache` można czyścić praktycznie w całości — każdy wpis najwyższego
  poziomu jest osobną, zaznaczoną pozycją, OPRÓCZ katalogów głównie z multimediami (automatycznie
  „do przejrzenia”, prawdziwy przykład `~/.cache/music-opus` = 8,5 GB muzyki) i znanych stanowych (`CACHE_STATEFUL`).
- Właściciel 2026-11-22: pozostałości po odinstalowanych aplikacjach w `~/.config` / `~/.local` muszą
  być znajdowane (deinstalatory zawsze zostawiają resztki) — projekt w docs/research/LEFTOVERS-DETECTION-2026-11.md.
- Z góry zaznaczane są tylko pozycje `Risk::Safe`, a do tego (właściciel 2026-11-22) pozostałości bez
  znaczników danych osobistych (`leftovers::PERSONAL_MARKERS`/`PERSONAL_EXT`). Kosz, historia,
  dane osobiste, prefiksy i profile w pozostałościach nigdy nie są zaznaczane z góry.
- Właściciel 2026-11-22: używaj każdego źródła dowodów i zmierz jego efekt na prawdziwych danych przed
  decyzją (np. aliasy `pacman -Fl`), zamiast zgadywać. Nowe mechanizmy mają przełącznik
  powrotu (`--no-files-db`).
- Usunięcie jest trwałe (bez kwarantanny); okno potwierdzenia to mówi.
- Okno nigdy nie działa jako root (odmawia). Zadania systemowe działają w terminalu przez własny
  pomocnik roota `xcleaner --root-clean` (sudo → doas → su, bez polkit); root dostaje tylko nazwy
  zadań i sam wylicza cele.
- Bez kodu i danych na GPL: reguł BleachBita/Stacera nie wolno kopiować (fakty sprawdzone na
  prawdziwym systemie są w porządku). Zobacz docs/research/UPSTREAM-CLEANERS-2026-11.md §1.5.

## Zasady bezpieczeństwa (obowiązkowe)
- Każde usuwanie idzie przez `xcleaner_core::fsops::Home` (openat2 wewnątrz HOME, bez
  podążania za symlinkami, bez przechodzenia przez punkty montowania, sprawdzanie właściciela).
  Bez `rm -rf` na ścieżkach tekstowych, bez powłoki.
- Reguły dla działającej aplikacji mają blokadę procesu i/lub sprawdzenie SingletonLock.
- Pliki stanu w katalogach cache idą do `Action::ClearDirKeep` (np. `vcs.json` yay).

## Sposób pracy
Przed każdym commitem: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`. Sprawdzaj na prawdziwej maszynie przez `xcleaner --scan` (tylko odczyt).
