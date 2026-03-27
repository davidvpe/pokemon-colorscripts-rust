include!(concat!(env!("OUT_DIR"), "/pokemon_data.rs"));

use std::env;
use std::io::{self, BufWriter, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

// Cumulative Pokedex counts per generation (exclusive end index into ALL_POKEMON).
// Gen 1 = indices 0..151, Gen 2 = 151..251, etc.
const GEN_END: &[usize] = &[151, 251, 386, 493, 649, 721, 809, 905, 1025];

fn generation_range(gen: usize) -> (usize, usize) {
    let total = ALL_POKEMON.len();
    let start = if gen <= 1 { 0 } else { GEN_END[gen - 2].min(total) };
    let end = GEN_END.get(gen - 1).copied().unwrap_or(total).min(total);
    (start, end)
}

fn print_help() {
    println!("Description: CLI utility to print out unicode image of a pokemon in your shell");
    println!();
    println!("Usage: pokemon-colorscripts [OPTION] [POKEMON NAME]");
    println!("  {:<20}\t{}", "-h, --help",       "Print this help.");
    println!("  {:<20}\t{}", "-l, --list",        "Print list of all pokemon");
    println!("  {:<20}\t{}", "-r, --random [N]",  "Show a random pokemon (optional: generation 1-9)");
    println!("  {:<20}\t{}", "-n, --name <name>", "Select pokemon by name");
    println!("  {:<20}\t{}", "--no-title",         "Do not print the pokemon name");
    println!("  {:<20}\t{}", "-i, --info",         "Show side panel with types and base stats");
    println!("  {:<20}\t{}", "-p, --pokedex",      "Show full Pokédex view (sprite + stats in one frame)");
    println!();
    println!("Examples:");
    println!("  pokemon-colorscripts --name pikachu");
    println!("  pokemon-colorscripts --random");
    println!("  pokemon-colorscripts --name charizard --info");
    println!("  pokemon-colorscripts --name charizard --pokedex");
    println!("  pokemon-colorscripts --random 1 --pokedex");
}

/// Returns the number of columns in the current terminal, or 80 as fallback.
fn terminal_width() -> usize {
    #[cfg(unix)]
    {
        #[repr(C)]
        struct Winsize { rows: u16, cols: u16, xpixel: u16, ypixel: u16 }
        unsafe {
            #[allow(improper_ctypes)]
            extern "C" { fn ioctl(fd: i32, request: u64, ...) -> i32; }
            let mut ws = Winsize { rows: 0, cols: 0, xpixel: 0, ypixel: 0 };
            #[cfg(target_os = "macos")]
            let tiocgwinsz: u64 = 0x4008_7468;
            #[cfg(not(target_os = "macos"))]
            let tiocgwinsz: u64 = 0x5413;
            if ioctl(1, tiocgwinsz, &mut ws as *mut Winsize) == 0 && ws.cols > 0 {
                return ws.cols as usize;
            }
        }
    }
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(80)
}

/// Returns the number of visible terminal columns in `s`, ignoring ANSI escape
/// sequences of the form `\x1b[...m`.
fn visible_len(s: &str) -> usize {
    let mut len = 0usize;
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            len += 1;
        }
    }
    len
}

/// Builds a proportional bar of `width` characters using `█` / `░`.
fn make_bar(val: u32, max: u32, width: usize) -> String {
    let filled = ((val as f64 / max as f64) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Returns an ANSI color code matching a Pokemon type name.
fn type_color(t: &str) -> &'static str {
    match t {
        "Normal"   => "\x1b[37m",
        "Fire"     => "\x1b[38;5;202m",
        "Water"    => "\x1b[34;1m",
        "Electric" => "\x1b[33;1m",
        "Grass"    => "\x1b[32;1m",
        "Ice"      => "\x1b[96m",
        "Fighting" => "\x1b[31;1m",
        "Poison"   => "\x1b[35m",
        "Ground"   => "\x1b[33m",
        "Flying"   => "\x1b[94m",
        "Psychic"  => "\x1b[35;1m",
        "Bug"      => "\x1b[32m",
        "Rock"     => "\x1b[33;2m",
        "Ghost"    => "\x1b[35;2m",
        "Dragon"   => "\x1b[34;1m",
        "Dark"     => "\x1b[90;1m",
        "Steel"    => "\x1b[37;1m",
        "Fairy"    => "\x1b[95m",
        _          => "\x1b[37m",
    }
}

/// Returns a color for a stat bar based on the stat's magnitude.
fn stat_bar_color(val: u32) -> &'static str {
    if val < 50       { "\x1b[31m"  }  // red   — poor
    else if val < 80  { "\x1b[33m"  }  // yellow — below avg
    else if val < 110 { "\x1b[32m"  }  // green  — good
    else              { "\x1b[92m"  }  // bright green — great
}

