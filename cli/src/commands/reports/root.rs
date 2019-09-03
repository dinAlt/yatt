use chrono::Duration;

use core::*;
use orm::statement::*;

use crate::format::*;
use crate::report::*;
use crate::parse::*;
use crate::*;

pub(crate) fn exec(ctx: &AppContext, ars: &ArgMatches) -> CliResult<()> {
    let intervals = ctx.db.intervals().by_statement(filter(and(
        not(gt(Interval::deleted_n(), 0)),
        ne(Interval::end_n(), CmpVal::Null),
    )))?;

    let ids = intervals.iter().fold(vec![], |mut acc, v| {
        if acc.iter().find(|&&n| n == v.node_id.unwrap()).is_none() {
            acc.push(v.node_id.unwrap());
        };
        acc
    });

    let mut nodes = vec![];
    for id in ids {
        let node = ctx.db.ancestors(id)?;
        if !node.iter().fold(false, |acc, v| acc || v.deleted) {
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

        for i in 0..high + 1 {
            if a[i].label == b[i].label {
                continue;
            }
            return a[i].label.cmp(&(b[i].label));
        }

        a[high].label.cmp(&b[high].label)
    });

    // dbg!(nodes
    //     .iter()
    //     .map(|v| format_task_name(&v))
    //     .collect::<Vec<String>>());

    let mut r = Report::new();
    r.push("Workhours");
    let mut old_path: &[Node] = &[];
    for nn in 0..nodes.len() {
        for i in 0.. {
            if i == old_path.len() || old_path[i].id != nodes[nn][i].id {
                old_path = &nodes[nn][..];
                push_path(&nodes[nn][i..], &mut r, &intervals, i);
                break;
            }
        }
    }

    ctx.printer.report(&r);

    Ok(())
}

fn push_path(pth: &[Node], rep: &mut Report, ints: &[Interval], pad: usize) {
    let mut pad = pad;
    for n in pth {
        let wh = ints
            .iter()
            .filter(|v| v.node_id.unwrap() == n.id)
            .fold(Duration::zero(), |acc, v| acc + (v.end.unwrap() - v.begin));
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

