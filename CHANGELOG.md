# Changelog

## 1.5.0 - 2026-10-08

- **The Claw replaces the Mystery Net.** One price a go, $50 for everyone. Steer, let the claw settle, drop it, and watch your prize ride to the chute, if it holds on. The middle of a prize grips best, heavy prizes slip more, and `P` shows how heavy everything in the glass is. Win something and a new prize drops in; come back to the casino and the glass is different. A steady hand can come out ahead.
- **Toyfish.** Plush toys you win at the Claw: twenty-one colors, five materials from plastic to holographic, and sizes from a keychain to a giant. They float in your tank, never eat, and sell for at least what a go costs.
- **Dress them up.** `/toybox` lists your toys beside a display of the one you picked. Snap on parts won in capsules: wings, wheels, a rotor, a rocket, a crown and more. Parts change how a toy moves, so watch it in your tank.
- **Signatures.** Some toys come fully dressed, with names, in lines of four: Mecha, Kaiju, Racers, Deep Sea, Space and Royals. Your shelf (`Tab` in the toybox) remembers everything you ever won, and a full row does not go unnoticed.

## 1.4.0 - 2026-10-07

- **The casino.** `/casino` opens Tollomind's lobby, six tables where your fished money goes: Blackjack, Spins with its Pearl Dive bonus, the Pufferfish (cash out before it pops), Bubble Up, the Derby and the Mystery Net. Every table prints what it pays back, and the house always wins.
- **Bet a fish.** Put a fish on the line and it plays for a quarter more than its worth, at every table but the Mystery Net. Win or tie and it swims home with its winnings; lose and Tollomind eats it.
- **Double or nothing**, after any win and as often as you dare: the money doubles, and so does every fish on the line.
- **Net a fish** in the Mystery Net, Legendaries included.
- **No ceiling on money.** Fortunes past a sextillion are written like 1.2e24.

## 1.3.0 - 2026-10-06

- **Sort and filter the index.** In `/index`, `←`/`→` pick a column, `S` sorts by it (up, down, off) and `F` filters it as you type; `C` clears it all. The command bar asks the same questions: `/index species:koi sort:-worth`, `/index weight:>1kg`, `/index "Coral"`. The table shows the command it is answering, so the words come by using it.
- **Fifteen new unfish.** Pale, idle, uncanny things crawl out of the Void: the Leech, Bones, the Ouroboros, the Graeae, the Still, Verso, the Molt and more, each acting on your fishes in its own way. Every one sells and dies like a fish.
- **Coffee is drunk by your casts too.** A cup still lasts a minute, and every fish you land drinks a tenth of it: a cup is about ten casts (a bait is five), and ten seconds of coffee are no longer an afternoon of fishing.
- **The shop keeps your place.** After you sell or buy something the cursor stays where you were instead of jumping to the top, and it no longer blinks away while you hold an arrow.
- **Food by the thirty.** Past 30 pellets, the shop's food counter steps 30 at a time.
- **Hold ↓ and steer in more terminals.** When a terminal hides which keys are held, the game asks the keyboard itself: on Windows straight away, on macOS once you allow Input Monitoring (it asks once, after a cast).
- The Shoalfish is hooked by its leading fish on the catch card, not in the water in front of it.

## 1.2.1 - 2026-10-04

- **Twins swim apart.** When a double splits in two, the new fish heads off the other way instead of shadowing its parent.
- **The octopus has its own tentacles back**: three, swaying together, in its own colour.
- **`cyclops`**: a new mutation that leaves a fish or a cow with a single eye. `revert` gives the others back.
- **A swallowed fish loses its tail.** A fish that engulfs another is drawn head, body, body, head, with no tail stuck inside it.
- **The wiki grew a dex.** Every fish, unfish, cow and mutation now has its own entry: what it looks like, an animated picture of what makes it special, and its numbers. Fishing is shown step by step.

## 1.2.0 - 2026-10-03

- **A wiki.** Every fish, tank, item and mechanic, how to fish, how to wire botfish, and a step-by-step calculator: [github.com/daniel-retamal/fishtanks/wiki](https://github.com/daniel-retamal/fishtanks/wiki).
- **Fishing lessons.** Until you land three fish in a row, every fish fights at its calmest. Lose one and the count starts again; after three, fishing is the real thing.
- **Thirty-three new Rares.** Snails and slugs that crawl the glass, a hermit crab that swaps shells, a moray that slips through the walls, a boxfish that bounces like an old screensaver, an octopus that inks and wanders off, parrots that echo you, a shy fish that hides when you type, and more. Every one does something.
- **Twenty-eight new Commons**, told apart by their looks alone: Chilean fish, aquarium fish and a few invented ones. One wild fish in eight is still a Rare, however many species there are.
- **How fed is a fish?** `/index`, `/show` and the shop's Sell page now say: a percentage while food still pays, `Full` when it no longer does. A fish burps when its last pellet fills it. Chocolate Milk, candy and a Candyfish's touch now pay past full.
- **Mutations, reworked.** Fins, feet, spikes, wings and tentacles each take a row above or below the body. New growths: lures, bills, fins, the moon. `revert` undoes one earlier mutation exactly. A fish is only offered mutations that would show. `bubblecolor` is now `wakecolor`, and colours whatever a fish leaves behind.
- `fishtanks update` is in the README, where it always should have been.

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
