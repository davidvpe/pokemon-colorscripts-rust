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
/// 1 pixel = 1 terminal line vertically, 2 chars horizontally (pixels are doubled).
const PADDING: u32 = 1;
/// Sprites taller than this (in pixels) are scaled down by 1.5x, matching the
/// Python make_art.py behaviour.
const HEIGHT_THRESHOLD: u32 = 32;
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
            if img.get_pixel(x, y)[3] == 255 {
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

/// Converts an RGBA image to a string of ANSI 24-bit colour escape sequences.
///
/// - Opaque pixels → █ with foreground colour escape
/// - Transparent pixels → space
/// - Each pixel is doubled horizontally (terminal chars are taller than wide)
/// - Consecutive same-colour pixels share one escape sequence
/// - Ends with reset escape \x1b[0m
fn image_to_ansi_art(img: RgbaImage) -> String {
    let (width, height) = (img.width(), img.height());
    let mut art = String::with_capacity((width * height * 25) as usize);

    for y in 0..height {
        art.push('\n');
        let mut old_color: Option<(u8, u8, u8)> = None;

        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let [r, g, b, a] = pixel.0;
            let ch = if a == 255 { '█' } else { ' ' };
            let new_color = (r, g, b);

            if Some(new_color) != old_color {
                art.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                old_color = Some(new_color);
            }

            art.push(ch);
            art.push(ch);
        }
    }

    art.push_str("\x1b[0m\n");
    art
}
