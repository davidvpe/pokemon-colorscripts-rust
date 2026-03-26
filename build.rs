use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("pokemon_data.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    // Use nameslist.txt for Pokedex order so generation filtering works by index.
    let names_content = fs::read_to_string("nameslist.txt")
        .expect("nameslist.txt not found");
    let names: Vec<String> = names_content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // Only embed Pokemon that have a colorscript file — skip missing ones silently.
    let available: Vec<(String, std::path::PathBuf)> = names
        .iter()
        .filter_map(|name| {
            let path = Path::new("colorscripts").join(format!("{}.txt", name));
            path.canonicalize().ok().map(|p| (name.clone(), p))
        })
        .collect();

    // get_pokemon_art — match over available pokemon
    writeln!(f, "fn get_pokemon_art(name: &str) -> Option<&'static str> {{").unwrap();
    writeln!(f, "    match name {{").unwrap();
    for (name, path) in &available {
        writeln!(f, "        {:?} => Some(include_str!({:?})),", name, path).unwrap();
    }
    writeln!(f, "        _ => None,").unwrap();
    writeln!(f, "    }}").unwrap();
    writeln!(f, "}}").unwrap();

    // ALL_POKEMON in Pokedex order — only those with colorscripts
    writeln!(f, "static ALL_POKEMON: &[&str] = &[").unwrap();
    for (name, _) in &available {
        writeln!(f, "    {:?},", name).unwrap();
    }
    writeln!(f, "];").unwrap();

    println!("cargo:rerun-if-changed=nameslist.txt");
    println!("cargo:rerun-if-changed=colorscripts");
    println!("cargo:rerun-if-changed=build.rs");
}
