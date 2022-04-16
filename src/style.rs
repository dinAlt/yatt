use core::result::Result;
use crossterm::style::{Color, ContentStyle};
use std::convert::TryInto;
use termimad::*;

use crate::errors::{CliError, CliResult};

#[derive(Clone, Debug)]
pub struct Colors {
  pub c1: Color,
  pub c2: Color,
  pub c3: Color,
  pub c4: Color,
  pub c5: Color,
}
impl Default for Colors {
  fn default() -> Self {
    Colors {
      c1: Color::Yellow,
      c2: Color::Magenta,
      c3: Color::Green,
      c4: Color::Red,
      c5: Color::Reset,
    }
  }
}

impl TryFrom<&str> for Colors {
  type Error = CliError;

  fn try_from(v: &str) -> Result<Self, Self::Error> {
    if v.is_empty() {
      Ok(Default::default())
    } else {
      let mut res: Colors = Default::default();
      for (idx, s) in
        v.split(':').filter(|v| !v.is_empty()).enumerate()
      {
        let c = parse_color(s)?;
        match idx {
          0 => res.c1 = c,
          1 => res.c2 = c,
          2 => res.c3 = c,
          3 => res.c4 = c,
          4 => res.c5 = c,
          _ => break,
        }
      }
      Ok(res)
    }
  }
}

fn parse_color(v: &str) -> CliResult<Color> {
  let c = Color::try_from(v);
  if let Ok(c) = c {
    return Ok(c);
  }
  if let Some(c) = Color::parse_ansi(v) {
    return Ok(c);
  }
  Err(CliError::Parse {
    message: format!("unable to parse colors from \"{}\"", v),
  })
}

#[derive(Clone)]
pub struct TaskStyle {
  pub default: ContentStyle,
  pub name: ContentStyle,
  pub start_time: ContentStyle,
  pub end_time: ContentStyle,
  pub created_time: ContentStyle,
  pub time_span: ContentStyle,
  pub tags: ContentStyle,
}

impl Default for TaskStyle {
  fn default() -> Self {
    Self::new(&Default::default())
  }
}

impl TaskStyle {
  pub(crate) fn new(colors: &Colors) -> Self {
    let default = ContentStyle {
      foreground_color: Some(colors.c5),
      ..Default::default()
    };
    let name = ContentStyle {
      foreground_color: Some(colors.c1),
      ..Default::default()
    };
    let start_time = ContentStyle {
      foreground_color: Some(colors.c2),
      ..Default::default()
    };
    let end_time = ContentStyle {
      foreground_color: Some(colors.c2),
      ..Default::default()
    };
    let created_time = ContentStyle {
      foreground_color: Some(colors.c2),
      ..Default::default()
    };
    let time_span = ContentStyle {
      foreground_color: Some(colors.c3),
      ..Default::default()
    };
    let task = ContentStyle {
      foreground_color: Some(colors.c3),
      ..Default::default()
    };

    TaskStyle {
      default,
      name,
      start_time,
      end_time,
      created_time,
      time_span,
      tags: task,
    }
  }

  pub(crate) fn empty() -> Self {
    let style = ContentStyle::default();

    TaskStyle {
      default: style,
      name: style,
      start_time: style,
      end_time: style,
      created_time: style,
      time_span: style,
      tags: style,
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
    Self::new(&Default::default())
  }
}

impl TaskListStyle {
  pub(crate) fn new(colors: &Colors) -> Self {
    let name = ContentStyle {
      foreground_color: Some(colors.c1),
      ..Default::default()
    };
    let create_date = ContentStyle {
      foreground_color: Some(colors.c2),
      ..Default::default()
    };
    let id = ContentStyle {
      foreground_color: Some(colors.c3),
      ..Default::default()
    };

    TaskListStyle {
      name,
      create_date,
      id,
    }
  }

  pub(crate) fn empty() -> Self {
    let style: ContentStyle = Default::default();
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
  pub plain: ContentStyle,
  pub report: MadSkin,
  pub task_list: TaskListStyle,
  pub screen_width: Option<usize>,
}

impl Default for AppStyle {
  fn default() -> Self {
    Self::new(&Default::default())
  }
}

impl AppStyle {
  pub(crate) fn new(colors: &Colors) -> Self {
    let plain = ContentStyle {
      foreground_color: Some(colors.c5),
      ..Default::default()
    };

    let (width, _) = terminal_size();
    let area: Option<usize> = if width < 4 {
      Some(120)
    } else {
      Some(width.try_into().unwrap())
    };
    let mut report = MadSkin::default();
    report.paragraph.set_fg(colors.c5);
    report.paragraph.align = Alignment::Center;
    report.table.align = Alignment::Center;
    report.bold.set_fg(colors.c1);
    report.italic.object_style = Default::default();
    report.italic.set_fg(colors.c2);
    report.inline_code.set_fgbg(colors.c5, Color::Reset);

    AppStyle {
      task: TaskStyle::new(colors),
      task_list: TaskListStyle::new(colors),
      error: ContentStyle {
        foreground_color: Some(colors.c4),
        ..Default::default()
      },
      plain,
      report,
      screen_width: area,
    }
  }

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
      plain: Default::default(),
      report,
      screen_width: Some(4000),
    }
  }
}
