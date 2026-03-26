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
    println!();
    println!("Examples:");
    println!("  pokemon-colorscripts --name pikachu");
    println!("  pokemon-colorscripts --random");
    println!("  pokemon-colorscripts --random 1");
    println!("  pokemon-colorscripts --random 1 --no-title");
}

fn show_random(gen: Option<usize>, no_title: bool) {
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

    if !no_title {
        println!("{}", name);
    }
    if let Some(art) = get_pokemon_art(name) {
        print!("{}", art);
    }
}

fn show_by_name(name: &str, no_title: bool) {
    if !no_title {
        println!("{}", name);
    }
    match get_pokemon_art(name) {
        Some(art) => print!("{}", art),
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

    // Strip --no-title from positional args; it can appear anywhere.
    let no_title = args.iter().any(|a| a == "--no-title");
    let pos: Vec<&str> = args.iter()
        .filter(|a| a.as_str() != "--no-title")
        .map(String::as_str)
        .collect();

    match pos.as_slice() {
        [] => print_help(),

        ["-h" | "--help" | "help"] => print_help(),

        ["-l" | "--list" | "list"] => list_pokemon(),

        ["-r" | "--random" | "random"] => show_random(None, no_title),

        ["-r" | "--random" | "random", gen] => {
            match gen.parse::<usize>() {
                Ok(g) if (1..=9).contains(&g) => show_random(Some(g), no_title),
                _ => {
                    eprintln!("Invalid generation '{}'. Use a number between 1 and 9.", gen);
                    std::process::exit(1);
                }
            }
        }

        ["-n" | "--name" | "name", name] => show_by_name(name, no_title),

        _ => {
            eprintln!("Input error. Run with --help for usage.");
            std::process::exit(1);
        }
    }
}
