use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

use super::bestiary::Rank;
use super::floor::{CHEST_SHARES, CROWD};

pub const LAYERS: usize = 5;
pub const LANES: u8 = 5;
pub const ENTRY_LANE: u8 = 2;
pub const ENTRY_GLYPH: char = '▲';
pub const FIRST_ROW: [u8; 3] = [0, 2, 4];
pub const MIN_ROW: usize = 3;
pub const MAX_ROW: usize = 5;
pub const CUT_PERCENT: u32 = 55;
const PERCENT: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomKind {
    Encounter,
    Den,
    Chest,
    BellJar,
    Event,
    School,
    Shrine,
    Altar,
    Laboratory,
    Vent,
    Cleaning,
    Vending,
    Wreck,
    Boss,
    Elevator,
}

impl RoomKind {
    pub const ALL: [RoomKind; 15] = [
        RoomKind::Encounter,
        RoomKind::Den,
        RoomKind::Chest,
        RoomKind::BellJar,
        RoomKind::Event,
        RoomKind::School,
        RoomKind::Shrine,
        RoomKind::Altar,
        RoomKind::Laboratory,
        RoomKind::Vent,
        RoomKind::Cleaning,
        RoomKind::Vending,
        RoomKind::Wreck,
        RoomKind::Boss,
        RoomKind::Elevator,
    ];

    pub fn glyph(self) -> char {
        match self {
            RoomKind::Encounter => '»',
            RoomKind::Den => 'ʘ',
            RoomKind::Chest => '$',
            RoomKind::BellJar => '#',
            RoomKind::Event => '?',
            RoomKind::School => 'S',
            RoomKind::Shrine => '+',
            RoomKind::Altar => '@',
            RoomKind::Laboratory => '*',
            RoomKind::Vent => '~',
            RoomKind::Cleaning => 'H',
            RoomKind::Vending => '¤',
            RoomKind::Wreck => '&',
            RoomKind::Boss => 'ω',
            RoomKind::Elevator => '▼',
        }
    }

    pub fn legend(self) -> &'static str {
        match self {
            RoomKind::Encounter => "encounter",
            RoomKind::Den => "den",
            RoomKind::Chest => "chest",
            RoomKind::BellJar => "bell jar",
            RoomKind::Event => "event",
            RoomKind::School => "school",
            RoomKind::Shrine => "shrine",
            RoomKind::Altar => "altar",
            RoomKind::Laboratory => "laboratory",
            RoomKind::Vent => "vent",
            RoomKind::Cleaning => "cleaners",
            RoomKind::Vending => "vending",
            RoomKind::Wreck => "wreck",
            RoomKind::Boss => "boss",
            RoomKind::Elevator => "elevator",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            RoomKind::Encounter => "Encounter",
            RoomKind::Den => "Den",
            RoomKind::Chest => "Sunken Chest",
            RoomKind::BellJar => "Bell Jar",
            RoomKind::Event => "Event",
            RoomKind::School => "Drifting School",
            RoomKind::Shrine => "Shrine",
            RoomKind::Altar => "Altar",
            RoomKind::Laboratory => "Laboratory",
            RoomKind::Vent => "Hydrothermal Vent",
            RoomKind::Cleaning => "Cleaning Station",
            RoomKind::Vending => "Vending Machine",
            RoomKind::Wreck => "Wreck",
            RoomKind::Boss => "Lair",
            RoomKind::Elevator => "Elevator",
        }
    }

    pub fn weight(self) -> u32 {
        match self {
            RoomKind::Encounter => 41,
            RoomKind::Den => 8,
            RoomKind::Chest => 12,
            RoomKind::BellJar => 4,
            RoomKind::Event => 10,
            RoomKind::School => 3,
            RoomKind::Shrine => 4,
            RoomKind::Altar => 3,
            RoomKind::Laboratory => 4,
            RoomKind::Vent => 5,
            RoomKind::Cleaning => 3,
            RoomKind::Vending => 3,
            RoomKind::Wreck | RoomKind::Boss | RoomKind::Elevator => 0,
        }
    }

    pub fn is_service(self) -> bool {
        matches!(
            self,
            RoomKind::School
                | RoomKind::Shrine
                | RoomKind::Altar
                | RoomKind::Laboratory
                | RoomKind::Vent
                | RoomKind::Cleaning
                | RoomKind::Vending
        )
    }

    pub fn shares(self) -> u64 {
        let (least, most) = CROWD;
        match self {
            RoomKind::Encounter => (u64::from(least) + u64::from(most)) / 2 * Rank::Common.shares(),
            RoomKind::Den => Rank::Elite.shares(),
            RoomKind::Chest => CHEST_SHARES,
            RoomKind::Boss => Rank::Boss.shares(),
            _ => 0,
        }
    }
}

