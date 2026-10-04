use serde::{Deserialize, Serialize};

use super::{App, Overlay};

pub const LESSON_STREAK: u32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lessons {
    streak: u32,
}

impl Lessons {
    pub fn learning(self) -> bool {
        self.streak < LESSON_STREAK
    }

    pub fn streak(self) -> u32 {
        self.streak
    }

    fn fought(&mut self, landed: bool) {
        if !self.learning() {
            return;
        }
        self.streak = if landed { self.streak + 1 } else { 0 };
    }
}

impl App {
    pub fn lessons(&self) -> Lessons {
        self.lessons
    }

    pub(super) fn leave_the_water(&mut self) {
        let Some(Overlay::Fishing(state)) = &self.active_overlay else {
            return;
        };
        if let Some(landed) = state.fought() {
            self.lessons.fought(landed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Launch, SaveFile};

    #[test]
    fn a_player_who_learned_to_fish_has_learned_it_in_the_next_session() {
        let mut app = App::launch(Launch::Player);
        app.lessons = Lessons {
            streak: LESSON_STREAK,
        };
        let text = app.snapshot().to_ron().expect("a save");
        let back = App::resume(SaveFile::from_ron(&text).expect("it reads"), 80, 24);
        assert!(!back.lessons().learning());
    }

    #[test]
    fn a_new_game_starts_with_lessons() {
        assert!(App::launch(Launch::Player).lessons().learning());
    }

    #[test]
    fn three_fish_landed_in_a_row_end_the_lessons_for_good() {
        let mut lessons = Lessons::default();
        lessons.fought(true);
        lessons.fought(true);
        assert!(lessons.learning());
        lessons.fought(true);
        assert!(!lessons.learning());
        lessons.fought(false);
        assert!(!lessons.learning(), "a graduate stays one");
    }

    #[test]
    fn a_fish_that_gets_away_starts_the_streak_again() {
        let mut lessons = Lessons::default();
        lessons.fought(true);
        lessons.fought(true);
        lessons.fought(false);
        assert_eq!(lessons.streak(), 0);
        assert!(lessons.learning());
    }
}
