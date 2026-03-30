/// Generator: downloads Pokemon sprites from pokemondb.net and converts them
/// to ANSI color escape art, writing one .txt file per Pokemon to colorscripts/.
///
/// Subcommands
/// -----------
///   (none)          Scrape sprites for every name in nameslist.txt
///   update-names    Scrape the Pokemon list from pokemondb and overwrite nameslist.txt
///
/// Flags
///   --force         Regenerate files even if they already exist

use image::{imageops, RgbaImage};
use std::env;
use std::fs;
use std::io::Read;
use std::thread;
use std::time::Duration;

const POKEMONDB_SPRITE_URL: &str = "https://img.pokemondb.net/sprites/sword-shield/icon/";
const POKEMONDB_SPRITES_PAGE: &str = "https://pokemondb.net/sprites";
const POKEMONDB_POKEDEX_URL: &str = "https://pokemondb.net/pokedex/";
const NAMES_FILE: &str = "nameslist.txt";
const OUTPUT_DIR: &str = "colorscripts";
const STATS_DIR: &str = "stats";
/// Uniform transparent padding added on every side after cropping.
const PADDING: u32 = 1;
/// Sprites taller than this (in pixels) are scaled down by 1.5x, matching the
/// Python make_art.py behaviour.
const HEIGHT_THRESHOLD: u32 = 64;
/// Milliseconds between requests — be polite to servers.
const REQUEST_DELAY_MS: u64 = 150;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("update-names") => {
            update_nameslist().unwrap_or_else(|e| {
                eprintln!("Error updating nameslist: {}", e);
                std::process::exit(1);
            });
        }
        Some("update-stats") => {
            let force = args.iter().any(|a| a == "--force");
            scrape_stats(force).unwrap_or_else(|e| {
                eprintln!("Error updating stats: {}", e);
                std::process::exit(1);
            });
        }
        Some("--help") | Some("-h") => print_usage(),
        Some("inspect") => {
            let name = args.get(1).map(String::as_str).unwrap_or_else(|| {
                eprintln!("Usage: generator inspect <pokemon>");
                std::process::exit(1);
            });
            inspect_sprite(name).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
        }
        Some(name) if !name.starts_with('-') => {
            let name = name.to_string();
            scrape_one(&name).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
        }
        _ => {
            let force = args.iter().any(|a| a == "--force");
            scrape(force).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
        }
    }
}

fn print_usage() {
    eprintln!("Usage: generator [SUBCOMMAND] [OPTIONS]");
    eprintln!();
    eprintln!("Subcommands:");
    eprintln!("  <name>          Force-regenerate sprite and stats for a single pokemon");
    eprintln!("  update-names    Scrape pokemon list from pokemondb and update nameslist.txt");
    eprintln!("  update-stats    Scrape base stats from pokemondb and update stats/");
    eprintln!("  (none)          Download sprites, generate colorscripts, and scrape stats");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --force         Regenerate files even if they already exist");
}

// ---------------------------------------------------------------------------
// update-names subcommand
// ---------------------------------------------------------------------------

/// Scrapes pokemondb.net/sprites and writes every Pokemon slug to nameslist.txt,
/// one per line, in page order (by Pokedex number).
fn update_nameslist() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Scraping Pokemon list from {}...", POKEMONDB_SPRITES_PAGE);

    let html = http_get_string(POKEMONDB_SPRITES_PAGE)?;
    let names = extract_pokemon_slugs(&html);

    if names.is_empty() {
        return Err("No Pokemon found — page structure may have changed".into());
    }

    let content = names.join("\n") + "\n";
    fs::write(NAMES_FILE, &content)?;

    eprintln!("Wrote {} Pokemon names to {}.", names.len(), NAMES_FILE);
    Ok(())
}

