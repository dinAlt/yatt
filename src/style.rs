use crossterm::style::{Color, ContentStyle};
use std::convert::TryInto;
use termimad::*;

#[derive(Clone)]
pub struct TaskStyle {
  pub name: ContentStyle,
  pub start_time: ContentStyle,
  pub end_time: ContentStyle,
  pub created_time: ContentStyle,
  pub time_span: ContentStyle,
}

impl Default for TaskStyle {
  fn default() -> Self {
    let name = ContentStyle {
      foreground_color: Some(Color::Yellow),
      ..Default::default()
    };
    let start_time = ContentStyle {
      foreground_color: Some(Color::Magenta),
      ..Default::default()
    };
    let end_time = ContentStyle {
      foreground_color: Some(Color::Magenta),
      ..Default::default()
    };
    let created_time = ContentStyle {
      foreground_color: Some(Color::Blue),
      ..Default::default()
    };
    let time_span = ContentStyle {
      foreground_color: Some(Color::Green),
      ..Default::default()
    };

    TaskStyle {
      name,
      start_time,
      end_time,
      created_time,
      time_span,
    }
  }
}

impl TaskStyle {
  pub(crate) fn empty() -> Self {
    let style = ContentStyle::default();

    TaskStyle {
      name: style,
      start_time: style,
      end_time: style,
      created_time: style,
      time_span: style,
    }
  }
}

pub struct TaskListStyle {
  pub name: ContentStyle,
  pub create_date: ContentStyle,
  pub id: ContentStyle,
}

impl Default for TaskListStyle {
  fn default() -> Self {
    let name = ContentStyle {
      foreground_color: Some(Color::Yellow),
      ..Default::default()
    };
    let create_date = ContentStyle {
      foreground_color: Some(Color::Magenta),
      ..Default::default()
    };
    let id = ContentStyle {
      foreground_color: Some(Color::Green),
      ..Default::default()
    };

    TaskListStyle {
      name,
      create_date,
      id,
    }
  }
}

impl TaskListStyle {
  pub(crate) fn empty() -> Self {
    let style = ContentStyle::default();

    TaskListStyle {
      name: style,
      create_date: style,
      id: style,
    }
  }
}

pub struct AppStyle {
  pub task: TaskStyle,
  pub error: ContentStyle,
  pub cmd: ContentStyle,
  pub report: MadSkin,
  pub task_list: TaskListStyle,
  pub screen_width: Option<usize>,
}

impl Default for AppStyle {
  fn default() -> Self {
    let cmd = ContentStyle::default();
    let (width, _) = terminal_size();
    let area: Option<usize> = if width < 4 {
      Some(120)
    } else {
      Some(width.try_into().unwrap())
    };
    let mut report = MadSkin::default();
    report.paragraph.align = Alignment::Center;
    report.table.align = Alignment::Center;
    report.bold.set_fg(Color::Yellow);
    report.italic.object_style = Default::default();
    report.italic.set_fg(Color::Magenta);
    report.inline_code.set_fgbg(Color::Reset, Color::Reset);

    AppStyle {
      task: TaskStyle::default(),
      task_list: TaskListStyle::default(),
      error: ContentStyle {
        foreground_color: Some(Color::Red),
        ..Default::default()
      },
      cmd,
      report,
      screen_width: area,
    }
  }
}

impl AppStyle {
  pub(crate) fn empty() -> Self {
    let report = MadSkin {
      scrollbar: ScrollBarStyle {
        track: StyledChar::new(Default::default(), ' '),
        thumb: StyledChar::new(Default::default(), ' '),
      },
      table: LineStyle {
        compound_style: Default::default(),
        align: Alignment::Unspecified,
      },
      bullet: StyledChar::new(Default::default(), '.'),
      quote_mark: StyledChar::new(
        CompoundStyle::new(None, None, Default::default()),
        '|',
      ),
      horizontal_rule: StyledChar::new(Default::default(), '―'),
      ..Default::default()
    };

    AppStyle {
      task: TaskStyle::empty(),
      task_list: TaskListStyle::empty(),
      error: Default::default(),
      cmd: Default::default(),
      report,
      screen_width: Some(4000),
    }
  }
}