pub fn expected_shares() -> u64 {
    let total: u32 = RoomKind::ALL.iter().map(|kind| kind.weight()).sum();
    let rooms: u64 = RoomKind::ALL
        .iter()
        .map(|kind| u64::from(kind.weight()) * kind.shares())
        .sum::<u64>()
        * LAYERS as u64;
    (rooms + u64::from(total) / 2) / u64::from(total) + RoomKind::Boss.shares()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    pub kind: RoomKind,
    pub lane: u8,
    pub seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bus {
    pub low: u8,
    pub high: u8,
}

impl Bus {
    pub fn holds(self, lane: u8) -> bool {
        (self.low..=self.high).contains(&lane)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub wreck: bool,
    pub events: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    pub rows: Vec<Vec<Room>>,
    pub entry: Vec<Bus>,
    pub buses: Vec<Vec<Bus>>,
}

impl Map {
    pub fn generate(seed: u64, shape: Shape) -> Map {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut lanes: Vec<Vec<u8>> = vec![FIRST_ROW.to_vec()];
        for _ in 1..LAYERS {
            lanes.push(row_lanes(&mut rng));
        }
        lanes.push(vec![ENTRY_LANE]);
        let entry = buses(&[ENTRY_LANE], &lanes[0], &mut rng);
        let buses: Vec<Vec<Bus>> = lanes
            .windows(2)
            .map(|pair| buses(&pair[0], &pair[1], &mut rng))
            .collect();
        let mut rows: Vec<Vec<Room>> = lanes
            .iter()
            .enumerate()
            .map(|(row, lanes)| {
                lanes
                    .iter()
                    .map(|&lane| Room {
                        kind: match row == LAYERS {
                            true => RoomKind::Boss,
                            false => kind(row, shape, &mut rng),
                        },
                        lane,
                        seed: rng.random(),
                    })
                    .collect()
            })
            .collect();
        if shape.wreck {
            let row = rng.random_range(1..LAYERS);
            let at = rng.random_range(0..rows[row].len());
            rows[row][at].kind = RoomKind::Wreck;
        }
        Map { rows, entry, buses }
    }

    pub fn boss_row(&self) -> usize {
        self.rows.len() - 1
    }

    pub fn room(&self, row: usize, index: usize) -> Option<Room> {
        self.rows.get(row)?.get(index).copied()
    }

    fn links(&self, row: Option<usize>) -> &[Bus] {
        match row {
            None => &self.entry,
            Some(row) => self.buses.get(row).map_or(&[], Vec::as_slice),
        }
    }

    pub fn linked(&self, from: Option<(usize, u8)>, to_lane: u8) -> bool {
        let lane = from.map_or(ENTRY_LANE, |(_, lane)| lane);
        self.links(from.map(|(row, _)| row))
            .iter()
            .any(|bus| bus.holds(lane) && bus.holds(to_lane))
    }

    pub fn next(&self, path: &[u8]) -> Vec<u8> {
        let row = path.len();
        let Some(rooms) = self.rows.get(row) else {
            return Vec::new();
        };
        let from = path
            .last()
            .and_then(|&index| self.room(row - 1, usize::from(index)))
            .map(|room| (row - 1, room.lane));
        (0..rooms.len() as u8)
            .filter(|&index| self.linked(from, rooms[usize::from(index)].lane))
            .collect()
    }
}

fn row_lanes(rng: &mut SmallRng) -> Vec<u8> {
    let count = rng.random_range(MIN_ROW..=MAX_ROW);
    let mut lanes: Vec<u8> = (0..LANES).collect();
    while lanes.len() > count {
        lanes.remove(rng.random_range(0..lanes.len()));
    }
    lanes
}

fn buses(parents: &[u8], children: &[u8], rng: &mut SmallRng) -> Vec<Bus> {
    let mut lanes: Vec<u8> = parents.iter().chain(children).copied().collect();
    lanes.sort_unstable();
    lanes.dedup();
    let count =
        |from: u8, to: u8, side: &[u8]| side.iter().filter(|&&l| l >= from && l <= to).count();
    let mut buses = Vec::new();
    let mut low = lanes[0];
    for pair in lanes.windows(2) {
        let (here, there) = (pair[0], pair[1]);
        let last = *lanes.last().unwrap_or(&there);
        let left_whole = count(low, here, parents) > 0 && count(low, here, children) > 0;
        let right_whole = count(there, last, parents) > 0 && count(there, last, children) > 0;
        if left_whole && right_whole && rng.random_range(0..PERCENT) < CUT_PERCENT {
            buses.push(Bus { low, high: here });
            low = there;
        }
    }
    buses.push(Bus {
        low,
        high: *lanes.last().unwrap_or(&low),
    });
    buses
}

fn kind(row: usize, shape: Shape, rng: &mut SmallRng) -> RoomKind {
    let allowed = |kind: &RoomKind| {
        kind.weight() > 0
            && (shape.events || *kind != RoomKind::Event)
            && (row > 0 || !kind.is_service())
    };
    let pool: Vec<RoomKind> = RoomKind::ALL.into_iter().filter(allowed).collect();
    let total: u32 = pool.iter().map(|kind| kind.weight()).sum();
    let mut roll = rng.random_range(0..total);
    for kind in &pool {
        if roll < kind.weight() {
            return *kind;
        }
        roll -= kind.weight();
    }
    RoomKind::Encounter
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEEDS: u64 = 300;
    const PLAIN: Shape = Shape {
        wreck: false,
        events: true,
    };

    fn every_map() -> impl Iterator<Item = Map> {
        (0..SEEDS).map(|seed| Map::generate(seed, PLAIN))
    }

    #[test]
    fn a_map_is_its_seed() {
        assert_eq!(Map::generate(7, PLAIN), Map::generate(7, PLAIN));
        assert_ne!(Map::generate(7, PLAIN), Map::generate(8, PLAIN));
    }

    #[test]
    fn a_floor_opens_on_three_passages_and_ends_at_its_boss() {
        for map in every_map() {
            assert_eq!(map.rows.len(), LAYERS + 1);
            let lanes: Vec<u8> = map.rows[0].iter().map(|room| room.lane).collect();
            assert_eq!(lanes, FIRST_ROW);
            assert_eq!(map.rows[map.boss_row()].len(), 1);
            assert_eq!(map.rows[map.boss_row()][0].kind, RoomKind::Boss);
            assert_eq!(map.next(&[]).len(), FIRST_ROW.len());
        }
    }

    #[test]
    fn every_room_is_reached_and_every_room_leads_on() {
        for map in every_map() {
            for row in 0..map.boss_row() {
                for (index, room) in map.rows[row].iter().enumerate() {
                    let path: Vec<u8> = (0..=row).map(|_| index as u8).collect();
                    let ahead = map.next(&path[..row + 1]);
                    assert!(
                        !ahead.is_empty(),
                        "row {row} lane {} is a dead end",
                        room.lane
                    );
                }
                for child in &map.rows[row + 1] {
                    let fed = map.rows[row]
                        .iter()
                        .any(|parent| map.linked(Some((row, parent.lane)), child.lane));
                    assert!(fed, "row {} lane {} is never reached", row + 1, child.lane);
                }
            }
        }
    }

    #[test]
    fn a_bus_never_crosses_another() {
        for map in every_map() {
            for buses in std::iter::once(&map.entry).chain(&map.buses) {
                for pair in buses.windows(2) {
                    assert!(pair[0].high < pair[1].low);
                }
            }
        }
    }

    #[test]
    fn the_first_rooms_are_never_services_and_a_wreck_lies_inside_the_floor() {
        for seed in 0..SEEDS {
            let map = Map::generate(
                seed,
                Shape {
                    wreck: true,
                    events: false,
                },
            );
            assert!(map.rows[0].iter().all(|room| !room.kind.is_service()));
            let wrecks: Vec<usize> = (0..map.rows.len())
                .filter(|&row| map.rows[row].iter().any(|r| r.kind == RoomKind::Wreck))
                .collect();
            assert_eq!(wrecks.len(), 1);
            assert!(wrecks[0] > 0 && wrecks[0] < map.boss_row());
            let events = map
                .rows
                .iter()
                .flatten()
                .filter(|r| r.kind == RoomKind::Event);
            assert_eq!(events.count(), 0);
        }
    }
}