/// Builds the stats panel lines from the embedded stats text.
///
/// Uses double-line box-drawing characters and ANSI colors for a mini-Pokédex
/// aesthetic. Width adapts to the longest content line (e.g. long display
/// names) so the panel never exceeds a sensible size.
///
/// Expected format of `stats_text`:
/// ```
/// #025
/// Pikachu
/// Electric
/// HP:35
/// Atk:55
/// Def:40
/// SpA:50
/// SpD:50
/// Spe:90
/// ```
fn build_stats_panel(stats_text: &str) -> Vec<String> {
    let mut iter = stats_text.lines();
    let dex_num      = iter.next().unwrap_or("#???");
    let display_name = iter.next().unwrap_or("???");
    let types_raw    = iter.next().unwrap_or("");

    let stat_labels = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];
    let mut stat_values: Vec<u32> = Vec::new();
    for line in iter {
        if let Some(colon) = line.find(':') {
            if let Ok(val) = line[colon + 1..].trim().parse::<u32>() {
                stat_values.push(val);
            }
        }
    }

    let bar_width = 15usize;
    let bst: u32 = stat_values.iter().sum();

    // Colored types: "Fire/Flying" → "🟠Fire / 🔵Flying" (via ANSI)
    let types_colored: String = types_raw.split('/')
        .enumerate()
        .map(|(i, t)| {
            let t = t.trim();
            let sep = if i == 0 { "" } else { " / " };
            format!("{}{}{}\x1b[0m", sep, type_color(t), t)
        })
        .collect();

    enum Row { Text(String), Sep }
    let mut rows: Vec<Row> = Vec::new();

    // Header: dimmed dex number · bold display name
    rows.push(Row::Text(format!(
        " \x1b[2m{}\x1b[0m · \x1b[1m{}\x1b[0m",
        dex_num, display_name
    )));
    // Types row
    rows.push(Row::Text(format!(" {}", types_colored)));
    rows.push(Row::Sep);
    // One row per stat
    for (i, &val) in stat_values.iter().enumerate() {
        let label = stat_labels.get(i).copied().unwrap_or("???");
        let bar   = make_bar(val, 255, bar_width);
        let color = stat_bar_color(val);
        rows.push(Row::Text(format!(
            " \x1b[2m{:<4}\x1b[0m{}{:>3}  {}\x1b[0m",
            label, color, val, bar
        )));
    }
    rows.push(Row::Sep);
    // BST row
    rows.push(Row::Text(format!(
        " \x1b[2mBST\x1b[0m \x1b[1m{:>3}\x1b[0m",
        bst
    )));

    // Measure inner width using visible_len so ANSI codes are not counted.
    let inner_w = rows.iter()
        .filter_map(|row| if let Row::Text(s) = row { Some(visible_len(s)) } else { None })
        .max()
        .unwrap_or(20)
        + 1; // one trailing space before right border

    // Double-line box borders in bold white for a Pokédex feel
    let eq_str = "═".repeat(inner_w);
    let top = format!("\x1b[1;37m╔{}╗\x1b[0m", eq_str);
    let mid = format!("\x1b[1;37m╠{}╣\x1b[0m", eq_str);
    let bot = format!("\x1b[1;37m╚{}╝\x1b[0m", eq_str);

    let mut panel: Vec<String> = Vec::new();
    panel.push(top);
    for row in &rows {
        match row {
            Row::Sep => panel.push(mid.clone()),
            Row::Text(s) => {
                let pad = inner_w - visible_len(s);
                panel.push(format!(
                    "\x1b[1;37m║\x1b[0m{}{}\x1b[1;37m║\x1b[0m",
                    s, " ".repeat(pad)
                ));
            }
        }
    }
    panel.push(bot);
    panel
}

