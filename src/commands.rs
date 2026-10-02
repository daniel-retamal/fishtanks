use crate::entities::cow::CowVariant;
use crate::fishes::mutations::Mutation;
use crate::fishes::species::{ALL_SPECIES, FishSpecies};
use crate::loot::{ConsumableKind, MilkVariant};
use crate::names::title_case;
use crate::tank::TankKind;
use crate::ui::fishing_overlay::Temper;
use crate::void_ritual::{GiveTarget, parse_give_target};

const TEMPER_FLAGS: [(&str, Temper); 2] = [
    ("--normal", Temper::Normal),
    ("--legendary", Temper::Legendary),
];

fn fish_temper(flags: &[&str]) -> Option<Temper> {
    TEMPER_FLAGS
        .iter()
        .find(|(flag, _)| flags.contains(flag))
        .map(|&(_, temper)| temper)
}

pub enum BuyTarget {
    Fish(FishSpecies),
    Tank(TankKind),
    Food { qty: u32 },
    Coffee { qty: u32 },
    Bait { qty: u32 },
}

impl BuyTarget {
    pub fn is_unique(&self) -> bool {
        matches!(self, BuyTarget::Fish(_) | BuyTarget::Tank(_))
    }
}

pub enum SellTarget {
    Fish(String),
    Tank(String),
    Blueprint(String),
    Junk { qty: u32 },
    Coffee { qty: u32 },
    Bait { qty: u32 },
    Milk { variant: MilkVariant, qty: u32 },
    Seed { kind: ConsumableKind, qty: u32 },
    Robotics { kind: ConsumableKind, qty: u32 },
}

impl SellTarget {
    pub fn is_unique(&self) -> bool {
        matches!(
            self,
            SellTarget::Fish(_) | SellTarget::Tank(_) | SellTarget::Blueprint(_)
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NamedKind {
    Fish,
    Tank,
    Blueprint,
}

impl NamedKind {
    pub const ALL: &'static [NamedKind] = &[NamedKind::Fish, NamedKind::Tank, NamedKind::Blueprint];

    pub fn keyword(self) -> &'static str {
        match self {
            NamedKind::Fish => "fish",
            NamedKind::Tank => "tank",
            NamedKind::Blueprint => "blueprint",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.keyword().eq_ignore_ascii_case(word))
    }

    fn sell(self, name: String) -> SellTarget {
        match self {
            NamedKind::Fish => SellTarget::Fish(name),
            NamedKind::Tank => SellTarget::Tank(name),
            NamedKind::Blueprint => SellTarget::Blueprint(name),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Clearance {
    #[default]
    Player,
    God,
    Debug,
}

pub const DEBUG_MODE_COMMAND: &str = "!debugmode";

fn clearance_of_word(word: &str) -> Clearance {
    COMMAND_NAMES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(word))
        .map(|&(_, clearance)| clearance)
        .unwrap_or(Clearance::Player)
}

pub struct Completion {
    pub ghost: String,
    pub tab_result: Option<String>,
}

#[derive(Default)]
pub struct CompletionCtx<'a> {
    pub fish_names: &'a [&'a str],
    pub consumable_names: &'a [&'a str],
    pub tank_names: &'a [&'a str],
    pub current_tank: &'a str,
    pub fish_in_tanks: &'a [(&'a str, &'a str)],
    pub has_cow_in_current: bool,
    pub entity_mutations: &'a [(&'a str, Vec<Mutation>)],
    pub graveyard_names: &'a [&'a str],
    pub programmable_names: &'a [&'a str],
    pub console_names: &'a [&'a str],
    pub blueprint_names: &'a [&'a str],
    pub arrangeable_names: &'a [&'a str],
    pub sellable_fish_names: &'a [String],
    pub sellable_tank_names: &'a [String],
    pub sellable_stackable_names: &'a [String],
    pub living_fish_names: &'a [String],
    pub clearance: Clearance,
}

impl CompletionCtx<'_> {
    fn sellable(&self, kind: NamedKind) -> Vec<&str> {
        match kind {
            NamedKind::Fish => self
                .sellable_fish_names
                .iter()
                .map(String::as_str)
                .collect(),
            NamedKind::Tank => self
                .sellable_tank_names
                .iter()
                .map(String::as_str)
                .collect(),
            NamedKind::Blueprint => self.blueprint_names.to_vec(),
        }
    }
}

fn all_mutation_tokens() -> Vec<&'static str> {
    let mut tokens: Vec<&'static str> = Mutation::ALL.iter().map(|m| m.token()).collect();
    tokens.sort_unstable();
    tokens
}

fn entity_mutation_tokens(
    name: &str,
    entity_mutations: &[(&str, Vec<Mutation>)],
) -> Vec<&'static str> {
    let Some((_, muts)) = entity_mutations
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
    else {
        return all_mutation_tokens();
    };
    let mut tokens: Vec<&'static str> = muts.iter().map(|m| m.token()).collect();
    tokens.sort_unstable();
    tokens
}

const FPS_ARG: &str = "<n>";
const CLOCK_ARG: &str = "<stages>";
const BLUEPRINT_ARG: &str = "<blueprint>";
const FISH_ARG: &str = "<fish>";
const ETCH_ARGS: &str = "<blueprint> <fish>";
const ETCH_COMMAND: &str = "etch";
const SELL_COMMAND: &str = "sell";
const SELL_ARG: &str = "<item>";
const NAME_ARG: &str = "<name>";
const QUANTITY_ARG: &str = "<quantity>";
const FILE_ARG: &str = "\"<file>\"";

static COMMAND_NAMES: &[(&str, Clearance)] = &[
    ("add", Clearance::Debug),
    ("bless", Clearance::God),
    ("buy", Clearance::Player),
    ("cheat", Clearance::Player),
    ("circuit", Clearance::Player),
    ("clock", Clearance::Player),
    ("clone", Clearance::God),
    ("console", Clearance::Player),
    ("consume", Clearance::Player),
    ("cowsay", Clearance::Player),
    ("etch", Clearance::Player),
    ("exit", Clearance::Player),
    ("expand", Clearance::God),
    ("export", Clearance::Player),
    ("feed", Clearance::Player),
    ("fish", Clearance::Player),
    ("fishtanks", Clearance::Player),
    ("ledger", Clearance::Player),
    ("flip", Clearance::Player),
    ("foundry", Clearance::Player),
    ("fps", Clearance::Player),
    ("freeze", Clearance::Player),
    ("give", Clearance::Debug),
    ("import", Clearance::Player),
    ("index", Clearance::Player),
    ("inventory", Clearance::Player),
    ("kill", Clearance::God),
    ("move", Clearance::Player),
    ("mutate", Clearance::God),
    ("names", Clearance::Player),
    ("nets", Clearance::Player),
    ("nudge", Clearance::Player),
    ("print", Clearance::Player),
    ("program", Clearance::Player),
    ("reset", Clearance::Debug),
    ("restore", Clearance::God),
    ("revive", Clearance::God),
    ("say", Clearance::Player),
    ("sell", Clearance::Player),
    ("shop", Clearance::Player),
    ("show", Clearance::Player),
    ("spawn", Clearance::God),
    ("startcallhome", Clearance::Debug),
    ("startcowabduction", Clearance::Debug),
    ("startfishabduction", Clearance::Debug),
    ("stats", Clearance::Player),
    ("subtract", Clearance::Debug),
    ("switch", Clearance::Player),
    ("unfreeze", Clearance::Player),
    ("voidspawn", Clearance::Debug),
    ("zen", Clearance::Player),
];

const BASE_RESOURCE_NAMES: &[&str] = &["food", "junk", "cash"];

fn resource_names() -> Vec<String> {
    let mut v: Vec<String> = BASE_RESOURCE_NAMES.iter().map(|s| s.to_string()).collect();
    for k in ConsumableKind::all() {
        v.push(k.lowercase_name());
    }
    v
}

pub fn autocomplete(input: &str, ctx: &CompletionCtx) -> Option<Completion> {
    if !input.starts_with('/') {
        return None;
    }
    let body = &input[1..];
    let word = body.split(' ').next().unwrap_or_default();
    if clearance_of_word(word) > ctx.clearance {
        return None;
    }

    match body.split_once(' ') {
        None => complete_command(body, ctx),
        Some((cmd, rest)) => match cmd.to_ascii_lowercase().as_str() {
            "add" => complete_add_subtract("add", rest),
            "buy" => complete_buy(rest),
            "consume" => complete_consume(rest, ctx.consumable_names),
            "fps" => complete_single_arg(rest, FPS_ARG),
            "clock" => complete_single_arg(rest, CLOCK_ARG),
            "index" => complete_index(rest, ctx.tank_names),
            "move" => complete_move(rest, ctx.fish_names, ctx.tank_names, ctx.fish_in_tanks),
            "mutate" => complete_mutate(rest, ctx.fish_names, ctx.entity_mutations),
            "sell" => complete_sell(rest, ctx),
            "show" => complete_show(rest, ctx.fish_in_tanks),
            "spawn" => complete_spawn(rest),
            "subtract" => complete_add_subtract("subtract", rest),
            "switch" => complete_switch(rest, ctx.tank_names, ctx.current_tank),
            "bless" => complete_name_arg("bless", rest, "<name>", &entity_names(ctx.fish_in_tanks)),
            "clone" => complete_name_arg("clone", rest, "<name>", &entity_names(ctx.fish_in_tanks)),
            "restore" => {
                complete_name_arg("restore", rest, "<name>", &entity_names(ctx.fish_in_tanks))
            }
            "revive" => complete_name_arg("revive", rest, "<name>", ctx.graveyard_names),
            "kill" => {
                let living: Vec<&str> = ctx.living_fish_names.iter().map(String::as_str).collect();
                complete_name_arg("kill", rest, "<name>", &living)
            }
            "expand" => complete_name_arg("expand", rest, "<tank>", ctx.tank_names),
            "program" => complete_name_arg("program", rest, "<name>", ctx.programmable_names),
            "console" => complete_name_arg("console", rest, NAME_ARG, ctx.console_names),
            "print" => complete_name_arg("print", rest, BLUEPRINT_ARG, ctx.blueprint_names),
            "etch" => complete_etch(rest, ctx),
            "freeze" => complete_name_arg("freeze", rest, "<name>", ctx.programmable_names),
            "unfreeze" => complete_name_arg("unfreeze", rest, "<name>", ctx.programmable_names),
            "flip" => complete_name_arg("flip", rest, "<name>", ctx.arrangeable_names),
            "nudge" => complete_name_arg("nudge", rest, "<name> <dx> <dy>", ctx.arrangeable_names),
            "give" => complete_give(rest),
            _ => None,
        },
    }
}

