<div align="center">

<pre>
      o
   o       ><(((º>
      ><>          ::<>
</pre>

  <h1>fishtanks</h1>

  <p><strong>An aquarium for your terminal.</strong></p>

  <p>Fishes swim, eat, grow and do zoomies while you work. Look over now and then.</p>

  <p>
    <a href="#install"><strong>Install</strong></a>
    &nbsp;·&nbsp;
    <a href="#playing">Playing</a>
    &nbsp;·&nbsp;
    <a href="https://github.com/daniel-retamal/fishtanks/wiki">Wiki</a>
    &nbsp;·&nbsp;
    <a href="CONTRIBUTING.md">Contributing</a>
  </p>

  <p>
    <a href="https://github.com/daniel-retamal/fishtanks/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/daniel-retamal/fishtanks?label=release&color=1f6feb" /></a>
    <a href="#install"><img alt="macOS, Linux and Windows" src="https://img.shields.io/badge/macOS%20%C2%B7%20Linux%20%C2%B7%20Windows-1f6feb" /></a>
    <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-1f6feb" /></a>
  </p>

  <img src="https://raw.githubusercontent.com/daniel-retamal/fishtanks/main/.github/assets/fishtank.gif" alt="The Fishtank: three goldfishes, a nishiki, an aka and a kuro among the kelp" width="900" />

</div>

---

> **Coffee.** Work-communion-enabling percolated Breverage. Allows terminal-humanii connection. Gives fishes something to believe in. _Faster reeling._

`fishtanks` is an aquarium that lives in a terminal window. Leave it open in a split pane beside your editor or your shell, and it keeps itself: the fishes drift, blow bubbles, chase food and burst into zoomies whether you touch it or not.

Under the calm water there is a game. Go fishing, sell what you catch, buy new fishes and new tanks, and each tank turns out to have its own rules. Some fishes mutate. Some die, and where they go depends on how they lived. Some can be wired together into computers. It saves itself as you play, and it is there where you left it the next time you open it.

## Install

One line, and nothing else to install first.

**macOS and Linux**

```sh
curl -fsSL https://retam.al/fishtanks/install.sh | sh
```

**Windows** (PowerShell)

```powershell
irm https://retam.al/fishtanks/install.ps1 | iex
```

**Homebrew** (macOS and Linux)

```sh
brew install daniel-retamal/tap/fishtanks
```

Then open a new terminal and type:

```sh
fishtanks
```

<details>
<summary>Other ways to get it</summary>

- **Download it yourself.** Every [release](https://github.com/daniel-retamal/fishtanks/releases/latest) has a build for each system: unpack it and run `fishtanks` from wherever you put it.
- **With Rust.** `cargo install fishtanks` builds it from source.

</details>

### Updating

`fishtanks` tells you when a newer version is out. Then type:

```sh
fishtanks update
```

It fetches the new version and swaps it in (with Homebrew it gives you the `brew` line to run instead). Your game carries over.

### What you need

A terminal with colour and Unicode, which is every terminal on macOS and Linux, and [Windows Terminal](https://aka.ms/terminal) on Windows (the default on Windows 11). Any size works down to 40 by 14 cells, though the fishes like room.

## Playing

The bar at the bottom takes commands. Type `/`, press `Tab` to complete, `↑` and `↓` to walk back through what you typed. In any window, the arrows move, `Enter` chooses and `Esc` closes.

| Command                   | What it does                                                           |
| ------------------------- | ---------------------------------------------------------------------- |
| `/feed`                   | Drop food. A fed fish grows, and a heavier fish is worth more          |
| `/fish`                   | Go fishing. Hook it, reel it in, and see what the tank gives you       |
| `/shop`                   | Buy fishes, food, coffee, bait and new tanks, and sell what you caught |
| `/inventory`              | What you are carrying. `/consume <item>` uses one                      |
| `/fishtanks`              | Every tank you own. `/switch "<tank>"` goes there                      |
| `/move "<fish>" "<tank>"` | Carry a fish to another tank                                           |
| `/index`                  | Every fish in this tank. `/show "<name>"` looks at one closely         |
| `/names`                  | Show or hide the fishes' names                                         |
| `/ledger`                 | Where your money came from, and where it went                          |
| `/casino`                 | Six tables run by Tollomind. Bet cash, or bet a fish's life            |
| `/zen`                    | Just the water. Any key brings the rest back                           |
| `/exit`                   | Leave. The game is saved, and it will be there when you come back      |

Anything you type that is not a command is said out loud. Most of the time nobody is listening.

### Fishing

Fishing is where catches come from: fish, cash, food, and stranger things.

<img src="https://raw.githubusercontent.com/daniel-retamal/fishtanks/main/.github/assets/fishing.gif" alt="A whole cast: the wait, the bite, the fight and the catch, with the keys held shown underneath" width="520" />

1. Type `/fish`, then **wait**. Touching `↓` before the bite scares the fish away.
2. When the tip of the rod dips, **press `↓` once**. You have half a second.
3. **Steer** with `←` and `→`: push the fish back toward the middle, before it reaches the red.
4. **Reel** by holding `↓`, but only while the block on the track is **green**. Let go when it turns red. Holding `↓` and nothing else never lands a fish.
5. When the catch bar on the right is full, the fish is yours. Name it and press `Enter`.

The [wiki](https://github.com/daniel-retamal/fishtanks/wiki/Fishing) has the long version.

### The casino

Where fished money goes to get stupid. `/casino` opens Tollomind's lobby: Blackjack, Spins with its Pearl Dive, a Pufferfish that puffs until it pops, Bubble Up, the Derby and the Claw, where you win Toyfish to dress up in `/toybox`. Bet cash, or put a fish on the line: it plays for more than it is worth, and it dies if it loses. Every win offers double or nothing, and nothing has a ceiling. The house always wins, except at the Claw, if your hands are steady.

<img src="https://raw.githubusercontent.com/daniel-retamal/fishtanks/main/.github/assets/casino.gif" alt="Bubble Up on the Stupid board: a bubble climbs to the ×1,000 shell and an Absolutely Stupid Win card counts up" width="720" />

Every table, its odds and its keys are on the [wiki](https://github.com/daniel-retamal/fishtanks/wiki/Casino).

### Getting started

You begin with a Fishtank, a few fishes, a little cash and a bag of food. Fishing is where money comes from, so `/fish` early and often. Sell what you do not want to keep, feed the ones you do, and save up for a second tank. Every tank is a new set of rules, not just more room. The [wiki](https://github.com/daniel-retamal/fishtanks/wiki) explains every fish, tank and mechanic.

### What is down there

- **Eighty-odd species.** Commons you can buy anywhere, rares that swim a little strangely, and legendaries that live in exactly one kind of tank and are never for sale.
- **Tanks with their own rules.** A graveyard that remembers your dead. A desert whose nights bring visitors. A candy tank. Some tanks are never sold at all: the sea gives you something, and you grow them from it.
- **Mutation.** Radiation, milk and time change fishes. Sometimes two fishes become one.
- **Life after death.** A fish that dies goes somewhere. Where depends on the mark it carried.
- **Fishes as logic.** With a computer, a fish becomes a gate you can wire. Latches, adders, and a calculator made entirely of fish have all been built.
- **Secrets.** `/cheat` takes a code. This README will not tell you any.

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/daniel-retamal/fishtanks/main/.github/assets/haunted.png" alt="The Hauntedtank, with a grave for each fish that died" /></td>
    <td><img src="https://raw.githubusercontent.com/daniel-retamal/fishtanks/main/.github/assets/desert.png" alt="The Desertank at night, a UFO lifting a merluza" /></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/daniel-retamal/fishtanks/main/.github/assets/matrix.png" alt="A Matrixtank where a cow says 6+4 and a botfish answers 10" /></td>
    <td><img src="https://raw.githubusercontent.com/daniel-retamal/fishtanks/main/.github/assets/candy.png" alt="The Candytank" /></td>
  </tr>
</table>

### Your save

The game writes itself down a second after you stop typing, every half minute, and whenever you close it, even by closing the window. It lives in:

| System  | Folder                                        |
| ------------------------- | ---------------------------------------------------------------------- |
| Linux   | `~/.local/share/fishtank/`                    |
| macOS   | `~/Library/Application Support/fishtank/`     |
| Windows | `%APPDATA%\fishtank\`                         |

`/export "my-tank.ron"` writes your whole game to a file you can keep or carry to another computer, and `/import "my-tank.ron"` brings it back. The game you replace is set aside, not lost.

### Uninstalling

`brew uninstall fishtanks` if you used Homebrew. Otherwise delete the `fishtanks` program: `which fishtanks` on macOS and Linux, or `where.exe fishtanks` on Windows, says where it is. Delete the save folder above to let the fishes go too.

## Contributing

New fishes and new tanks are the contributions this project loves most, and bug reports are always welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Acknowledgements

A great deal of the art in these tanks started life on [asciiart.website](https://asciiart.website/) and [asciiart.eu](https://www.asciiart.eu/), and it would not exist without the artists who drew it there, above all **Joan Stark** (Spunk) and **Laura Brown**.

[ASCII-Aquarium](https://github.com/POWER-PILL/ASCII-Aquarium) and [asciiquarium](https://github.com/cmatsuoka/asciiquarium) came first, and this project would not have started without them.

The Turbofish swims as `::<>`, Rust's turbofish, a name given to that syntax by [Anna Harren](https://turbo.fish/about).

Built with [ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm).

## License

The code is [MIT](LICENSE). The ASCII art drawn by the artists above remains theirs.