/// Extracts Pokemon slugs from pokemondb HTML.
///
/// Each Pokemon infocard looks like:
///   <a class="infocard" href="/sprites/bulbasaur">...</a>
///
/// We grab the path segment after "/sprites/" and skip any non-Pokemon
/// hrefs (section anchors, sub-pages with extra slashes, etc.).
fn extract_pokemon_slugs(html: &str) -> Vec<String> {
    let prefix = "href=\"/sprites/";
    let mut names: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut pos = 0;

    while let Some(rel) = html[pos..].find(prefix) {
        let start = pos + rel + prefix.len();
        if let Some(end) = html[start..].find('"') {
            let slug = &html[start..start + end];
            if !slug.contains('/') && !slug.contains('#') && !slug.is_empty() {
                if seen.insert(slug.to_string()) {
                    names.push(slug.to_string());
                }
            }
            pos = start + end;
        } else {
            break;
        }
    }

    names
}

// ---------------------------------------------------------------------------
// scrape-one subcommand
// ---------------------------------------------------------------------------

/// Downloads the raw sprite and prints a summary of alpha values to diagnose missing pixels.
fn inspect_sprite(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let img = download_sprite(name)?.to_rgba8();
    let (w, h) = img.dimensions();
    println!("Raw sprite: {}x{}", w, h);

    let mut fully_opaque = 0u32;
    let mut semi = 0u32;
    let mut fully_transparent = 0u32;

    for y in 0..h {
        for x in 0..w {
            let a = img.get_pixel(x, y)[3];
            match a {
                255 => fully_opaque += 1,
                0   => fully_transparent += 1,
                _   => {
                    let [r, g, b, _] = img.get_pixel(x, y).0;
                    println!("  semi-transparent ({},{}) rgba=({},{},{},{})", x, y, r, g, b, a);
                    semi += 1;
                }
            }
        }
    }

    println!("fully opaque (alpha=255): {}", fully_opaque);
    println!("semi-transparent (0<a<255): {}", semi);
    println!("fully transparent (alpha=0): {}", fully_transparent);
    Ok(())
}