/// Extracts the display name (line 2) from embedded stats text, falling back
/// to the slug `name` if stats are not available.
fn display_name_from_stats(name: &str, stats_text: Option<&str>) -> String {
    stats_text
        .and_then(|t| t.lines().nth(1))
        .filter(|s| !s.is_empty())
        .unwrap_or(name)
        .to_string()
}

/// Renders the pokemon art with an optional side stats panel.
///
/// When `show_info` is true and stats are embedded for this pokemon, the panel
/// is printed to the right of the sprite, vertically centered.
fn render_with_info(name: &str, art: &str, show_info: bool, no_title: bool) {
    let stats_text = get_pokemon_stats(name);

    let stats_panel: Vec<String> = if show_info {
        match stats_text {
            Some(text) => build_stats_panel(text),
            None => vec![],
        }
    } else {
        vec![]
    };

    // Use the proper display name (from stats) for the title line
    let title = display_name_from_stats(name, stats_text);

    if stats_panel.is_empty() {
        if !no_title {
            println!("{}", title);
        }
        print!("{}", art);
        return;
    }

    if !no_title {
        println!("{}", title);
    }

    // art is "\n[row0]\n[row1]\n...\n[rowN]\x1b[0m\n"
    let all_parts: Vec<&str> = art.split('\n').collect();
    let art_lines: Vec<&str> = if all_parts.len() >= 2 {
        all_parts[1..all_parts.len() - 1].to_vec()
    } else {
        vec![]
    };

    let max_art_width = art_lines.iter().map(|l| visible_len(l)).max().unwrap_or(0);

    // Vertically center the stats panel relative to the sprite
    let art_height = art_lines.len();
    let panel_height = stats_panel.len();
    let v_offset = art_height.saturating_sub(panel_height) / 2;

    let gap = 3usize;
    let total = art_height.max(v_offset + panel_height);

    for i in 0..total {
        let art_line = art_lines.get(i).copied().unwrap_or("");
        let panel_line = i
            .checked_sub(v_offset)
            .and_then(|idx| stats_panel.get(idx))
            .map(String::as_str)
            .unwrap_or("");

        let vis = visible_len(art_line);
        let padding = max_art_width + gap - vis;
        // Trim any trailing reset from the art line, then reset before padding
        let art_clean = art_line.trim_end_matches("\x1b[0m");
        println!("{}\x1b[0m{}{}", art_clean, " ".repeat(padding), panel_line);
    }
}

