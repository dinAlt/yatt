use crossterm_style::{Color, ObjectStyle};
use termimad::*;

pub struct TaskStyle {
    pub name: ObjectStyle,
    pub start_time: ObjectStyle,
    pub end_time: ObjectStyle,
    pub time_span: ObjectStyle,
}

impl Default for TaskStyle {
    fn default() -> Self {
        let name = ObjectStyle::default().fg(Color::Yellow);
        let start_time = ObjectStyle::default().fg(Color::Magenta);
        let end_time = ObjectStyle::default().fg(Color::Magenta);
        let time_span = ObjectStyle::default().fg(Color::Green);
        TaskStyle {
            name,
            start_time,
            end_time,
            time_span,
        }
    }
}

pub struct AppStyle {
    pub task: TaskStyle,
    pub error: ObjectStyle,
    pub cmd: ObjectStyle,
    pub mad: MadSkin,
    pub screen_width: Option<usize>,
}

impl Default for AppStyle {
    fn default() -> Self {
        let cmd = ObjectStyle::default();
        let (width, _) = terminal_size();
        let mut area = None;
        if width < 4 {
            area = Some(120);
        }

        AppStyle {
            task: TaskStyle::default(),
            error: ObjectStyle::default().fg(Color::Red),
            cmd,
            mad: MadSkin::default(),
            screen_width: area,
        }
    }
}