/// Force-regenerates the colorscript and stats for a single pokemon by name.
fn scrape_one(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let names_content = fs::read_to_string(NAMES_FILE)
        .map_err(|_| format!("{} not found — run from project root", NAMES_FILE))?;
    let index = names_content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .position(|n| n == name)
        .map(|i| i + 1)
        .unwrap_or(1);

    fs::create_dir_all(OUTPUT_DIR)?;
    fs::create_dir_all(STATS_DIR)?;

    eprint!("sprite: {}... ", name);
    match process_pokemon(name) {
        Ok(art) => {
            fs::write(format!("{}/{}.txt", OUTPUT_DIR, name), &art)?;
            eprintln!("ok");
        }
        Err(e) => eprintln!("FAILED: {}", e),
    }

    thread::sleep(Duration::from_millis(REQUEST_DELAY_MS));

    eprint!("stats:  {}... ", name);
    match fetch_pokemon_stats(name, index) {
        Ok(stats) => {
            fs::write(format!("{}/{}.txt", STATS_DIR, name), &stats)?;
            eprintln!("ok");
        }
        Err(e) => eprintln!("FAILED: {}", e),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// scrape subcommand
// ---------------------------------------------------------------------------

fn scrape(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let names_content = fs::read_to_string(NAMES_FILE)
        .map_err(|_| format!("{} not found — run from project root", NAMES_FILE))?;

    fs::create_dir_all(OUTPUT_DIR)?;

    let names: Vec<&str> = names_content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let total = names.len();
    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for (i, name) in names.iter().enumerate() {
        let output_path = format!("{}/{}.txt", OUTPUT_DIR, name);

        if !force && fs::metadata(&output_path).is_ok() {
            skipped += 1;
            continue;
        }

        eprint!("[{}/{}] {}... ", i + 1, total, name);

        match process_pokemon(name) {
            Ok(art) => {
                fs::write(&output_path, &art)?;
                eprintln!("ok");
                ok += 1;
            }
            Err(e) => {
                eprintln!("FAILED: {}", e);
                failed += 1;
            }
        }

        thread::sleep(Duration::from_millis(REQUEST_DELAY_MS));
    }

    eprintln!(
        "\nDone. {} generated, {} skipped, {} failed.",
        ok, skipped, failed
    );

    eprintln!("\nScraping base stats from pokemondb...");
    scrape_stats(force)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// stats subcommand
// ---------------------------------------------------------------------------

/// Scrapes base stats from pokemondb.net/pokedex/{name} for every Pokemon in
/// nameslist.txt and writes one .txt file per Pokemon to stats/.
fn scrape_stats(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let names_content = fs::read_to_string(NAMES_FILE)
        .map_err(|_| format!("{} not found — run from project root", NAMES_FILE))?;

    fs::create_dir_all(STATS_DIR)?;

    let names: Vec<&str> = names_content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    let total = names.len();
    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for (i, name) in names.iter().enumerate() {
        let output_path = format!("{}/{}.txt", STATS_DIR, name);

        if !force && fs::metadata(&output_path).is_ok() {
            skipped += 1;
            continue;
        }

        eprint!("[{}/{}] {} stats... ", i + 1, total, name);

        match fetch_pokemon_stats(name, i + 1) {
            Ok(stats) => {
                fs::write(&output_path, &stats)?;
                eprintln!("ok");
                ok += 1;
            }
            Err(e) => {
                eprintln!("FAILED: {}", e);
                failed += 1;
            }
        }

        thread::sleep(Duration::from_millis(REQUEST_DELAY_MS));
    }

    eprintln!(
        "Stats: {} generated, {} skipped, {} failed.",
        ok, skipped, failed
    );
    Ok(())
}

/// Downloads the pokemondb pokedex page for `name` and extracts the display
/// name, types, and base stats, writing them in a compact text format:
///
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
fn fetch_pokemon_stats(name: &str, pokedex_num: usize) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("{}{}", POKEMONDB_POKEDEX_URL, name);
    let html = http_get_string(&url)?;

    let display_name = extract_display_name(&html).unwrap_or_else(|| capitalize(name));
    let types = extract_types(&html);
    let stats = extract_base_stats(&html);

    if types.is_empty() {
        return Err("Could not parse types — page structure may have changed".into());
    }
    if stats.len() < 6 {
        return Err(format!("Expected 6 base stats, got {}", stats.len()).into());
    }

    Ok(format!(
        "#{:03}\n{}\n{}\nHP:{}\nAtk:{}\nDef:{}\nSpA:{}\nSpD:{}\nSpe:{}\n",
        pokedex_num,
        display_name,
        types.join("/"),
        stats[0], stats[1], stats[2], stats[3], stats[4], stats[5]
    ))
}

/// Extracts the Pokemon's proper display name from the page title.
///
/// The `<title>` is like:
///   `Charizard Pokédex: stats, moves, evolution & locations | Pokémon Database`
///
/// We take everything before the first " Pokédex" to get "Charizard", which
/// preserves names like "Mr. Mime", "Mime Jr.", "Farfetch'd", "Nidoran♂", etc.
fn extract_display_name(html: &str) -> Option<String> {
    let title_open = "<title>";
    let pos = html.find(title_open)?;
    let after = &html[pos + title_open.len()..];
    let title_end = after.find("</title>").unwrap_or(after.len());
    let title = &after[..title_end];
    // "é" in "Pokédex" is 2 bytes; find() on a &str works with byte offsets and
    // always returns positions at char boundaries, so this slice is safe.
    let sep_pos = title.find(" Pokédex").or_else(|| title.find(" Pok"))?;
    Some(title[..sep_pos].trim().to_string())
}

/// Extracts up to two type names from the "Type" row of the first vitals table.
///
/// Pokemondb HTML looks like:
/// ```html
/// <th>Type</th>
/// <td>
///   <a class="type-icon type-fire" href="/type/fire">Fire</a>
///   <a class="type-icon type-flying" href="/type/flying">Flying</a>
/// </td>
/// ```
fn extract_types(html: &str) -> Vec<String> {
    let type_header = "Type</th>";
    let type_pos = match html.find(type_header) {
        Some(p) => p,
        None => return vec![],
    };

    // Slice to the end of this table row only. Using "</tr>" (pure ASCII) as
    // boundary avoids any UTF-8 char-boundary panics that a fixed byte offset
    // like `type_pos + 600` could cause when the HTML contains multi-byte chars.
    let row_end = html[type_pos..]
        .find("</tr>")
        .map(|p| type_pos + p)
        .unwrap_or(html.len());
    let section = &html[type_pos..row_end];

    let prefix = "class=\"type-icon type-";
    let mut types = Vec::new();
    let mut pos = 0;

    while let Some(rel) = section[pos..].find(prefix) {
        let start = pos + rel + prefix.len();
        if let Some(end_q) = section[start..].find('"') {
            let type_slug = &section[start..start + end_q];
            if !type_slug.is_empty() {
                types.push(capitalize(type_slug));
            }
            pos = start + end_q;
        } else {
            break;
        }
        if types.len() >= 2 {
            break;
        }
    }

    types
}

/// Extracts the six base stats (HP, Attack, Defense, Sp. Atk, Sp. Def, Speed)
/// from the first stats table on the page (base form).
///
/// Each row looks like:
/// ```html
/// <th>HP</th>
/// <td class="cell-num">45</td>
/// ```
fn extract_base_stats(html: &str) -> Vec<u32> {
    let stat_headers = [
        "<th>HP</th>",
        "<th>Attack</th>",
        "<th>Defense</th>",
        "<th>Sp. Atk</th>",
        "<th>Sp. Def</th>",
        "<th>Speed</th>",
    ];
    let cell_marker = "<td class=\"cell-num\">";
    let mut result = Vec::new();
    let mut search_pos = 0;

    for header in &stat_headers {
        if let Some(rel) = html[search_pos..].find(header) {
            let abs_pos = search_pos + rel;
            let after = &html[abs_pos + header.len()..];
            if let Some(cell_rel) = after.find(cell_marker) {
                let val_start = cell_rel + cell_marker.len();
                let val_str = &after[val_start..];
                if let Some(tag_end) = val_str.find('<') {
                    if let Ok(val) = val_str[..tag_end].trim().parse::<u32>() {
                        result.push(val);
                        search_pos = abs_pos + header.len();
                        continue;
                    }
                }
            }
        }
        // Parsing failed for this stat — push 0 as a placeholder
        result.push(0);
    }

    result
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn process_pokemon(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let img = download_sprite(name)?;
    let img = img.to_rgba8();
    let img = crop_to_content(img);
    let img = resize_if_too_large(img);
    let img = add_padding(img, PADDING);
    Ok(image_to_ansi_art(img))
}

fn download_sprite(name: &str) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    let url = format!("{}{}.png", POKEMONDB_SPRITE_URL, name);
    let bytes = http_get_bytes(&url)?;
    Ok(image::load_from_memory(&bytes)?)
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

fn http_get_bytes(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;
    let mut bytes: Vec<u8> = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn http_get_string(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;
    Ok(response.into_string()?)
}

// ---------------------------------------------------------------------------
// Image processing
// ---------------------------------------------------------------------------

/// Crops the image to the bounding box of fully-opaque pixels,
/// removing any transparent margins from the original PNG.
fn crop_to_content(img: RgbaImage) -> RgbaImage {
    let (width, height) = (img.width(), img.height());
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;

    for y in 0..height {
        for x in 0..width {
            if img.get_pixel(x, y)[3] > 0 {
                if x < min_x { min_x = x; }
                if y < min_y { min_y = y; }
                if x > max_x { max_x = x; }
                if y > max_y { max_y = y; }
                found = true;
            }
        }
    }

    if !found {
        return img;
    }

    imageops::crop_imm(&img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}

/// Scales the image down by 1.5x if its height exceeds HEIGHT_THRESHOLD,
/// matching the Python make_art.py behaviour. Uses nearest-neighbour filtering
/// (anti_aliasing=False equivalent).
fn resize_if_too_large(img: RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    if h > HEIGHT_THRESHOLD {
        let new_w = (w as f32 / 1.5).round() as u32;
        let new_h = (h as f32 / 1.5).round() as u32;
        imageops::resize(&img, new_w, new_h, imageops::FilterType::Nearest)
    } else {
        img
    }
}

/// Adds `pad` pixels of fully-transparent space on every side of the image.
/// Gives every sprite a uniform margin without changing its natural size.
fn add_padding(img: RgbaImage, pad: u32) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut padded = image::ImageBuffer::from_pixel(
        w + pad * 2,
        h + pad * 2,
        image::Rgba([0u8, 0, 0, 0]),
    );
    imageops::overlay(&mut padded, &img, pad as i64, pad as i64);
    padded
}

/// Converts an RGBA image to a string of ANSI 24-bit colour escape sequences
/// using Unicode half-block characters (▀/▄) to pack 2 vertical pixels per
/// terminal cell.
///
/// - Both opaque   → ▀  fg=top colour, bg=bottom colour
/// - Top only      → ▀  fg=top colour, bg=reset
/// - Bottom only   → ▄  fg=bottom colour, bg=reset
/// - Both transp.  → space (bg reset so terminal background shows through)
/// - Consecutive same-state pixels share escape sequences
/// - Each row ends with \x1b[0m to reset colour state
fn image_to_ansi_art(img: RgbaImage) -> String {
    let (width, height) = (img.width(), img.height());
    let rows = (height + 1) / 2;
    let mut art = String::with_capacity((width * rows as u32 * 30) as usize);

    for row in 0..rows {
        art.push('\n');
        let y_top = row * 2;
        let y_bot = row * 2 + 1;

        // None = currently reset, Some(rgb) = currently set to this colour
        let mut cur_fg: Option<(u8, u8, u8)> = None;
        let mut cur_bg: Option<(u8, u8, u8)> = None;

        for x in 0..width {
            let [tr, tg, tb, ta] = img.get_pixel(x, y_top).0;
            let [br, bg, bb, ba] = if y_bot < height {
                img.get_pixel(x, y_bot).0
            } else {
                [0, 0, 0, 0]
            };

            let top = if ta > 0 { Some((tr, tg, tb)) } else { None };
            let bot = if ba > 0 { Some((br, bg, bb)) } else { None };

            match (top, bot) {
                (None, None) => {
                    if cur_bg.is_some() {
                        art.push_str("\x1b[49m");
                        cur_bg = None;
                    }
                    art.push(' ');
                }
                (Some(tc), None) => {
                    if cur_fg != Some(tc) {
                        art.push_str(&format!("\x1b[38;2;{};{};{}m", tc.0, tc.1, tc.2));
                        cur_fg = Some(tc);
                    }
                    if cur_bg.is_some() {
                        art.push_str("\x1b[49m");
                        cur_bg = None;
                    }
                    art.push('▀');
                }
                (None, Some(bc)) => {
                    if cur_fg != Some(bc) {
                        art.push_str(&format!("\x1b[38;2;{};{};{}m", bc.0, bc.1, bc.2));
                        cur_fg = Some(bc);
                    }
                    if cur_bg.is_some() {
                        art.push_str("\x1b[49m");
                        cur_bg = None;
                    }
                    art.push('▄');
                }
                (Some(tc), Some(bc)) => {
                    if cur_fg != Some(tc) {
                        art.push_str(&format!("\x1b[38;2;{};{};{}m", tc.0, tc.1, tc.2));
                        cur_fg = Some(tc);
                    }
                    if cur_bg != Some(bc) {
                        art.push_str(&format!("\x1b[48;2;{};{};{}m", bc.0, bc.1, bc.2));
                        cur_bg = Some(bc);
                    }
                    art.push('▀');
                }
            }
        }

        art.push_str("\x1b[0m");
    }

    art.push('\n');
    art
}