/// Full-frame Pokédex view: sprite on the left, info panel on the right.
/// The right panel expands to fill the terminal width.
///
/// Color scheme:
///   - Outer frame ────── bold red     (Pokédex body)
///   - Screen divider ─── bright cyan  (the screen edge)
///   - Header title ───── bright green (retro pixel vibe)
///   - Types / bars ───── per-type / per-value colors
fn render_pokedex(name: &str, art: &str) {
    let stats_text = match get_pokemon_stats(name) {
        Some(t) => t,
        None => {
            println!("{}", name);
            print!("{}", art);
            return;
        }
    };

    // ── Parse stats ────────────────────────────────────────────────────
    let mut iter = stats_text.lines();
    let dex_num      = iter.next().unwrap_or("#???");
    let display_name = iter.next().unwrap_or(name);
    let types_raw    = iter.next().unwrap_or("");

    let stat_labels = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];
    let mut stat_values: Vec<u32> = Vec::new();
    for line in iter {
        if let Some(colon) = line.find(':') {
            if let Ok(val) = line[colon + 1..].trim().parse::<u32>() {
                stat_values.push(val);
            }
        }
    }
    let bst: u32 = stat_values.iter().sum();

    // ── Parse art lines ────────────────────────────────────────────────
    let all_parts: Vec<&str> = art.split('\n').collect();
    let art_lines: Vec<&str> = if all_parts.len() >= 2 {
        all_parts[1..all_parts.len() - 1].to_vec()
    } else {
        vec![]
    };
    let max_art_w = art_lines.iter().map(|l| visible_len(l)).max().unwrap_or(0);

    // ── Layout: right panel fills available terminal width ─────────────
    let term_w = terminal_width();
    let l_pad  = 2usize;
    let left_w = max_art_w + l_pad * 2;
    // Frame: ║ + left_w + ╦ + right_w + ║ = term_w  →  right_w = term_w − left_w − 3
    let right_w = term_w.saturating_sub(left_w + 3).max(30);
    // Stat row: "  {label:<4} {val:>3}  {bar} " — fixed chars = 13, remainder = bar
    let bar_w = right_w.saturating_sub(13).max(5);

    // ── Colored types ──────────────────────────────────────────────────
    let types_colored: String = types_raw.split('/')
        .enumerate()
        .map(|(i, t)| {
            let t = t.trim();
            let sep = if i == 0 { "" } else { " / " };
            format!("{}{}{}\x1b[0m", sep, type_color(t), t)
        })
        .collect();
    let types_vis = visible_len(&types_colored);

    // ── Right panel rows ───────────────────────────────────────────────
    enum RRow { Text(String), Sep }
    let mut rrows: Vec<RRow> = Vec::new();

    // Row 0: Pokémon name (bold, prominent)
    rrows.push(RRow::Text(format!("  \x1b[1;97m{}\x1b[0m", display_name)));

    // Row 1: dex number (left) + type badge (right-aligned)
    {
        let left_part = format!("  \x1b[2m{}\x1b[0m", dex_num);
        let left_vis  = 2 + dex_num.len();
        // layout: left_part | gap | " " types_colored " "
        // total visible = left_vis + gap + 1 + types_vis + 1 = right_w
        let gap = right_w.saturating_sub(left_vis + types_vis + 2);
        rrows.push(RRow::Text(format!("{}{} {}\x1b[0m ", left_part, " ".repeat(gap), types_colored)));
    }

    rrows.push(RRow::Sep);

    // Rows 3–8: one per stat, bar width adapts to right_w
    for (i, &val) in stat_values.iter().enumerate() {
        let label = stat_labels.get(i).copied().unwrap_or("???");
        let bar   = make_bar(val, 255, bar_w);
        let color = stat_bar_color(val);
        // visible = 2+4+1+3+2+bar_w+1 = bar_w+13 = right_w  → rpad = 0
        rrows.push(RRow::Text(format!(
            "  \x1b[2m{:<4}\x1b[0m {}{:>3}  {}\x1b[0m ",
            label, color, val, bar
        )));
    }

    rrows.push(RRow::Sep);

    // BST row
    rrows.push(RRow::Text(format!("  \x1b[2mBST\x1b[0m  \x1b[1m{}\x1b[0m", bst)));

    // ── Color tokens ───────────────────────────────────────────────────
    let rb  = "\x1b[31;1m";   // bold red    — outer Pokédex body
    let scr = "\x1b[96m";     // bright cyan — screen edge / inner divider
    let rst = "\x1b[0m";

    // ── Pre-computed border strings ────────────────────────────────────
    let total_inner = left_w + 1 + right_w;
    let eq_all   = "═".repeat(total_inner);
    let eq_left  = "═".repeat(left_w);
    let eq_right = "═".repeat(right_w);

    // ── Header ─────────────────────────────────────────────────────────
    let title_str = format!("{}\x1b[92;1m[ POKÉDEX ]{rst}", rst);
    let title_vis = 11usize; // visible chars in "[ POKÉDEX ]"
    let name_str  = format!("{}\x1b[2m{dex_num}{rst} · \x1b[1m{display_name}{rst} ", rst);
    let name_vis  = dex_num.len() + 3 + display_name.len() + 1; // " · " + trailing space
    let hdr_gap   = total_inner.saturating_sub(1 + title_vis + 1 + name_vis);

    // ── Draw ───────────────────────────────────────────────────────────
    println!("{rb}╔{eq_all}╗{rst}");
    println!("{rb}║{rst} {title_str} {}{name_str}{rb}║{rst}", " ".repeat(hdr_gap));
    println!("{rb}╠{eq_left}{scr}╦{rb}{eq_right}╣{rst}");

    let total_rows = art_lines.len().max(rrows.len());
    for i in 0..total_rows {
        let art_line  = art_lines.get(i).copied().unwrap_or("");
        let art_clean = art_line.trim_end_matches("\x1b[0m");
        let art_vis   = visible_len(art_line);
        let r_pad     = left_w.saturating_sub(l_pad + art_vis);
        let left_cell = format!(
            "{}{art_clean}\x1b[0m{}",
            " ".repeat(l_pad),
            " ".repeat(r_pad)
        );

        match rrows.get(i) {
            Some(RRow::Sep) => {
                println!("{rb}║{rst}{left_cell}{scr}╠{rb}{eq_right}╣{rst}");
            }
            Some(RRow::Text(s)) => {
                let rpad = right_w.saturating_sub(visible_len(s));
                println!(
                    "{rb}║{rst}{left_cell}{scr}║{rst}{s}{}{rb}║{rst}",
                    " ".repeat(rpad)
                );
            }
            None => {
                println!("{rb}║{rst}{left_cell}{scr}║{rst}{}{rb}║{rst}", " ".repeat(right_w));
            }
        }
    }

    println!("{rb}╚{eq_left}{scr}╩{rb}{eq_right}╝{rst}");
}

