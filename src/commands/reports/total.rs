use chrono::Duration;
use std::cmp::Ordering;

use crate::core::*;
use yatt_orm::statement::*;
use yatt_orm::FieldVal;

use crate::parse::*;
use crate::report::*;
use crate::*;

pub(crate) fn exec<T: DBRoot, P: Printer>(
  ctx: &AppContext<T, P>,
  args: &ArgMatches,
) -> CliResult<()> {
  let (start, end) = if let Some(v) = args.values_of("period") {
    parse_period(
      &v.collect::<Vec<_>>().join(" "),
      &PeriodOpts::default(),
    )?
  } else {
    (Local::today().and_hms(0, 0, 0).into(), Local::now().into())
  };
  let mut intervals: Vec<Interval> = ctx.db.get_by_statement(
    filter(and(
      and(
        or(
          gt(Interval::end_n(), start),
          eq(Interval::end_n(), FieldVal::Null),
        ),
        lt(Interval::begin_n(), end),
      ),
      not(gt(Interval::deleted_n(), 0)),
    ))
    .sort(Interval::begin_n(), SortDir::Ascend),
  )?;
  if !intervals.is_empty() {
    if intervals[0].begin < start {
      intervals[0].begin = start.to_owned();
    }
    let high = intervals.len() - 1;
    if intervals[high].end.is_none()
      || intervals[high].end.unwrap() > end
    {
      intervals[high].end = Some(end.to_owned());
    }
  }

  let ids = intervals.iter().fold(vec![], |mut acc, v| {
    if !acc.iter().any(|&n| n == v.node_id.unwrap()) {
      acc.push(v.node_id.unwrap());
    };
    acc
  });

  let mut nodes = vec![];
  for id in ids {
    let node = ctx.db.ancestors(id)?;
    if !node.iter().any(|v| v.deleted) {
      nodes.push(node);
    }
  }

  nodes.sort_by(|a, b| {
    let high = {
      if a.len() > b.len() {
        b.len()
      } else {
        a.len()
      }
    } - 1;

    for i in 0..=high {
      if a[i].label == b[i].label {
        continue;
      }
      return a[i].label.cmp(&(b[i].label));
    }

    let res = a[high].label.cmp(&b[high].label);
    if let Ordering::Equal = res {
      return a.len().cmp(&b.len());
    }

    res
  });

  let mut r = Report::new();
  r.push("Total time.");
  r.push((start, end));
  let mut old_path: &[Node] = &[];
  let mut sub_total = Duration::zero();
  let mut total = Duration::zero();
  let mut round = 0;
  for node in &nodes {
    for i in 0.. {
      if i == old_path.len() || old_path[i].id != node[i].id {
        old_path = &node[..];
        if i == 0 {
          if round > 1 && !sub_total.is_zero() {
            r.push(Row::SubTotal(vec![Cell::Duration(sub_total)]));
          }
          sub_total = Duration::zero();
          round = 0;
        }
        push_path(
          &node[i..],
          &mut r,
          &intervals,
          i,
          &mut sub_total,
          &mut total,
        );
        round += 1;
        break;
      }
    }
  }

  if !sub_total.is_zero() && round > 1 {
    r.push(Row::SubTotal(vec![Cell::Duration(sub_total)]));
  }
  if !total.is_zero() {
    r.push(Row::Total(vec![Cell::Duration(total)]));
  }

  ctx.printer.report(&r);

  Ok(())
}

fn push_path(
  pth: &[Node],
  rep: &mut Report,
  ints: &[Interval],
  pad: usize,
  sub_total: &mut Duration,
  total: &mut Duration,
) {
  let mut pad = pad;
  for n in pth {
    let wh = Duration::seconds(
      ints
        .iter()
        .filter(|v| v.node_id.unwrap() == n.id)
        .fold(Duration::zero(), |acc, v| {
          acc + (v.end.unwrap() - v.begin)
        })
        .num_seconds(),
    );
    *sub_total = *sub_total + wh;
    *total = *total + wh;

    let mut row = vec![];
    if pad == 0 {
      row.push(Cell::String(n.label.to_owned()));
    } else {
      row.push(Cell::Nested(
        Box::new(Cell::String(n.label.to_owned())),
        pad,
      ));
    };
    if !wh.is_zero() {
      row.push(Cell::Duration(wh));
    }
    if pad == 0 {
      rep.push(row);
    } else {
      rep.push(Row::Nested(row));
    }
    pad += 1;
  }
}

pub fn register<'a>(app: App<'a, 'a>) -> App {
  app.subcommand(
    SubCommand::with_name("total")
      .about("Total time for period (default - currernt day).")
      .arg(
        Arg::with_name("period")
          .short("p")
          .long("period")
          .help("report period")
          .takes_value(true)
          .multiple(true),
      ),
  )
}
