use ratatui::style::Color;

use crate::sprite::Band;

pub const EYE_PAINT: char = 'e';
pub const JUNK_PAINT: char = 'j';
const PALETTE_RADIX: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    Palette(usize),
    Eye,
    Junk,
}

impl Paint {
    fn of(letter: char) -> Option<Paint> {
        match letter {
            EYE_PAINT => Some(Paint::Eye),
            JUNK_PAINT => Some(Paint::Junk),
            digit => digit
                .to_digit(PALETTE_RADIX)
                .map(|index| Paint::Palette(index as usize)),
        }
    }

    pub fn color(self, palette: &[Color], shift: usize, eye: Color) -> Color {
        match self {
            Paint::Palette(index) => palette[(index + shift) % palette.len()],
            Paint::Eye => eye,
            Paint::Junk => palette[shift % palette.len()],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Art {
    pub lines: &'static [&'static str],
    pub paint: &'static [&'static str],
    pub body_row: usize,
}

impl Art {
    pub fn width(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
    }

    pub fn leaves_free(&self, band: Band) -> bool {
        let body = self.lines[self.body_row].trim();
        if body.contains(' ') {
            return false;
        }
        match band {
            Band::Top => self.body_row == 0,
            Band::Bottom => self.body_row + 1 == self.lines.len(),
        }
    }

    pub fn cells(&self) -> Vec<Vec<Option<(char, Paint)>>> {
        let width = self.width();
        self.lines
            .iter()
            .zip(self.paint.iter())
            .map(|(line, paint)| {
                let mut row: Vec<Option<(char, Paint)>> = line
                    .chars()
                    .zip(paint.chars())
                    .map(|(glyph, letter)| {
                        if glyph == ' ' {
                            return None;
                        }
                        Paint::of(letter).map(|paint| (glyph, paint))
                    })
                    .collect();
                row.resize(width, None);
                row
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Figure {
    pub left: &'static [Art],
    pub right: &'static [Art],
    pub zooming: Option<(Art, Art)>,
}

impl Figure {
    pub fn pose(&self, facing_left: bool, wave: bool, zooming: bool) -> Art {
        if zooming && let Some((left, right)) = self.zooming {
            return if facing_left { left } else { right };
        }
        let frames = if facing_left { self.left } else { self.right };
        let frame = usize::from(wave) % frames.len();
        frames[frame]
    }

    pub fn arts(&self) -> impl Iterator<Item = Art> + '_ {
        self.left.iter().chain(self.right.iter()).copied().chain(
            self.zooming
                .into_iter()
                .flat_map(|(left, right)| [left, right]),
        )
    }

    pub fn width(&self) -> usize {
        self.arts().map(|art| art.width()).max().unwrap_or(0)
    }

    pub fn leaves_free(&self, band: Band) -> bool {
        self.arts().all(|art| art.leaves_free(band))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fishes::species::{ALL_SPECIES, BodyTemplate};

    #[test]
    fn every_figure_paints_each_glyph_it_draws() {
        for &species in ALL_SPECIES {
            let BodyTemplate::Figure(figure) = species.config().body else {
                continue;
            };
            for art in figure.arts() {
                assert_eq!(
                    art.lines.len(),
                    art.paint.len(),
                    "{}",
                    species.display_name()
                );
                assert!(art.body_row < art.lines.len(), "{}", species.display_name());
                for (line, paint) in art.lines.iter().zip(art.paint.iter()) {
                    assert_eq!(
                        line.chars().count(),
                        paint.chars().count(),
                        "{}: {line:?} is painted by {paint:?}",
                        species.display_name()
                    );
                    for (glyph, letter) in line.chars().zip(paint.chars()) {
                        assert!(
                            glyph == ' ' || Paint::of(letter).is_some(),
                            "{}: {glyph:?} has no paint in {paint:?}",
                            species.display_name()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_figure_faces_both_ways_in_every_pose() {
        for &species in ALL_SPECIES {
            let BodyTemplate::Figure(figure) = species.config().body else {
                continue;
            };
            assert_eq!(
                figure.left.len(),
                figure.right.len(),
                "{}",
                species.display_name()
            );
            for (left, right) in figure.left.iter().zip(figure.right.iter()) {
                assert_eq!(left.width(), right.width(), "{}", species.display_name());
                assert_eq!(left.body_row, right.body_row, "{}", species.display_name());
            }
        }
    }
}