fn show_random(gen: Option<usize>, no_title: bool, show_info: bool, show_pokedex: bool) {
    let (start, end) = match gen {
        None => (0, ALL_POKEMON.len()),
        Some(g) => generation_range(g),
    };

    if start >= end {
        eprintln!("No pokemon available for generation {}.", gen.unwrap_or(0));
        std::process::exit(1);
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    let name = ALL_POKEMON[start + (nanos % (end - start))];

    if let Some(art) = get_pokemon_art(name) {
        if show_pokedex {
            render_pokedex(name, art);
        } else {
            render_with_info(name, art, show_info, no_title);
        }
    }
}

fn show_by_name(name: &str, no_title: bool, show_info: bool, show_pokedex: bool) {
    match get_pokemon_art(name) {
        Some(art) => {
            if show_pokedex {
                render_pokedex(name, art);
            } else {
                render_with_info(name, art, show_info, no_title);
            }
        }
        None => {
            eprintln!("Invalid pokemon '{}'", name);
            std::process::exit(1);
        }
    }
}

fn list_pokemon() {
    if let Ok(mut child) = Command::new("less").stdin(Stdio::piped()).spawn() {
        if let Some(stdin) = child.stdin.take() {
            let mut w = BufWriter::new(stdin);
            for name in ALL_POKEMON {
                let _ = writeln!(w, "{}", name);
            }
        }
        let _ = child.wait();
    } else {
        let stdout = io::stdout();
        let mut w = BufWriter::new(stdout.lock());
        for name in ALL_POKEMON {
            let _ = writeln!(w, "{}", name);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    // Strip flags that can appear anywhere before positional matching.
    let no_title     = args.iter().any(|a| a == "--no-title");
    let show_info    = args.iter().any(|a| a == "--info"    || a == "-i");
    let show_pokedex = args.iter().any(|a| a == "--pokedex" || a == "-p");
    let pos: Vec<&str> = args.iter()
        .filter(|a| {
            let s = a.as_str();
            s != "--no-title" && s != "--info" && s != "-i" && s != "--pokedex" && s != "-p"
        })
        .map(String::as_str)
        .collect();

    match pos.as_slice() {
        [] => print_help(),

        ["-h" | "--help" | "help"] => print_help(),

        ["-l" | "--list" | "list"] => list_pokemon(),

        ["-r" | "--random" | "random"] => show_random(None, no_title, show_info, show_pokedex),

        ["-r" | "--random" | "random", gen] => {
            match gen.parse::<usize>() {
                Ok(g) if (1..=9).contains(&g) => show_random(Some(g), no_title, show_info, show_pokedex),
                _ => {
                    eprintln!("Invalid generation '{}'. Use a number between 1 and 9.", gen);
                    std::process::exit(1);
                }
            }
        }

        ["-n" | "--name" | "name", name] => show_by_name(name, no_title, show_info, show_pokedex),

        _ => {
            eprintln!("Input error. Run with --help for usage.");
            std::process::exit(1);
        }
    }
}