pub fn tab_complete(input: &str, ctx: &CompletionCtx) -> Option<String> {
    autocomplete(input, ctx).and_then(|c| c.tab_result)
}

fn complete_command(partial: &str, ctx: &CompletionCtx) -> Option<Completion> {
    if partial.is_empty() {
        return None;
    }
    let partial_lower = partial.to_ascii_lowercase();
    let allowed: Vec<&str> = COMMAND_NAMES
        .iter()
        .filter(|&&(_, clearance)| clearance <= ctx.clearance)
        .map(|&(name, _)| name)
        .filter(|&n| ctx.has_cow_in_current || n != "cowsay")
        .collect();

    if allowed.iter().any(|&n| n == partial_lower) {
        let args = command_args_placeholder(&partial_lower);
        return if args.is_empty() {
            None
        } else {
            Some(Completion {
                ghost: format!(" {}", args),
                tab_result: Some(format!("/{} ", partial_lower)),
            })
        };
    }

    let matches: Vec<&str> = allowed
        .iter()
        .copied()
        .filter(|&n| n.starts_with(partial_lower.as_str()))
        .collect();

    if matches.is_empty() {
        return None;
    }

    let first = matches[0];
    let args = command_args_placeholder(first);
    let ghost = if args.is_empty() {
        first[partial_lower.len()..].to_string()
    } else {
        format!("{} {}", &first[partial_lower.len()..], args)
    };

    let tab_result = if matches.len() == 1 {
        Some(format!("/{} ", first))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > partial_lower.len() {
            Some(format!("/{}", cp))
        } else {
            Some(format!("/{} ", first))
        }
    };

    Some(Completion { ghost, tab_result })
}

fn complete_single_arg(rest: &str, placeholder: &str) -> Option<Completion> {
    if !rest.is_empty() {
        return None;
    }
    Some(Completion {
        ghost: placeholder.to_string(),
        tab_result: None,
    })
}

