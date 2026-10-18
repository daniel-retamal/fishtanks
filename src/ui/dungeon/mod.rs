use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::dungeon::Above;
use crate::dungeon::expedition::Expedition;
use crate::dungeon::screen::{DungeonState, View};
use crate::ui::layout::Screen;

mod battle;
mod canvas;
mod map;
mod room;
mod words;

pub struct DungeonOverlay<'a> {
    state: &'a DungeonState,
    expedition: Option<&'a Expedition>,
    above: Above<'a>,
    screen: Screen,
    cursor_visible: bool,
}

impl<'a> DungeonOverlay<'a> {
    pub fn new(
        state: &'a DungeonState,
        expedition: Option<&'a Expedition>,
        above: Above<'a>,
        screen: Screen,
        cursor_visible: bool,
    ) -> Self {
        Self {
            state,
            expedition,
            above,
            screen,
            cursor_visible,
        }
    }
}

impl Widget for DungeonOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        self.draw_view(buf);
        if let Some(words) = &self.state.words {
            words::draw(buf, words, self.cursor_visible, self.screen);
        }
    }
}

impl DungeonOverlay<'_> {
    fn draw_view(&self, buf: &mut Buffer) {
        match (&self.state.view, self.expedition) {
            (View::Fight(fight), _) => battle::draw(buf, fight, self.screen),
            (View::Muster(muster), _) => {
                let panel = muster.panel();
                let sheet = room::Sheet {
                    panel: &panel,
                    cursor: room::Cursor {
                        selected: muster.selected(),
                        fish: 0,
                        reading: false,
                    },
                    scroll: &self.state.scroll,
                };
                room::draw(buf, &sheet, self.screen);
            }
            (View::Map(chart), Some(expedition)) => map::draw(
                buf,
                expedition,
                chart,
                (&self.state.scroll, &self.state.sheet_scroll),
                self.screen,
            ),
            (View::Room(lobby), Some(expedition)) => {
                let Some(panel) = expedition.panel(lobby.fish, &self.above) else {
                    return;
                };
                let sheet = room::Sheet {
                    panel: &panel,
                    cursor: room::Cursor {
                        selected: lobby.selected.min(panel.rows.len().saturating_sub(1)),
                        fish: lobby.fish,
                        reading: lobby.reading,
                    },
                    scroll: &self.state.scroll,
                };
                room::draw(buf, &sheet, self.screen);
            }
            (View::Map(_) | View::Room(_), None) => {}
        }
    }
}
