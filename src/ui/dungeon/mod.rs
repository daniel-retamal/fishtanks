use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::dungeon::state::DungeonState;
use crate::ui::layout::Screen;

mod battle;

pub struct DungeonOverlay<'a> {
    state: &'a DungeonState,
    screen: Screen,
}

impl<'a> DungeonOverlay<'a> {
    pub fn new(state: &'a DungeonState, screen: Screen) -> Self {
        Self { state, screen }
    }
}

impl Widget for DungeonOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        battle::draw(buf, self.state, self.screen);
    }
}
