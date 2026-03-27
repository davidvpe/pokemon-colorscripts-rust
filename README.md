# Pokemon Colorscripts

Print colored unicode sprites of Pokemon directly in your terminal. Inspired by
[DT's colorscripts](https://gitlab.com/dwt1/shell-color-scripts).

Sprites sourced from [pokemondb.net](https://pokemondb.net/sprites).

## Demo

![demo of program in action](./demo_images/colorscript-demo.gif)

![demo of random pokemon on terminal spawn](./demo_images/poke_demo.gif)

## Requirements

- A terminal with true color support (most modern terminals)

## Installation

**macOS / Linux**
```sh
curl -fsSL https://raw.githubusercontent.com/davidvpe/pokemon-colorscripts-rust/main/install.sh | sudo bash
```

**Windows** (PowerShell)
```powershell
irm https://raw.githubusercontent.com/davidvpe/pokemon-colorscripts-rust/main/install.ps1 | iex
```

Downloads the pre-built binary for your platform from the latest release.
No build tools required — all sprites are embedded in the binary.

## Usage

```
pokemon-colorscripts [OPTION] [POKEMON NAME]

  -h, --help            Print this help
  -l, --list            List all available pokemon
  -r, --random [N]      Show a random pokemon (optional: generation 1–9)
  -n, --name <name>     Show a specific pokemon by name
      --no-title        Do not print the pokemon name
  -i, --info            Show a side panel with types and base stats
  -p, --pokedex         Show a full Pokédex frame (sprite + stats)
```

### Modes

> All output is rendered in full 24-bit color in a true-color terminal.
> The examples below show the layout without color.

---

**Basic** — `pokemon-colorscripts --name pikachu`

```
Pikachu

              ████
            ██████                  ████
            ██████                ████████
          ████████        ██████████████████
          ████████    ██████████████████████
        ████████    ██████████████████████
      ██████████████████████████████████
    ██████████████████████████████████
    ████████████████████████████████
  ████████████████████████  ██████████
  ██████████████████████████  ████████
    ████████████████████████████████
  ████████████████████████████████
    ████████████████████████████████
          ████████████████████████
          ██████████████████████
            ████████████████████
              ████████████████
                    ████████
                        ██
```

---

**Info panel** — `pokemon-colorscripts --name pikachu --info`

Stats panel appears to the right of the sprite, vertically centered.

```
Pikachu
                                                 ╔══════════════════════════╗
              ████                               ║ #025 · Pikachu           ║
            ██████                  ████         ║ Electric                 ║
            ██████                ████████       ╠══════════════════════════╣
          ████████        ██████████████████     ║ HP   35  ██░░░░░░░░░░░░░ ║
          ████████    ██████████████████████     ║ Atk  55  ███░░░░░░░░░░░░ ║
        ████████    ██████████████████████       ║ Def  40  ██░░░░░░░░░░░░░ ║
      ██████████████████████████████████         ║ SpA  50  ███░░░░░░░░░░░░ ║
    ██████████████████████████████████           ║ SpD  50  ███░░░░░░░░░░░░ ║
    ████████████████████████████████             ║ Spe  90  █████░░░░░░░░░░ ║
  ████████████████████████  ██████████           ╠══════════════════════════╣
  ██████████████████████████  ████████           ║ BST 320                  ║
    ████████████████████████████████             ╚══════════════════════════╝
  ████████████████████████████████
    ████████████████████████████████
          ████████████████████████
          ██████████████████████
            ████████████████████
              ████████████████
                    ████████
                        ██
```

---

**Pokédex view** — `pokemon-colorscripts --name pikachu --pokedex`

Full frame that expands to fill the terminal width. The right panel adapts
its stat bar lengths to the available space.

```
╔═════════════════════════════════════════════════════════════════════════════════╗
║ [ POKÉDEX ]                                                      #025 · Pikachu ║
╠══════════════════════════════════════════════════╦══════════════════════════════╣
║                                                  ║  Pikachu                     ║
║                ████                              ║  #025               Electric ║
║              ██████                  ████        ╠══════════════════════════════╣
║              ██████                ████████      ║  HP    35  ██░░░░░░░░░░░░░░░ ║
║            ████████        ██████████████████    ║  Atk   55  ████░░░░░░░░░░░░░ ║
║            ████████    ██████████████████████    ║  Def   40  ███░░░░░░░░░░░░░░ ║
║          ████████    ██████████████████████      ║  SpA   50  ███░░░░░░░░░░░░░░ ║
║        ██████████████████████████████████        ║  SpD   50  ███░░░░░░░░░░░░░░ ║
║      ██████████████████████████████████          ║  Spe   90  ██████░░░░░░░░░░░ ║
║      ████████████████████████████████            ╠══════════════════════════════╣
║    ████████████████████████  ██████████          ║  BST  320                    ║
║    ██████████████████████████  ████████          ║                              ║
║      ████████████████████████████████            ║                              ║
║    ████████████████████████████████              ║                              ║
║      ████████████████████████████████            ║                              ║
║            ████████████████████████              ║                              ║
║            ██████████████████████                ║                              ║
║              ████████████████████                ║                              ║
║                ████████████████                  ║                              ║
║                      ████████                    ║                              ║
║                          ██                      ║                              ║
║                                                  ║                              ║
╚══════════════════════════════════════════════════╩══════════════════════════════╝
```

---

### Examples

```sh
# Specific pokemon
pokemon-colorscripts --name pikachu
pokemon-colorscripts --name charizard --info
pokemon-colorscripts --name mewtwo --pokedex

# Random
pokemon-colorscripts --random
pokemon-colorscripts --random 1          # Gen 1 only
pokemon-colorscripts --random --pokedex

# Without the name printed
pokemon-colorscripts --name snorlax --no-title
```

### Pokemon name spelling

Names follow the in-game spelling. A few exceptions:

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

**bash** — add to `~/.bashrc`:
```sh
pokemon-colorscripts --random
```

**zsh** — add to `~/.zshrc`:
```sh
pokemon-colorscripts --random
```

**fish** — add to `~/.config/fish/config.fish`:
```fish
pokemon-colorscripts --random
```

**PowerShell** — add to your profile (`$PROFILE`):
```powershell
pokemon-colorscripts --random
```
> Run `New-Item -Force $PROFILE` first if the file doesn't exist yet.

## How it works

All ~1000 sprite files are embedded directly into the binary at compile time — no
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