fn complete_add_subtract(cmd: &str, rest: &str) -> Option<Completion> {
    if rest.is_empty() {
        return Some(Completion {
            ghost: "<resource> <amount>".to_string(),
            tab_result: None,
        });
    }
    let resources = resource_names();
    let rest_lower = rest.to_ascii_lowercase();
    let rest_trimmed = rest_lower.trim_end();

    for resource in &resources {
        if rest_trimmed == resource.as_str() {
            let space = if rest.ends_with(' ') { "" } else { " " };
            return Some(Completion {
                ghost: format!("{}<amount>", space),
                tab_result: None,
            });
        }
        let prefix_with_space = format!("{} ", resource);
        if rest_lower.starts_with(&prefix_with_space) {
            let after = &rest_lower[prefix_with_space.len()..];
            if after.is_empty() {
                return Some(Completion {
                    ghost: "<amount>".to_string(),
                    tab_result: None,
                });
            }
            return None;
        }
    }

    let typed_len = rest.len();
    let matches: Vec<&str> = resources
        .iter()
        .map(|s| s.as_str())
        .filter(|r| r.starts_with(rest_lower.as_str()))
        .collect();
    if matches.is_empty() {
        return None;
    }
    let first = matches[0];
    let ghost = format!("{} <amount>", &first[typed_len..]);
    let tab_result = if matches.len() == 1 {
        Some(format!("/{} {} ", cmd, first))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > typed_len {
            Some(format!("/{} {}", cmd, cp))
        } else {
            Some(format!("/{} {} ", cmd, first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_mutate(
    rest: &str,
    fish_names: &[&str],
    entity_mutations: &[(&str, Vec<Mutation>)],
) -> Option<Completion> {
    if rest.is_empty() {
        return Some(Completion {
            ghost: "<name> <mutation>".to_string(),
            tab_result: None,
        });
    }

    let (quoted, inner) = if let Some(s) = rest.strip_prefix('"') {
        (true, s)
    } else {
        (false, rest)
    };

    if quoted {
        if let Some(end_pos) = inner.find('"') {
            let name = &inner[..end_pos];
            let after = inner[end_pos + 1..].trim_start();
            if after.is_empty() {
                return Some(Completion {
                    ghost: "<mutation>".to_string(),
                    tab_result: Some(format!("/mutate \"{}\" ", name)),
                });
            }
            return complete_mutation_part(after, name, true, entity_mutations);
        }
        let inner_lower = inner.to_ascii_lowercase();
        let matches: Vec<&str> = fish_names
            .iter()
            .copied()
            .filter(|&n| n.to_ascii_lowercase().starts_with(inner_lower.as_str()))
            .collect();
        if matches.is_empty() {
            return Some(Completion {
                ghost: "\" <mutation>".to_string(),
                tab_result: None,
            });
        }
        let first = matches[0];
        let ghost = format!("{}\" <mutation>", &first[inner.len()..]);
        let tab_result = if matches.len() == 1 {
            Some(format!("/mutate \"{}\" ", first))
        } else {
            let cp = longest_common_prefix(&matches);
            if cp.len() > inner.len() {
                Some(format!("/mutate \"{}", cp))
            } else {
                Some(format!("/mutate \"{}\" ", first))
            }
        };
        return Some(Completion { ghost, tab_result });
    }

    let words: Vec<&str> = inner.split_whitespace().collect();
    for end in (1..=words.len()).rev() {
        let candidate = words[..end].join(" ");
        let candidate_lower = candidate.to_ascii_lowercase();
        if let Some(&display) = fish_names
            .iter()
            .find(|&&n| n.to_ascii_lowercase() == candidate_lower)
        {
            let after_words = &words[end..];
            if after_words.is_empty() {
                let trailing = rest.ends_with(' ');
                return Some(Completion {
                    ghost: if trailing {
                        "<mutation>".to_string()
                    } else {
                        " <mutation>".to_string()
                    },
                    tab_result: Some(format!("/mutate {} ", display)),
                });
            }
            let after = after_words.join(" ");
            return complete_mutation_part(&after, display, false, entity_mutations);
        }
    }

    let partial_lower = inner.to_ascii_lowercase();
    let matches: Vec<&str> = fish_names
        .iter()
        .copied()
        .filter(|&n| n.to_ascii_lowercase().starts_with(partial_lower.as_str()))
        .collect();
    if matches.is_empty() {
        return None;
    }
    let first = matches[0];
    let ghost = format!("{} <mutation>", &first[inner.len()..]);
    let tab_result = if matches.len() == 1 {
        Some(format!("/mutate {} ", first))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > inner.len() {
            Some(format!("/mutate {}", cp))
        } else {
            Some(format!("/mutate {} ", first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_mutation_part(
    after: &str,
    name: &str,
    quoted: bool,
    entity_mutations: &[(&str, Vec<Mutation>)],
) -> Option<Completion> {
    let after_lower = after.to_ascii_lowercase();
    let allowed = entity_mutation_tokens(name, entity_mutations);
    let matches: Vec<&str> = allowed
        .iter()
        .copied()
        .filter(|&m| m.starts_with(after_lower.as_str()))
        .collect();
    if matches.is_empty() {
        return None;
    }
    let first = matches[0];
    let ghost = first[after.len()..].to_string();
    let prefix = if quoted {
        format!("/mutate \"{}\" ", name)
    } else {
        format!("/mutate {} ", name)
    };
    let tab_result = if matches.len() == 1 {
        Some(format!("{}{}", prefix, first))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > after.len() {
            Some(format!("{}{}", prefix, cp))
        } else {
            Some(format!("{}{}", prefix, first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_spawn(rest: &str) -> Option<Completion> {
    match rest.split_once(' ') {
        None => complete_species(rest),
        Some((_, name_rest)) => {
            if name_rest.is_empty() {
                Some(Completion {
                    ghost: "<name>".to_string(),
                    tab_result: None,
                })
            } else {
                None
            }
        }
    }
}

fn complete_species(partial: &str) -> Option<Completion> {
    if partial.is_empty() {
        return Some(Completion {
            ghost: "<species> <name>".to_string(),
            tab_result: None,
        });
    }
    let partial_lower = partial.to_ascii_lowercase();
    let names: Vec<String> = ALL_SPECIES
        .iter()
        .map(|s| s.display_name().to_ascii_lowercase())
        .collect();
    let matches: Vec<&str> = names
        .iter()
        .map(|s| s.as_str())
        .filter(|n| n.starts_with(partial_lower.as_str()))
        .collect();
    if matches.is_empty() {
        return None;
    }
    let first = matches[0];
    let ghost = format!("{} <name>", &first[partial.len()..]);
    let tab_result = if matches.len() == 1 {
        Some(format!("/spawn {} ", first))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > partial.len() {
            Some(format!("/spawn {}", cp))
        } else {
            Some(format!("/spawn {} ", first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn finish_name_tab(cmd: &str, name: &str, quoted: bool) -> String {
    if quoted {
        format!("/{} \"{}\"", cmd, name)
    } else {
        format!("/{} {}", cmd, name)
    }
}

fn complete_name_arg(
    cmd: &str,
    rest: &str,
    placeholder: &str,
    candidates: &[&str],
) -> Option<Completion> {
    if rest.is_empty() {
        if candidates.is_empty() {
            return None;
        }
        return Some(Completion {
            ghost: placeholder.to_string(),
            tab_result: None,
        });
    }
    let (quoted, inner) = if let Some(s) = rest.strip_prefix('"') {
        (true, s)
    } else {
        (false, rest)
    };
    if quoted && inner.contains('"') {
        return None;
    }
    let inner_lower = inner.to_ascii_lowercase();
    let matches: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|&n| n.to_ascii_lowercase().starts_with(inner_lower.as_str()))
        .collect();
    if matches.is_empty() {
        if quoted {
            return Some(Completion {
                ghost: "\"".to_string(),
                tab_result: None,
            });
        }
        return None;
    }
    let first = matches[0];
    let ghost = if quoted {
        format!("{}\"", &first[inner.len()..])
    } else {
        first[inner.len()..].to_string()
    };
    let tab_result = if matches.len() == 1 {
        Some(finish_name_tab(cmd, first, quoted))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > inner.len() {
            Some(if quoted {
                format!("/{} \"{}", cmd, cp)
            } else {
                format!("/{} {}", cmd, cp)
            })
        } else {
            Some(finish_name_tab(cmd, first, quoted))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_etch(rest: &str, ctx: &CompletionCtx) -> Option<Completion> {
    if rest.is_empty() {
        return Some(Completion {
            ghost: ETCH_ARGS.to_string(),
            tab_result: None,
        });
    }
    let Some((blueprint, fish_rest)) = split_first_arg(rest) else {
        return complete_name_arg(ETCH_COMMAND, rest, ETCH_ARGS, ctx.blueprint_names);
    };
    let known = ctx
        .blueprint_names
        .iter()
        .find(|name| name.eq_ignore_ascii_case(&blueprint))?;
    let command = format!("{ETCH_COMMAND} \"{known}\"");
    complete_name_arg(&command, &fish_rest, FISH_ARG, ctx.programmable_names)
}

fn complete_consume(rest: &str, consumable_names: &[&str]) -> Option<Completion> {
    complete_name_arg("consume", rest, "<consumable>", consumable_names)
}

fn complete_switch(rest: &str, tank_names: &[&str], current_tank: &str) -> Option<Completion> {
    let available: Vec<&str> = tank_names
        .iter()
        .copied()
        .filter(|&n| !n.eq_ignore_ascii_case(current_tank))
        .collect();
    if available.is_empty() {
        return None;
    }
    complete_name_arg("switch", rest, "<name>", &available)
}

fn entity_names<'a>(fish_in_tanks: &[(&'a str, &'a str)]) -> Vec<&'a str> {
    fish_in_tanks.iter().map(|(n, _)| *n).collect()
}

fn give_target_names() -> Vec<String> {
    let mut v: Vec<String> = vec!["cash".to_string(), "food".to_string(), "junk".to_string()];
    for kind in ConsumableKind::all() {
        v.push(kind.lowercase_name());
    }
    for &species in ALL_SPECIES {
        v.push(species.config().name.to_ascii_lowercase());
    }
    for &kind in TankKind::all() {
        if !kind.config().sellable {
            continue;
        }
        v.push(kind.display_name().to_ascii_lowercase());
    }
    v.push("cow".to_string());
    for variant in CowVariant::ALL {
        v.push(format!("{} cow", variant.display_name()));
    }
    v
}

fn complete_give(rest: &str) -> Option<Completion> {
    let names = give_target_names();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    complete_name_arg("give", rest, "<thing>", &refs)
}

fn complete_move(
    rest: &str,
    fish_names: &[&str],
    tank_names: &[&str],
    fish_in_tanks: &[(&str, &str)],
) -> Option<Completion> {
    if rest.is_empty() {
        return Some(Completion {
            ghost: "<entity> <tank>".to_string(),
            tab_result: None,
        });
    }

    let (fish_quoted, inner) = if let Some(s) = rest.strip_prefix('"') {
        (true, s)
    } else {
        (false, rest)
    };

    if fish_quoted {
        if let Some(end_pos) = inner.find('"') {
            let fish_name = &inner[..end_pos];
            let after_fish = inner[end_pos + 1..].trim_start();
            if after_fish.is_empty() {
                return Some(Completion {
                    ghost: "<tank>".to_string(),
                    tab_result: Some(format!("/move \"{}\" \"", fish_name)),
                });
            }
            let (tank_quoted, tank_inner) = if let Some(s) = after_fish.strip_prefix('"') {
                (true, s)
            } else {
                (false, after_fish)
            };
            if tank_quoted && tank_inner.contains('"') {
                return None;
            }
            return complete_move_tank(
                fish_name,
                tank_inner,
                tank_names,
                fish_in_tanks,
                true,
                tank_quoted,
            );
        }
        let inner_lower = inner.to_ascii_lowercase();
        let matches: Vec<&str> = fish_names
            .iter()
            .copied()
            .filter(|&n| n.to_ascii_lowercase().starts_with(inner_lower.as_str()))
            .collect();
        if matches.is_empty() {
            return Some(Completion {
                ghost: "\" <tank>\"".to_string(),
                tab_result: None,
            });
        }
        let first = matches[0];
        let ghost = format!("{}\" <tank>\"", &first[inner.len()..]);
        let tab_result = if matches.len() == 1 {
            Some(format!("/move \"{}\" \"", first))
        } else {
            let cp = longest_common_prefix(&matches);
            if cp.len() > inner.len() {
                Some(format!("/move \"{}", cp))
            } else {
                Some(format!("/move \"{}\" \"", first))
            }
        };
        return Some(Completion { ghost, tab_result });
    }

    let words: Vec<&str> = inner.split_whitespace().collect();
    for end in (1..=words.len()).rev() {
        let candidate = words[..end].join(" ");
        if let Some(&display) = fish_names
            .iter()
            .find(|&&n| n.eq_ignore_ascii_case(&candidate))
        {
            let after_words = &words[end..];
            if after_words.is_empty() {
                let trailing = rest.ends_with(' ');
                return Some(Completion {
                    ghost: if trailing {
                        "<tank>".to_string()
                    } else {
                        " <tank>".to_string()
                    },
                    tab_result: Some(format!("/move {} ", display)),
                });
            }
            let tank_partial = after_words.join(" ");
            return complete_move_tank(
                display,
                &tank_partial,
                tank_names,
                fish_in_tanks,
                false,
                false,
            );
        }
    }

    let partial_lower = inner.to_ascii_lowercase();
    let matches: Vec<&str> = fish_names
        .iter()
        .copied()
        .filter(|&n| n.to_ascii_lowercase().starts_with(partial_lower.as_str()))
        .collect();
    if matches.is_empty() {
        return None;
    }
    let first = matches[0];
    let ghost = format!("{} <tank>", &first[inner.len()..]);
    let tab_result = if matches.len() == 1 {
        Some(format!("/move {} ", first))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > inner.len() {
            Some(format!("/move {}", cp))
        } else {
            Some(format!("/move {} ", first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_move_tank(
    fish_name: &str,
    tank_partial: &str,
    tank_names: &[&str],
    fish_in_tanks: &[(&str, &str)],
    fish_quoted: bool,
    tank_quoted: bool,
) -> Option<Completion> {
    let fish_home: Option<&str> = fish_in_tanks
        .iter()
        .find(|(fn_, _)| fn_.eq_ignore_ascii_case(fish_name))
        .map(|(_, tn)| *tn);
    let available: Vec<&str> = tank_names
        .iter()
        .copied()
        .filter(|&t| fish_home.is_none_or(|h| !t.eq_ignore_ascii_case(h)))
        .collect();
    let partial_lower = tank_partial.to_ascii_lowercase();
    let matches: Vec<&str> = available
        .iter()
        .copied()
        .filter(|&n| n.to_ascii_lowercase().starts_with(partial_lower.as_str()))
        .collect();
    let fish_part = if fish_quoted {
        format!("/move \"{}\" ", fish_name)
    } else {
        format!("/move {} ", fish_name)
    };
    if matches.is_empty() {
        if tank_quoted {
            return Some(Completion {
                ghost: "\"".to_string(),
                tab_result: None,
            });
        }
        return None;
    }
    let first = matches[0];
    let ghost = if tank_quoted {
        format!("{}\"", &first[tank_partial.len()..])
    } else {
        first[tank_partial.len()..].to_string()
    };
    let tab_result = if matches.len() == 1 {
        if tank_quoted {
            Some(format!("{}\"{}\"", fish_part, first))
        } else {
            Some(format!("{}{}", fish_part, first))
        }
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > tank_partial.len() {
            if tank_quoted {
                Some(format!("{}\"{}", fish_part, cp))
            } else {
                Some(format!("{}{}", fish_part, cp))
            }
        } else if tank_quoted {
            Some(format!("{}\"{}\"", fish_part, first))
        } else {
            Some(format!("{}{}", fish_part, first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_index(rest: &str, tank_names: &[&str]) -> Option<Completion> {
    if tank_names.is_empty() {
        return None;
    }
    if rest.is_empty() {
        return Some(Completion {
            ghost: "<name>".to_string(),
            tab_result: None,
        });
    }
    let (quoted, inner) = if let Some(s) = rest.strip_prefix('"') {
        (true, s)
    } else {
        (false, rest)
    };
    if quoted && inner.contains('"') {
        return None;
    }
    let inner_lower = inner.to_ascii_lowercase();
    let matches: Vec<&str> = tank_names
        .iter()
        .copied()
        .filter(|&n| n.to_ascii_lowercase().starts_with(inner_lower.as_str()))
        .collect();
    if matches.is_empty() {
        if quoted {
            return Some(Completion {
                ghost: "\"".to_string(),
                tab_result: None,
            });
        }
        return None;
    }
    let first = matches[0];
    let ghost = if quoted {
        format!("{}\"", &first[inner.len()..])
    } else {
        first[inner.len()..].to_string()
    };
    let tab_result = if matches.len() == 1 {
        if quoted {
            Some(format!("/index \"{}\"", first))
        } else {
            Some(format!("/index {}", first))
        }
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > inner.len() {
            if quoted {
                Some(format!("/index \"{}", cp))
            } else {
                Some(format!("/index {}", cp))
            }
        } else if quoted {
            Some(format!("/index \"{}\"", first))
        } else {
            Some(format!("/index {}", first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_show(rest: &str, fish_in_tanks: &[(&str, &str)]) -> Option<Completion> {
    let all_names: Vec<&str> = fish_in_tanks.iter().map(|(n, _)| *n).collect();
    if all_names.is_empty() {
        return None;
    }
    if rest.is_empty() {
        return Some(Completion {
            ghost: "<name>".to_string(),
            tab_result: None,
        });
    }
    let (quoted, inner) = if let Some(s) = rest.strip_prefix('"') {
        (true, s)
    } else {
        (false, rest)
    };
    if quoted && inner.contains('"') {
        return None;
    }
    let inner_lower = inner.to_ascii_lowercase();
    let matches: Vec<&str> = all_names
        .iter()
        .copied()
        .filter(|&n| n.to_ascii_lowercase().starts_with(inner_lower.as_str()))
        .collect();
    if matches.is_empty() {
        if quoted {
            return Some(Completion {
                ghost: "\"".to_string(),
                tab_result: None,
            });
        }
        return None;
    }
    let first = matches[0];
    let ghost = if quoted {
        format!("{}\"", &first[inner.len()..])
    } else {
        first[inner.len()..].to_string()
    };
    let tab_result = if matches.len() == 1 {
        if quoted {
            Some(format!("/show \"{}\"", first))
        } else {
            Some(format!("/show {}", first))
        }
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > inner.len() {
            if quoted {
                Some(format!("/show \"{}", cp))
            } else {
                Some(format!("/show {}", cp))
            }
        } else if quoted {
            Some(format!("/show \"{}\"", first))
        } else {
            Some(format!("/show {}", first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn split_trailing_qty(rest: &str) -> (&str, u32) {
    let rest = rest.trim();
    if let Some((name, qty_str)) = rest.rsplit_once(char::is_whitespace)
        && let Ok(n) = qty_str.trim().parse::<u32>()
        && n > 0
    {
        return (name.trim(), n);
    }
    (rest, 1)
}

fn buyable_item_names() -> Vec<String> {
    let mut v: Vec<String> = FishSpecies::all_buyable()
        .iter()
        .map(|s| s.display_name().to_ascii_lowercase())
        .collect();
    for kind in TankKind::all_buyable() {
        v.push(kind.display_name().to_ascii_lowercase());
    }
    v.push("food".to_string());
    v.push("coffee".to_string());
    v.push("bait".to_string());
    v
}

const STACKABLE_BUY_NAMES: &[&str] = &["food", "coffee", "bait"];

fn resolve_stackable_sell(name_lower: &str, qty: u32) -> Option<SellTarget> {
    match name_lower {
        "junk" => Some(SellTarget::Junk { qty }),
        "coffee" => Some(SellTarget::Coffee { qty }),
        "bait" => Some(SellTarget::Bait { qty }),
        _ => MilkVariant::ALL
            .iter()
            .find(|&&v| v.display_name().to_ascii_lowercase() == name_lower)
            .map(|&variant| SellTarget::Milk { variant, qty })
            .or_else(|| {
                ConsumableKind::seeds()
                    .into_iter()
                    .find(|kind| kind.lowercase_name() == name_lower)
                    .map(|kind| SellTarget::Seed { kind, qty })
            })
            .or_else(|| {
                ConsumableKind::robotics_stock()
                    .into_iter()
                    .find(|kind| kind.lowercase_name() == name_lower)
                    .map(|kind| SellTarget::Robotics { kind, qty })
            }),
    }
}

fn complete_buy(rest: &str) -> Option<Completion> {
    let names = buyable_item_names();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();

    if rest.is_empty() {
        return Some(Completion {
            ghost: "<item>".to_string(),
            tab_result: None,
        });
    }

    let rest_lower = rest.to_ascii_lowercase();
    let rest_trimmed = rest_lower.trim_end();

    for &name in STACKABLE_BUY_NAMES {
        if rest_trimmed == name {
            let space = if rest.ends_with(' ') { "" } else { " " };
            return Some(Completion {
                ghost: format!("{}<quantity>", space),
                tab_result: None,
            });
        }
        let prefix_space = format!("{} ", name);
        if rest_lower.starts_with(&prefix_space) {
            let after = &rest_lower[prefix_space.len()..];
            return if after.is_empty() {
                Some(Completion {
                    ghost: "<quantity>".to_string(),
                    tab_result: None,
                })
            } else {
                None
            };
        }
    }

    if refs.contains(&rest_trimmed) {
        return None;
    }

    let typed_len = rest_trimmed.len();
    let matches: Vec<&str> = refs
        .iter()
        .copied()
        .filter(|&n| n.starts_with(rest_trimmed))
        .collect();

    if matches.is_empty() {
        return None;
    }

    let first = matches[0];
    let is_stackable = STACKABLE_BUY_NAMES.contains(&first);
    let ghost = if is_stackable {
        format!("{} <quantity>", &first[typed_len..])
    } else {
        first[typed_len..].to_string()
    };

    let tab_result = if matches.len() == 1 {
        Some(format!("/buy {}", first))
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > typed_len {
            Some(format!("/buy {}", cp))
        } else {
            Some(format!("/buy {} ", first))
        }
    };
    Some(Completion { ghost, tab_result })
}

fn complete_sell(rest: &str, ctx: &CompletionCtx) -> Option<Completion> {
    let owned: Vec<(NamedKind, Vec<&str>)> = NamedKind::ALL
        .iter()
        .map(|&kind| (kind, ctx.sellable(kind)))
        .filter(|(_, names)| !names.is_empty())
        .collect();
    let stackable: Vec<&str> = ctx
        .sellable_stackable_names
        .iter()
        .map(String::as_str)
        .collect();

    if owned.is_empty() && stackable.is_empty() {
        return None;
    }

    if rest.is_empty() {
        return Some(Completion {
            ghost: SELL_ARG.to_string(),
            tab_result: None,
        });
    }

    let rest_lower = rest.to_ascii_lowercase();
    let rest_trimmed = rest_lower.trim_end();

    for (kind, names) in &owned {
        if let Some(after) = rest_lower.strip_prefix(&format!("{} ", kind.keyword())) {
            let name = &rest[rest.len() - after.len()..];
            let cmd = format!("{SELL_COMMAND} {}", kind.keyword());
            return complete_name_arg(&cmd, name, NAME_ARG, names);
        }
    }

    for &name in &stackable {
        let name_lower = name.to_ascii_lowercase();
        if rest_trimmed == name_lower {
            let space = if rest.ends_with(' ') { "" } else { " " };
            return Some(Completion {
                ghost: format!("{space}{QUANTITY_ARG}"),
                tab_result: None,
            });
        }
        let prefix_space = format!("{} ", name_lower);
        if rest_lower.starts_with(&prefix_space) {
            let after = &rest_lower[prefix_space.len()..];
            return if after.is_empty() {
                Some(Completion {
                    ghost: QUANTITY_ARG.to_string(),
                    tab_result: None,
                })
            } else {
                None
            };
        }
    }

    let keywords: Vec<&str> = owned.iter().map(|(kind, _)| kind.keyword()).collect();
    if keywords.contains(&rest_trimmed) {
        return Some(Completion {
            ghost: format!(" {NAME_ARG}"),
            tab_result: Some(format!("/{SELL_COMMAND} {rest_trimmed} ")),
        });
    }

    let typed_len = rest_trimmed.len();
    let matches: Vec<&str> = keywords
        .iter()
        .chain(stackable.iter())
        .copied()
        .filter(|&n| n.to_ascii_lowercase().starts_with(rest_trimmed))
        .collect();

    let first = *matches.first()?;
    let is_keyword = keywords.contains(&first);
    let argument = if is_keyword { NAME_ARG } else { QUANTITY_ARG };
    let ghost = format!("{} {argument}", &first[typed_len..]);

    let tab_result = if matches.len() == 1 && is_keyword {
        format!("/{SELL_COMMAND} {first} ")
    } else if matches.len() == 1 {
        format!("/{SELL_COMMAND} {first}")
    } else {
        let cp = longest_common_prefix(&matches);
        if cp.len() > typed_len {
            format!("/{SELL_COMMAND} {cp}")
        } else {
            format!("/{SELL_COMMAND} {first} ")
        }
    };
    Some(Completion {
        ghost,
        tab_result: Some(tab_result),
    })
}

fn parse_buy(rest: &str) -> Action {
    if rest.is_empty() {
        return Action::Unknown;
    }
    let (name_part, qty) = split_trailing_qty(rest);
    let name_lower = name_part.to_ascii_lowercase();

    if let Some(species) = FishSpecies::parse(&name_lower) {
        return if species.config().buyable {
            Action::Buy(BuyTarget::Fish(species))
        } else {
            Action::Unknown
        };
    }
    if let Some(kind) = TankKind::parse(&name_lower) {
        return Action::Buy(BuyTarget::Tank(kind));
    }
    match name_lower.as_str() {
        "food" => Action::Buy(BuyTarget::Food { qty }),
        "coffee" => Action::Buy(BuyTarget::Coffee { qty }),
        "bait" => Action::Buy(BuyTarget::Bait { qty }),
        _ => Action::Unknown,
    }
}

fn parse_sell(rest: &str) -> Action {
    let rest = rest.trim();
    if let Some((word, named)) = rest.split_once(char::is_whitespace)
        && let Some(kind) = NamedKind::parse(word)
    {
        let name = parse_rest_or_quoted(named);
        if name.trim().is_empty() {
            return Action::Unknown;
        }
        return Action::Sell(kind.sell(name));
    }
    let (name_part, qty) = split_trailing_qty(rest);
    match resolve_stackable_sell(&name_part.to_ascii_lowercase(), qty) {
        Some(target) => Action::Sell(target),
        None => Action::Unknown,
    }
}

fn command_args_placeholder(cmd: &str) -> &'static str {
    match cmd {
        "add" | "subtract" => "<resource> <amount>",
        "buy" => "<item>",
        "consume" => "<consumable>",
        "cowsay" => "\"<text>\"",
        "say" => "\"<text>\"",
        "fps" => FPS_ARG,
        "clock" => CLOCK_ARG,
        "index" | "show" | "switch" => "<name>",
        "bless" | "clone" | "restore" | "revive" | "kill" | "program" | "console" | "freeze"
        | "unfreeze" | "flip" => "<name>",
        "nudge" => "<name> <dx> <dy>",
        "print" => BLUEPRINT_ARG,
        "etch" => ETCH_ARGS,
        "expand" => "<tank>",
        "give" => "<thing>",
        "export" | "import" => FILE_ARG,
        "move" => "<entity> <tank>",
        "mutate" => "<name> <mutation>",
        "sell" => SELL_ARG,
        "spawn" => "<species> <name>",
        _ => "",
    }
}

fn longest_common_prefix<'a>(strings: &[&'a str]) -> &'a str {
    if strings.is_empty() {
        return "";
    }
    let first = strings[0];
    let len = strings.iter().skip(1).fold(first.len(), |acc, s| {
        first
            .chars()
            .zip(s.chars())
            .take_while(|(a, b)| a == b)
            .count()
            .min(acc)
    });
    &first[..len]
}

pub enum Action {
    Feed,
    SetFps(f32),
    SetClock(u32),
    Spawn(FishSpecies, String),
    Mutate(String, String),
    Show {
        name: String,
        all: bool,
    },
    Index {
        all: bool,
        tank_filter: Option<String>,
    },
    Fish {
        no_escape: bool,
        no_fight: bool,
        temper: Option<Temper>,
    },
    Inventory,
    Shop,
    Consume {
        name: String,
    },
    ToggleNames,
    ToggleNets,
    ToggleStats,
    Zen,
    ModResource {
        name: String,
        delta: i32,
    },
    Switch(String),
    Move {
        fish: String,
        tank: String,
    },
    Fishtanks,
    Ledger,
    Circuit,
    Foundry,
    Exit,
    Reset,
    Export(String),
    Import(String),
    Cowsay(String),
    Say(String),
    VoidSpawn,
    StartVoidWish {
        skip: bool,
    },
    StartFishAbduction,
    StartCowAbduction,
    StartCallHome,
    Give(GiveTarget),
    Revive(String),
    Kill(String),
    Clone(String),
    Bless,
    Expand(String),
    Restore(String),
    Program(String),
    Console(String),
    Print(String),
    Etch {
        blueprint: String,
        fish: String,
    },
    SetFrozen {
        name: String,
        frozen: bool,
    },
    Nudge {
        name: String,
        dx: i32,
        dy: i32,
    },
    Flip(String),
    Buy(BuyTarget),
    Sell(SellTarget),
    Cheat,
    ToggleDebugMode,
    Unknown,
}

fn greedy_name_words<'a>(words: &[&str], names: &[&'a str]) -> Option<(usize, &'a str)> {
    for end in (1..=words.len()).rev() {
        let candidate = words[..end].join(" ");
        if let Some(&display) = names.iter().find(|&&n| n.eq_ignore_ascii_case(&candidate)) {
            return Some((end, display));
        }
    }
    None
}

fn parse_rest_or_quoted(rest: &str) -> String {
    let rest = rest.trim();
    if let Some(inner) = rest.strip_prefix('"')
        && let Some(end) = inner.find('"')
    {
        return inner[..end].to_string();
    }
    if let Some(inner) = rest.strip_prefix('\'')
        && let Some(end) = inner.find('\'')
    {
        return inner[..end].to_string();
    }
    rest.to_string()
}

fn name_command(rest: &str, make: fn(String) -> Action) -> Action {
    let name = parse_rest_or_quoted(rest);
    if name.trim().is_empty() {
        Action::Unknown
    } else {
        make(name)
    }
}

fn parse_raw_arg(rest: &str) -> String {
    let rest = rest.trim();
    if let Some(inner) = rest.strip_prefix('"')
        && let Some(end) = inner.find('"')
    {
        return inner[..end].to_string();
    }
    if let Some(inner) = rest.strip_prefix('\'')
        && let Some(end) = inner.find('\'')
    {
        return inner[..end].to_string();
    }
    rest.split_whitespace().next().unwrap_or("").to_string()
}

fn parse_name_greedy(rest: &str, names: &[&str]) -> Option<String> {
    split_name(rest, names).map(|(name, _)| name)
}

fn split_name(rest: &str, names: &[&str]) -> Option<(String, String)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    if rest.starts_with('"') || rest.starts_with('\'') {
        let quote = rest.chars().next().unwrap();
        let inner = &rest[1..];
        let end = inner.find(quote)?;
        let name = inner[..end].to_string();
        if name.is_empty() {
            return None;
        }
        return Some((name, inner[end + 1..].trim().to_string()));
    }
    let words: Vec<&str> = rest.split_whitespace().collect();
    let (consumed, display) = greedy_name_words(&words, names)?;
    Some((display.to_string(), words[consumed..].join(" ")))
}

fn parse_name_and_all_flag(rest: &str, names: &[&str]) -> Option<(String, bool)> {
    let (name, after) = split_name(rest, names)?;
    Some((name, after.eq_ignore_ascii_case("all")))
}

fn parse_fish_tank_args(
    rest: &str,
    fish_names: &[&str],
    tank_names: &[&str],
) -> Option<(String, String)> {
    let (fish, tank_rest) = split_name(rest, fish_names)?;
    let tank = parse_name_greedy(&tank_rest, tank_names)?;
    Some((fish, tank))
}

fn split_first_arg(rest: &str) -> Option<(String, String)> {
    let rest = rest.trim_start();
    if rest.starts_with('"') || rest.starts_with('\'') {
        return split_name(rest, &[]);
    }
    let (word, after) = rest.split_once(' ')?;
    Some((word.to_string(), after.to_string()))
}

fn parse_etch(rest: &str, fish_names: &[&str]) -> Option<Action> {
    let (blueprint, fish_rest) = split_first_arg(rest)?;
    let fish = parse_name_greedy(&fish_rest, fish_names)?;
    Some(Action::Etch { blueprint, fish })
}

fn parse_nudge(rest: &str, fish_names: &[&str]) -> Option<Action> {
    let (name, offsets) = split_name(rest, fish_names)?;
    let mut words = offsets.split_whitespace();
    let dx = words.next()?.parse::<i32>().ok()?;
    let dy = words.next()?.parse::<i32>().ok()?;
    Some(Action::Nudge { name, dx, dy })
}

pub fn parse(input: &str, fish_names: &[&str], tank_names: &[&str]) -> Action {
    let input = input.trim();
    if input.eq_ignore_ascii_case(DEBUG_MODE_COMMAND) {
        return Action::ToggleDebugMode;
    }
    if !input.starts_with('/') {
        return Action::Unknown;
    }
    let body = &input[1..];

    let (cmd_lower, rest) = match body.find(' ') {
        Some(pos) => (body[..pos].to_ascii_lowercase(), body[pos + 1..].trim()),
        None => (body.to_ascii_lowercase(), ""),
    };

    match cmd_lower.as_str() {
        "mutate" => {
            if rest.is_empty() {
                return Action::Unknown;
            }
            match split_name(rest, fish_names) {
                Some((name, mutation)) => Action::Mutate(name, mutation),
                None => Action::Unknown,
            }
        }
        "switch" => match parse_name_greedy(rest, tank_names) {
            Some(name) if !name.is_empty() => Action::Switch(name),
            _ => Action::Unknown,
        },
        "move" => {
            if rest.is_empty() {
                return Action::Unknown;
            }
            match parse_fish_tank_args(rest, fish_names, tank_names) {
                Some((fish, tank)) => Action::Move { fish, tank },
                None => Action::Unknown,
            }
        }
        "show" => {
            if rest.is_empty() {
                return Action::Unknown;
            }
            match parse_name_and_all_flag(rest, fish_names) {
                Some((name, all)) => Action::Show { name, all },
                None => Action::Unknown,
            }
        }
        "index" => {
            if rest.is_empty() {
                return Action::Index {
                    all: false,
                    tank_filter: None,
                };
            }
            if rest.eq_ignore_ascii_case("all") {
                return Action::Index {
                    all: true,
                    tank_filter: None,
                };
            }
            match parse_name_greedy(rest, tank_names) {
                Some(name) if !name.is_empty() => Action::Index {
                    all: false,
                    tank_filter: Some(name),
                },
                _ => Action::Unknown,
            }
        }
        "consume" => {
            let name = parse_rest_or_quoted(rest);
            if name.is_empty() {
                Action::Unknown
            } else {
                Action::Consume { name }
            }
        }
        "feed" => {
            if rest.is_empty() || rest.parse::<u64>().is_ok() {
                return Action::Feed;
            }
            Action::Unknown
        }
        "fps" => rest.parse().map(Action::SetFps).unwrap_or(Action::Unknown),
        "clock" => rest
            .parse()
            .map(Action::SetClock)
            .unwrap_or(Action::Unknown),
        "add" | "subtract" => {
            let trimmed = rest.trim();
            let (resource, n_str) = match trimmed.rsplit_once(char::is_whitespace) {
                Some((r, n)) if !r.trim().is_empty() && !n.trim().is_empty() => {
                    (r.trim(), n.trim())
                }
                _ => return Action::Unknown,
            };
            let Ok(qty) = n_str.parse::<u32>() else {
                return Action::Unknown;
            };
            let delta = if cmd_lower == "add" {
                qty as i32
            } else {
                -(qty as i32)
            };
            Action::ModResource {
                name: title_case(resource),
                delta,
            }
        }
        "spawn" => {
            let mut parts = rest.splitn(2, ' ');
            let species_str = match parts.next() {
                Some(s) if !s.is_empty() => s,
                _ => return Action::Unknown,
            };
            let name_raw = match parts.next() {
                Some(n) => n.trim().trim_matches('"').to_string(),
                None => return Action::Unknown,
            };
            if name_raw.is_empty() {
                return Action::Unknown;
            }
            match FishSpecies::parse(species_str) {
                Some(species) => Action::Spawn(species, name_raw),
                None => Action::Unknown,
            }
        }
        "buy" => parse_buy(rest),
        "sell" => parse_sell(rest),
        "give" => match parse_give_target(rest) {
            Some(target) => Action::Give(target),
            None => Action::Unknown,
        },
        "revive" => name_command(rest, Action::Revive),
        "kill" => name_command(rest, Action::Kill),
        "clone" => name_command(rest, Action::Clone),
        "bless" => Action::Bless,
        "expand" => name_command(rest, Action::Expand),
        "restore" => name_command(rest, Action::Restore),
        "program" => match parse_name_greedy(rest, fish_names) {
            Some(name) => Action::Program(name),
            None => Action::Unknown,
        },
        "console" => match parse_name_greedy(rest, fish_names) {
            Some(name) => Action::Console(name),
            None => Action::Unknown,
        },
        "print" => name_command(rest, Action::Print),
        "etch" => parse_etch(rest, fish_names).unwrap_or(Action::Unknown),
        "nudge" => parse_nudge(rest, fish_names).unwrap_or(Action::Unknown),
        "flip" => match parse_name_greedy(rest, fish_names) {
            Some(name) => Action::Flip(name),
            None => Action::Unknown,
        },
        "freeze" | "unfreeze" => match parse_name_greedy(rest, fish_names) {
            Some(name) => Action::SetFrozen {
                name,
                frozen: cmd_lower == "freeze",
            },
            None => Action::Unknown,
        },
        "fish" => {
            let flags: Vec<&str> = rest.split_whitespace().collect();
            Action::Fish {
                no_escape: flags.contains(&"--no-escape"),
                no_fight: flags.contains(&"--no-fight"),
                temper: fish_temper(&flags),
            }
        }
        "exit" => Action::Exit,
        "reset" => Action::Reset,
        "export" => name_command(rest, Action::Export),
        "import" => name_command(rest, Action::Import),
        "cowsay" => {
            let text = parse_raw_arg(rest);
            if text.is_empty() {
                Action::Unknown
            } else {
                Action::Cowsay(text)
            }
        }
        "say" => {
            let text = parse_raw_arg(rest);
            if text.is_empty() {
                Action::Unknown
            } else {
                Action::Say(text)
            }
        }
        "voidspawn" => Action::VoidSpawn,
        "startfishabduction" => Action::StartFishAbduction,
        "startcowabduction" => Action::StartCowAbduction,
        "startcallhome" => Action::StartCallHome,
        "startvoidwish" => {
            let flags: Vec<&str> = rest.split_whitespace().collect();
            Action::StartVoidWish {
                skip: flags.contains(&"--skip"),
            }
        }
        "fishtanks" => Action::Fishtanks,
        "ledger" => Action::Ledger,
        "circuit" => Action::Circuit,
        "foundry" => Action::Foundry,
        "inventory" => Action::Inventory,
        "shop" => Action::Shop,
        "names" => Action::ToggleNames,
        "nets" => Action::ToggleNets,
        "stats" => Action::ToggleStats,
        "zen" => Action::Zen,
        "cheat" => Action::Cheat,
        _ => Action::Unknown,
    }
}

impl Action {
    pub fn clearance(&self) -> Clearance {
        use Action::*;
        match self {
            Spawn(..) | Mutate(..) | Revive(_) | Kill(_) | Clone(_) | Bless | Expand(_)
            | Restore(_) => Clearance::God,
            ModResource { .. }
            | Give(_)
            | VoidSpawn
            | StartVoidWish { .. }
            | StartFishAbduction
            | StartCowAbduction
            | StartCallHome
            | Reset => Clearance::Debug,
            Fish {
                no_escape,
                no_fight,
                temper,
            } if *no_escape || *no_fight || temper.is_some() => Clearance::Debug,
            Feed
            | SetFps(_)
            | SetClock(_)
            | Show { .. }
            | Index { .. }
            | Fish { .. }
            | Inventory
            | Shop
            | Consume { .. }
            | ToggleNames
            | ToggleNets
            | ToggleStats
            | Zen
            | Switch(_)
            | Move { .. }
            | Fishtanks
            | Ledger
            | Circuit
            | Foundry
            | Exit
            | Export(_)
            | Import(_)
            | Cowsay(_)
            | Say(_)
            | Program(_)
            | Console(_)
            | Print(_)
            | Etch { .. }
            | SetFrozen { .. }
            | Nudge { .. }
            | Flip(_)
            | Buy(_)
            | Sell(_)
            | Cheat
            | ToggleDebugMode
            | Unknown => Clearance::Player,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_names() -> (&'static [&'static str], &'static [&'static str]) {
        (&[], &[])
    }

    #[test]
    fn parse_feed_is_the_whole_command() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/feed", fish, tanks), Action::Feed));
    }

    #[test]
    fn a_saved_script_s_feed_with_a_number_still_drops_one_portion() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/feed 5", fish, tanks), Action::Feed));
    }

    #[test]
    fn parse_feed_invalid_arg_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/feed abc", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_freeze_and_unfreeze_share_one_action() {
        let fish: &[&str] = &["Neo"];
        assert!(matches!(
            parse("/freeze \"Neo\"", fish, &[]),
            Action::SetFrozen { name, frozen: true } if name == "Neo"
        ));
        assert!(matches!(
            parse("/unfreeze Neo", fish, &[]),
            Action::SetFrozen { name, frozen: false } if name == "Neo"
        ));
    }

    #[test]
    fn parse_nudge_reads_both_offsets() {
        let fish: &[&str] = &["Neo"];
        assert!(matches!(
            parse("/nudge \"Neo\" 3 -2", fish, &[]),
            Action::Nudge { name, dx: 3, dy: -2 } if name == "Neo"
        ));
        assert!(matches!(
            parse("/nudge Neo 0 1", fish, &[]),
            Action::Nudge { dx: 0, dy: 1, .. }
        ));
    }

    #[test]
    fn parse_nudge_needs_two_whole_numbers() {
        let fish: &[&str] = &["Neo"];
        assert!(matches!(parse("/nudge Neo 3", fish, &[]), Action::Unknown));
        assert!(matches!(
            parse("/nudge Neo 3 up", fish, &[]),
            Action::Unknown
        ));
        assert!(matches!(parse("/nudge Neo", fish, &[]), Action::Unknown));
    }

    #[test]
    fn parse_flip_takes_a_known_fish() {
        let fish: &[&str] = &["Neo"];
        assert!(matches!(parse("/flip Neo", fish, &[]), Action::Flip(n) if n == "Neo"));
        assert!(matches!(parse("/flip Nobody", fish, &[]), Action::Unknown));
    }

    #[test]
    fn parse_freeze_needs_a_known_fish() {
        let fish: &[&str] = &["Neo"];
        assert!(matches!(
            parse("/freeze Nobody", fish, &[]),
            Action::Unknown
        ));
        assert!(matches!(parse("/freeze", fish, &[]), Action::Unknown));
    }

    #[test]
    fn parse_fps() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/fps 60", fish, tanks), Action::SetFps(v) if v == 60.0));
    }

    #[test]
    fn parse_fps_invalid_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/fps nope", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_spawn_quoted_name() {
        let (fish, tanks) = no_names();
        let action = parse("/spawn merluza \"Nemo\"", fish, tanks);
        assert!(
            matches!(&action, Action::Spawn(s, n) if *s == FishSpecies::Merluza && n == "Nemo")
        );
    }

    #[test]
    fn parse_spawn_unquoted_name() {
        let (fish, tanks) = no_names();
        let action = parse("/spawn salmon Bob", fish, tanks);
        assert!(matches!(&action, Action::Spawn(s, n) if *s == FishSpecies::Salmon && n == "Bob"));
    }

    #[test]
    fn parse_spawn_unknown_species_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/spawn unknownspecies Blob", fish, tanks),
            Action::Unknown
        ));
    }

    #[test]
    fn parse_spawn_missing_name_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/spawn merluza", fish, tanks),
            Action::Unknown
        ));
    }

    #[test]
    fn parse_exit() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/exit", fish, tanks), Action::Exit));
    }

    #[test]
    fn parse_fishtanks() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/fishtanks", fish, tanks),
            Action::Fishtanks
        ));
    }

    #[test]
    fn parse_inventory() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/inventory", fish, tanks),
            Action::Inventory
        ));
    }

    #[test]
    fn parse_shop() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/shop", fish, tanks), Action::Shop));
    }

    #[test]
    fn parse_names() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/names", fish, tanks), Action::ToggleNames));
    }

    #[test]
    fn parse_nets() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/nets", fish, tanks), Action::ToggleNets));
    }

    #[test]
    fn parse_stats() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/stats", fish, tanks), Action::ToggleStats));
        assert!(matches!(parse("/zen", fish, tanks), Action::Zen));
    }

    #[test]
    fn parse_voidspawn() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/voidspawn", fish, tanks),
            Action::VoidSpawn
        ));
    }

    #[test]
    fn parse_fish_no_flags() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/fish", fish, tanks),
            Action::Fish {
                no_escape: false,
                no_fight: false,
                temper: None
            }
        ));
    }

    #[test]
    fn parse_fish_no_escape_flag() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/fish --no-escape", fish, tanks),
            Action::Fish {
                no_escape: true,
                no_fight: false,
                temper: None
            }
        ));
    }

    #[test]
    fn parse_fish_no_fight_flag() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/fish --no-fight", fish, tanks),
            Action::Fish {
                no_escape: false,
                no_fight: true,
                temper: None
            }
        ));
    }

    #[test]
    fn parse_fish_both_flags() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/fish --no-escape --no-fight", fish, tanks),
            Action::Fish {
                no_escape: true,
                no_fight: true,
                temper: None
            }
        ));
    }

    #[test]
    fn parse_fish_temper_flags() {
        let (fish, tanks) = no_names();
        for (line, forced) in [
            ("/fish --normal", Temper::Normal),
            ("/fish --legendary", Temper::Legendary),
        ] {
            let action = parse(line, fish, tanks);
            assert!(
                matches!(action, Action::Fish { temper: Some(temper), .. } if temper == forced),
                "{line}"
            );
            assert_eq!(action.clearance(), Clearance::Debug, "{line}");
        }
    }

    #[test]
    fn parse_index_no_arg() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/index", fish, tanks),
            Action::Index {
                all: false,
                tank_filter: None
            }
        ));
    }

    #[test]
    fn parse_index_all() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/index all", fish, tanks),
            Action::Index {
                all: true,
                tank_filter: None
            }
        ));
    }

    #[test]
    fn parse_index_tank_filter() {
        let fish: &[&str] = &[];
        let tanks: &[&str] = &["Ocean"];
        assert!(matches!(
            parse("/index Ocean", fish, tanks),
            Action::Index { all: false, tank_filter: Some(ref n) } if n == "Ocean"
        ));
    }

    #[test]
    fn parse_show_unquoted() {
        let fish: &[&str] = &["Nemo"];
        let tanks: &[&str] = &[];
        assert!(
            matches!(parse("/show Nemo", fish, tanks), Action::Show { ref name, all: false } if name == "Nemo")
        );
    }

    #[test]
    fn parse_show_quoted() {
        let fish: &[&str] = &["Blue Tang"];
        let tanks: &[&str] = &[];
        assert!(matches!(
            parse("/show \"Blue Tang\"", fish, tanks),
            Action::Show { ref name, all: false } if name == "Blue Tang"
        ));
    }

    #[test]
    fn parse_show_with_all_flag() {
        let fish: &[&str] = &["Nemo"];
        let tanks: &[&str] = &[];
        assert!(matches!(
            parse("/show Nemo all", fish, tanks),
            Action::Show { ref name, all: true } if name == "Nemo"
        ));
    }

    #[test]
    fn parse_show_no_arg_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/show", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_switch_known_tank() {
        let fish: &[&str] = &[];
        let tanks: &[&str] = &["Ocean"];
        assert!(matches!(
            parse("/switch Ocean", fish, tanks),
            Action::Switch(ref n) if n == "Ocean"
        ));
    }

    #[test]
    fn parse_switch_no_arg_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/switch", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_move_known_fish_and_tank() {
        let fish: &[&str] = &["Nemo"];
        let tanks: &[&str] = &["Ocean"];
        assert!(matches!(
            parse("/move Nemo Ocean", fish, tanks),
            Action::Move { ref fish, ref tank } if fish == "Nemo" && tank == "Ocean"
        ));
    }

    #[test]
    fn parse_move_quoted_args() {
        let fish: &[&str] = &["Blue Tang"];
        let tanks: &[&str] = &["Coral Reef"];
        assert!(matches!(
            parse("/move \"Blue Tang\" \"Coral Reef\"", fish, tanks),
            Action::Move { ref fish, ref tank } if fish == "Blue Tang" && tank == "Coral Reef"
        ));
    }

    #[test]
    fn parse_mutate_unquoted() {
        let fish: &[&str] = &["Nemo"];
        let tanks: &[&str] = &[];
        assert!(matches!(
            parse("/mutate Nemo eye+", fish, tanks),
            Action::Mutate(ref n, ref m) if n == "Nemo" && m == "eye+"
        ));
    }

    #[test]
    fn parse_mutate_quoted_name() {
        let fish: &[&str] = &["Blue Tang"];
        let tanks: &[&str] = &[];
        assert!(matches!(
            parse("/mutate \"Blue Tang\" size+", fish, tanks),
            Action::Mutate(ref n, ref m) if n == "Blue Tang" && m == "size+"
        ));
    }

    #[test]
    fn parse_add_resource() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/add food 10", fish, tanks),
            Action::ModResource { ref name, delta: 10 } if name == "Food"
        ));
    }

    #[test]
    fn parse_subtract_resource() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/subtract cash 5", fish, tanks),
            Action::ModResource { ref name, delta: -5 } if name == "Cash"
        ));
    }

    #[test]
    fn parse_consume_unquoted() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/consume coffee", fish, tanks),
            Action::Consume { ref name } if name == "coffee"
        ));
    }

    #[test]
    fn parse_unknown_command() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/glorp", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_no_slash_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("feed 5", fish, tanks), Action::Unknown));
    }

    #[test]
    fn autocomplete_partial_command_ghost() {
        let result = autocomplete("/fe", &CompletionCtx::default());
        let c = result.unwrap();
        assert!(c.ghost.starts_with("ed"));
    }

    #[test]
    fn autocomplete_exact_command_with_args_ghost() {
        let result = autocomplete("/fps", &CompletionCtx::default());
        let c = result.unwrap();
        assert_eq!(c.ghost, " <n>");
    }

    #[test]
    fn feed_is_offered_whole_and_never_with_an_amount() {
        let player = CompletionCtx::default();
        assert!(
            autocomplete("/feed", &player).is_none(),
            "nothing follows /feed"
        );
        assert!(autocomplete("/feed ", &player).is_none());
        let ghost = autocomplete("/fee", &player).expect("a ghost").ghost;
        assert_eq!(ghost, "d", "the ghost is the command alone");
    }

    #[test]
    fn a_locked_command_is_neither_offered_nor_completed() {
        let player = CompletionCtx::default();
        assert!(autocomplete("/spaw", &player).is_none());
        assert!(autocomplete("/give ca", &player).is_none());
        let god = CompletionCtx {
            clearance: Clearance::God,
            ..Default::default()
        };
        assert!(autocomplete("/spaw", &god).is_some());
        assert!(autocomplete("/give ca", &god).is_none());
    }

    #[test]
    fn every_command_word_is_cleared_like_the_action_it_parses_to() {
        let fish: &[&str] = &["Nemo"];
        let tanks: &[&str] = &["Fishtank"];
        let samples: &[(&str, &str)] = &[
            ("add", "cash 5"),
            ("bless", ""),
            ("clone", "Nemo"),
            ("expand", "Fishtank"),
            ("give", "cash"),
            ("kill", "Nemo"),
            ("mutate", "Nemo eyeincrease"),
            ("restore", "Nemo"),
            ("revive", "Nemo"),
            ("spawn", "merluza \"Dory\""),
            ("subtract", "cash 5"),
            ("export", "\"fishtank.ron\""),
            ("import", "\"fishtank.ron\""),
            ("cheat", ""),
            ("feed", ""),
            ("shop", ""),
        ];
        for &(name, clearance) in COMMAND_NAMES {
            let args = samples
                .iter()
                .find(|(word, _)| *word == name)
                .map(|&(_, args)| args)
                .unwrap_or_default();
            let action = parse(&format!("/{name} {args}"), fish, tanks);
            if matches!(action, Action::Unknown) {
                continue;
            }
            assert_eq!(action.clearance(), clearance, "/{name}");
        }
    }

    #[test]
    fn the_fishing_cheats_are_debug_flags() {
        assert_eq!(
            parse("/fish --no-escape", &[], &[]).clearance(),
            Clearance::Debug
        );
        assert_eq!(parse("/fish", &[], &[]).clearance(), Clearance::Player);
    }

    #[test]
    fn debug_mode_is_typed_with_a_bang_whatever_its_case() {
        assert!(matches!(
            parse("  !DebugMode ", &[], &[]),
            Action::ToggleDebugMode
        ));
    }

    #[test]
    fn autocomplete_exact_command_tab_result_adds_space() {
        let result = autocomplete("/fps", &CompletionCtx::default());
        let c = result.unwrap();
        assert_eq!(c.tab_result, Some("/fps ".to_string()));
    }

    #[test]
    fn autocomplete_spawn_partial_species_ghost() {
        let result = autocomplete(
            "/spawn mer",
            &CompletionCtx {
                clearance: Clearance::Debug,
                ..Default::default()
            },
        );
        let c = result.unwrap();
        assert!(c.ghost.contains("luza"));
    }

    #[test]
    fn autocomplete_unknown_prefix_returns_none() {
        let result = autocomplete("/zzz", &CompletionCtx::default());
        assert!(result.is_none());
    }

    #[test]
    fn autocomplete_fish_name_ghost_in_mutate() {
        let result = autocomplete(
            "/mutate Ne",
            &CompletionCtx {
                fish_names: &["Nemo"],
                clearance: Clearance::God,
                ..Default::default()
            },
        );
        let c = result.unwrap();
        assert!(c.ghost.contains("mo"));
    }

    #[test]
    fn autocomplete_mutation_name_ghost() {
        let result = autocomplete(
            "/mutate Nemo eye",
            &CompletionCtx {
                fish_names: &["Nemo"],
                clearance: Clearance::God,
                ..Default::default()
            },
        );
        let c = result.unwrap();
        assert!(!c.ghost.is_empty());
    }

    #[test]
    fn autocomplete_show_fish_name() {
        let result = autocomplete(
            "/show Ne",
            &CompletionCtx {
                fish_in_tanks: &[("Nemo", "Tank1")],
                ..Default::default()
            },
        );
        let c = result.unwrap();
        assert!(c.ghost.contains("mo"));
    }

    #[test]
    fn autocomplete_no_input_returns_none() {
        let result = autocomplete("", &CompletionCtx::default());
        assert!(result.is_none());
    }

    #[test]
    fn autocomplete_cowsay_hidden_when_no_cow() {
        let result = autocomplete("/cow", &CompletionCtx::default());
        assert!(result.is_none());
    }

    #[test]
    fn autocomplete_cowsay_visible_when_cow_present() {
        let result = autocomplete(
            "/cow",
            &CompletionCtx {
                has_cow_in_current: true,
                ..Default::default()
            },
        );
        assert!(result.is_some());
    }

    #[test]
    fn parse_startvoidwish_no_flags() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/startvoidwish", fish, tanks),
            Action::StartVoidWish { skip: false }
        ));
    }

    #[test]
    fn parse_startvoidwish_skip_flag() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/startvoidwish --skip", fish, tanks),
            Action::StartVoidWish { skip: true }
        ));
    }

    #[test]
    fn parse_revive_takes_unquoted_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/revive Nemo", fish, tanks), Action::Revive(n) if n == "Nemo"));
    }

    #[test]
    fn parse_revive_takes_quoted_multiword_name() {
        let (fish, tanks) = no_names();
        assert!(
            matches!(parse("/revive \"Sir Bubbles\"", fish, tanks), Action::Revive(n) if n == "Sir Bubbles")
        );
    }

    #[test]
    fn parse_revive_without_name_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/revive", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_clone_bless_restore_capture_the_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/clone Bob", fish, tanks), Action::Clone(n) if n == "Bob"));
        assert!(matches!(parse("/bless", fish, tanks), Action::Bless));
        assert!(matches!(parse("/restore Bob", fish, tanks), Action::Restore(n) if n == "Bob"));
    }

    #[test]
    fn parse_expand_captures_tank_name() {
        let (fish, tanks) = no_names();
        assert!(
            matches!(parse("/expand Helltank", fish, tanks), Action::Expand(n) if n == "Helltank")
        );
    }

    #[test]
    fn parse_give_cash_maps_to_give_target() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/give cash", fish, tanks),
            Action::Give(GiveTarget::Cash)
        ));
    }

    #[test]
    fn parse_give_tank_resolves_kind() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/give alientank", fish, tanks),
            Action::Give(GiveTarget::Tank(TankKind::Alien))
        ));
    }

    #[test]
    fn parse_give_unknown_thing_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/give nonsense", fish, tanks),
            Action::Unknown
        ));
    }

    #[test]
    fn parse_buy_fish_by_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/buy salmon", fish, tanks),
            Action::Buy(BuyTarget::Fish(FishSpecies::Salmon))
        ));
    }

    #[test]
    fn parse_buy_tank_by_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/buy helltank", fish, tanks),
            Action::Buy(BuyTarget::Tank(TankKind::Hell))
        ));
    }

    #[test]
    fn parse_buy_food_default_qty() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/buy food", fish, tanks),
            Action::Buy(BuyTarget::Food { qty: 1 })
        ));
    }

    #[test]
    fn parse_buy_food_with_qty() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/buy food 5", fish, tanks),
            Action::Buy(BuyTarget::Food { qty: 5 })
        ));
    }

    #[test]
    fn parse_buy_coffee_with_qty() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/buy coffee 3", fish, tanks),
            Action::Buy(BuyTarget::Coffee { qty: 3 })
        ));
    }

    #[test]
    fn parse_buy_bait_with_qty() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/buy bait 10", fish, tanks),
            Action::Buy(BuyTarget::Bait { qty: 10 })
        ));
    }

    #[test]
    fn parse_buy_unknown_item_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/buy nonsense", fish, tanks),
            Action::Unknown
        ));
    }

    #[test]
    fn parse_buy_no_arg_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/buy", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_sell_fish_by_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell fish Nemo", fish, tanks),
            Action::Sell(SellTarget::Fish(ref n)) if n == "Nemo"
        ));
    }

    #[test]
    fn parse_sell_tank_by_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell Tank \"Ocean\"", fish, tanks),
            Action::Sell(SellTarget::Tank(ref n)) if n == "Ocean"
        ));
    }

    #[test]
    fn parse_sell_blueprint_by_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell blueprint \"Ring Clock\"", fish, tanks),
            Action::Sell(SellTarget::Blueprint(ref n)) if n == "Ring Clock"
        ));
    }

    #[test]
    fn parse_sell_never_guesses_the_kind_of_a_named_thing() {
        let fish: &[&str] = &["Nemo"];
        let tanks: &[&str] = &["Nemo"];
        assert!(matches!(parse("/sell Nemo", fish, tanks), Action::Unknown));
        assert!(matches!(
            parse("/sell \"Nemo\"", fish, tanks),
            Action::Unknown
        ));
    }

    #[test]
    fn parse_sell_a_kind_with_no_name_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/sell fish", fish, tanks), Action::Unknown));
        assert!(matches!(
            parse("/sell blueprint \"\"", fish, tanks),
            Action::Unknown
        ));
    }

    #[test]
    fn parse_sell_junk_default_qty() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell junk", fish, tanks),
            Action::Sell(SellTarget::Junk { qty: 1 })
        ));
    }

    #[test]
    fn parse_sell_junk_with_qty() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell junk 5", fish, tanks),
            Action::Sell(SellTarget::Junk { qty: 5 })
        ));
    }

    #[test]
    fn parse_sell_milk_multiword() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell chocolate milk", fish, tanks),
            Action::Sell(SellTarget::Milk {
                variant: MilkVariant::Chocolate,
                qty: 1,
            })
        ));
    }

    #[test]
    fn parse_sell_milk_multiword_with_qty() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell strawberry milk 3", fish, tanks),
            Action::Sell(SellTarget::Milk {
                variant: MilkVariant::Strawberry,
                qty: 3,
            })
        ));
    }

    #[test]
    fn parse_sell_a_tank_seed_by_its_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell necronomicon 2", fish, tanks),
            Action::Sell(SellTarget::Seed {
                kind: ConsumableKind::Necronomicon,
                qty: 2,
            })
        ));
    }

    #[test]
    fn every_tank_seed_that_has_a_name_sells_by_it_and_the_nameless_one_cannot_be_typed() {
        let (fish, tanks) = no_names();
        for seed in ConsumableKind::seeds() {
            let name = seed.lowercase_name();
            let line = format!("/sell {name}");
            if name.trim().is_empty() {
                assert!(
                    matches!(parse(&line, fish, tanks), Action::Unknown),
                    "a nameless item is found and sold from the menu, never typed"
                );
                continue;
            }
            let Action::Sell(SellTarget::Seed { kind, qty }) = parse(&line, fish, tanks) else {
                panic!("{line} did not reach the seed it names");
            };
            assert!(kind == seed, "{line} reached the wrong seed");
            assert_eq!(qty, 1);
        }
    }

    #[test]
    fn parse_sell_robotics_stock_by_its_spaced_name() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell blank wafer 5", fish, tanks),
            Action::Sell(SellTarget::Robotics {
                kind: ConsumableKind::BlankWafer,
                qty: 5,
            })
        ));
        assert!(matches!(
            parse("/sell blank circuit blueprint", fish, tanks),
            Action::Sell(SellTarget::Robotics {
                kind: ConsumableKind::BlankBlueprint,
                qty: 1,
            })
        ));
    }

    #[test]
    fn parse_sell_never_reaches_stock_that_is_on_no_shelf() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/sell food", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_foundry() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/foundry", fish, tanks), Action::Foundry));
    }

    #[test]
    fn parse_sell_no_arg_is_unknown() {
        let (fish, tanks) = no_names();
        assert!(matches!(parse("/sell", fish, tanks), Action::Unknown));
    }

    #[test]
    fn parse_sell_fish_quoted_multiword() {
        let (fish, tanks) = no_names();
        assert!(matches!(
            parse("/sell fish \"Blue Tang\"", fish, tanks),
            Action::Sell(SellTarget::Fish(ref n)) if n == "Blue Tang"
        ));
        assert!(matches!(
            parse("/sell fish Seed2's Clone", fish, tanks),
            Action::Sell(SellTarget::Fish(ref n)) if n == "Seed2's Clone"
        ));
    }

    #[test]
    fn autocomplete_buy_partial_name_ghost() {
        let result = autocomplete("/buy sa", &CompletionCtx::default());
        let c = result.unwrap();
        assert!(c.ghost.contains("lmon"));
    }

    #[test]
    fn autocomplete_buy_food_shows_qty_ghost() {
        let result = autocomplete("/buy food", &CompletionCtx::default());
        let c = result.unwrap();
        assert!(c.ghost.contains("<quantity>"));
    }

    #[test]
    fn autocomplete_buy_coffee_shows_qty_ghost() {
        let result = autocomplete("/buy coffee", &CompletionCtx::default());
        let c = result.unwrap();
        assert!(c.ghost.contains("<quantity>"));
    }

    #[test]
    fn autocomplete_buy_unique_no_qty_ghost() {
        let result = autocomplete("/buy salmon", &CompletionCtx::default());
        assert!(result.is_none());
    }

    #[test]
    fn autocomplete_sell_empty_shows_item_ghost() {
        let result = autocomplete(
            "/sell ",
            &CompletionCtx {
                sellable_stackable_names: &["Junk".to_string()],
                ..Default::default()
            },
        );
        let c = result.unwrap();
        assert_eq!(c.ghost, "<item>");
    }

    #[test]
    fn autocomplete_sell_stackable_exact_shows_qty_ghost() {
        let result = autocomplete(
            "/sell Junk",
            &CompletionCtx {
                sellable_stackable_names: &["Junk".to_string()],
                ..Default::default()
            },
        );
        let c = result.unwrap();
        assert!(c.ghost.contains("<quantity>"));
    }

    #[test]
    fn autocomplete_sell_offers_the_kind_before_the_name() {
        let fish = ["Nemo".to_string()];
        let ctx = CompletionCtx {
            sellable_fish_names: &fish,
            ..Default::default()
        };

        let kind = autocomplete("/sell fi", &ctx).unwrap();
        assert_eq!(kind.ghost, "sh <name>");
        assert_eq!(kind.tab_result.as_deref(), Some("/sell fish "));
        assert!(
            autocomplete("/sell Ne", &ctx).is_none(),
            "a bare name is not a sale"
        );

        let name = autocomplete("/sell fish Ne", &ctx).unwrap();
        assert_eq!(name.ghost, "mo");
        assert_eq!(name.tab_result.as_deref(), Some("/sell fish Nemo"));
    }

    #[test]
    fn autocomplete_sell_only_offers_kinds_the_player_can_sell() {
        let blueprints = ["Ring Clock"];
        let ctx = CompletionCtx {
            blueprint_names: &blueprints,
            ..Default::default()
        };

        assert!(autocomplete("/sell ti", &ctx).is_none());
        let quoted = autocomplete("/sell blueprint \"Ri", &ctx).unwrap();
        assert_eq!(quoted.ghost, "ng Clock\"");
        assert_eq!(
            quoted.tab_result.as_deref(),
            Some("/sell blueprint \"Ring Clock\"")
        );
    }

    #[test]
    fn console_takes_the_name_of_the_fish_it_hands_the_keyboard_to() {
        let fish = ["Pad"];
        assert!(matches!(
            parse("/console \"Pad\"", &fish, &[]),
            Action::Console(name) if name == "Pad"
        ));
        assert!(matches!(
            parse("/console pad", &fish, &[]),
            Action::Console(name) if name == "Pad"
        ));
        assert!(matches!(parse("/console", &fish, &[]), Action::Unknown));
    }

    #[test]
    fn autocomplete_console_offers_only_fish_that_bind_a_key() {
        let pads = ["Pad"];
        let ctx = CompletionCtx {
            console_names: &pads,
            programmable_names: &["Pad", "Neo"],
            ..Default::default()
        };
        assert_eq!(autocomplete("/console", &ctx).unwrap().ghost, " <name>");
        let named = autocomplete("/console P", &ctx).unwrap();
        assert_eq!(named.tab_result.as_deref(), Some("/console Pad"));
        assert!(autocomplete("/console N", &ctx).is_none());
    }

    #[test]
    fn autocomplete_sell_no_items_returns_none() {
        let result = autocomplete("/sell ", &CompletionCtx::default());
        assert!(result.is_none());
    }
}
