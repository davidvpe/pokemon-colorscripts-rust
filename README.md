# Pokemon Colorscripts

Print colored unicode sprites of Pokemon directly in your terminal. Inspired by
[DT's colorscripts](https://gitlab.com/dwt1/shell-color-scripts).

Sprites sourced from [pokemondb.net](https://pokemondb.net/sprites).

## Demo

![demo of program in action](./demo_images/colorscript-demo.gif)

![demo of random pokemon on terminal spawn](./demo_images/poke_demo.gif)

## Requirements

- A terminal with true color support (most modern terminals)
- Rust toolchain (for building from source)

## Installation

```sh
git clone https://github.com/your-username/pokemon-colorscripts-rust.git
cd pokemon-colorscripts-rust
sudo ./install.sh
```

The install script builds the release binary and copies it to `/usr/local/bin/pokemon-colorscripts`.
All sprites are embedded in the binary — no extra files needed.

## Usage

```
pokemon-colorscripts [OPTION] [POKEMON NAME]

  -h, --help      Print this help
  -l, --list      List all available pokemon
  -r, --random    Show a random pokemon
  -n, --name      Show a specific pokemon by name
```

```sh
pokemon-colorscripts --name pikachu
pokemon-colorscripts --random
pokemon-colorscripts --list
```

Pokemon names are generally spelled as in the games. A few exceptions:

```
farfetch'd  → farfetchd
mr. mime    → mr-mime
nidoran ♀   → nidoran-f
nidoran ♂   → nidoran-m
type: null  → type-null
flabébé     → flabebe
```

If unsure, use `--list` and grep:

```sh
pokemon-colorscripts --list | grep mime
```

### Run on terminal startup

Add to your `~/.zshrc` or `~/.bashrc`:

```sh
pokemon-colorscripts --random
```

## How it works

All ~900 sprite files are embedded directly into the binary at compile time — no
external files are needed after installation.

Sprites are downloaded from pokemondb.net (sword-shield icons) and converted to
ANSI 24-bit color escape sequences using a bundled generator tool. Each pixel
becomes a `█` character colored with its original RGB value.

### Regenerating sprites

The generator is a separate binary included in this repo:

```sh
# Update the pokemon list from pokemondb.net
cargo run --bin generator --features generator -- update-names

# Generate all colorscripts (only missing ones)
cargo run --bin generator --features generator

# Force regenerate all existing files
cargo run --bin generator --features generator -- --force
```

Sprites are also regenerated automatically every week via GitHub Actions.

## License

MIT
