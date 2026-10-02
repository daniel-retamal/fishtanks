# Changelog

## 1.1.0 - 2026-10-02

- **Hold on to your junk.** Something happens once you have a hundred.
- **More than fish on the line.** Cash, food, junk, coffee and bait come up about twice as often: a third of what you pull up is no longer a fish.
- **Every fish fights at its own pace.** Most fish fight at a normal pace and legendary ones faster; now and then a common fish fights like a legend. The control turns green while it sits in the safe zone and red outside it, and holding ↓ alone never lands a fish.
- **The Heaventank is found, not given.** It no longer appears on your first sale: fish up a Golden Pearl and grow it. The souls of fish sold before then wait for it.
- **Held keys work in more terminals**, Herdr included: fishing and `/console` now believe only the key presses and releases a terminal really sends.

## 1.0.4 - 2026-09-25

- **Fishing on macOS and Linux plays like Windows.** Hold ↓ to reel and ←→ to steer, everywhere; the ↑ key that stopped the reel is gone, and hooking a fish no longer starts reeling on its own. In a terminal that reports key releases (Ghostty, kitty, WezTerm, Alacritty, iTerm2 with the kitty keyboard protocol) it is exactly the Windows game, reeling while you steer included. macOS's built-in Terminal tells programs only about the last key held, so there reel between steers.

## 1.0.3 - 2026-09-25

- **Closing the window really closes the game on macOS and Linux.** Before, a game whose terminal window was closed could keep running unseen, using a whole processor core, and the next `fishtanks` said it was "already swimming in another window". If that happens to you on 1.0.2 or older, run `pkill -9 fishtanks` once, then update.
- **`fishtanks update` gives Homebrew players the whole command**: `brew update && brew upgrade fishtanks`. Homebrew refreshes its list of versions only about once a day, so `brew upgrade` on its own can miss a release from the same day.

## 1.0.2 - 2026-09-25

- **Fishing works on macOS and Linux.** Those terminals never tell a program that a key was let go, so the reel kept reeling after you lifted ↓ and the bar emptied in a blink. Now ↓ starts the reel and ↑ stops it there (the bottom of the fishing window says so), and steering follows your keys as they repeat. On Windows nothing changes: hold ↓ to reel.
- Console mode (`/console`) no longer sticks a key down on macOS and Linux either.

## 1.0.1 - 2026-09-25

- **`fishtanks update` works on Windows.** 1.0.0 updated itself by starting a PowerShell script, which Windows Defender blocks as a suspected trojan. It now downloads the new version and swaps itself in, with no script at all. If you have 1.0.0 on Windows, update once by running the install line again; from 1.0.1 on, `fishtanks update` does it.

## 1.0.0 - 2026-09-25

The first public release. An aquarium for your terminal, and a game under the water.

- **The aquarium.** ASCII fishes swim, eat, grow, blow bubbles and burst into zoomies on their own. `/zen` hides everything but the water.
- **Fishing** with `/fish`, a small reel-in game, and a **shop** to buy fishes, food, coffee, bait and tanks, and to sell what you catch. `/ledger` shows where the money went.
- **Tanks with their own rules**: coral, candy, a haunted graveyard, a desert with visitors at night, and tanks that are never sold, grown from what the sea gives you.
- **Twenty-odd species**, from commons to legendaries that live in only one kind of tank.
- **Mutation, fusion, life after death**, and fishes you can wire into logic, up to a calculator made of fish.
- **Always saved.** The game writes itself down as you play and when you close it. `/export` and `/import` carry it between computers.
- **One-line installs** on macOS, Linux and Windows, and a notice when a newer version is out: `fishtanks update`.
