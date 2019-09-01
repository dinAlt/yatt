use crate::*;
use core::{Interval, Node};
use orm::statement::*;

pub(crate) fn exec(ctx: &AppContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;

    if let Some((node, interval)) = res {
        let task = ctx.db.ancestors(node.id)?;
        return Err(CliError::Task {
            source: TaskError::CmdTaskInterval {
                message: "Interval already running.".to_string(),
                interval,
                task,
            },
        });
    };

    let inteval = ctx.db.intervals().with_max(&Interval::begin_n())?;
    if inteval.is_none() {
        return Err(CliError::Task {
            source: TaskError::Cmd {
                message: "There is no priviosly started tasks.".to_string(),
            },
        });
    }

    let mut interval = inteval.unwrap();
    let now = Utc::now();
    interval.id = 0;
    interval.begin = now;
    interval.end = None;

    let node = ctx
        .db
        .nodes()
        .filter(eq(Node::id_n(), interval.node_id.unwrap()))?;
    if node.is_empty() {
        return Err(CliError::Unexpected {
            message: format!("node with id {}", interval.node_id.unwrap()),
        });
    };

    let node = node.first().unwrap();

    ctx.db.intervals().save(&interval)?;

    let task = &ctx.db.ancestors(node.id)?;
    ctx.printer.interval_cmd(&IntervalCmdData {
        cmd_text: &"Restarting...",
        interval: IntervalData {
            interval: &interval,
            task,
            title: IntervalData::default_title(),
        },
    });

    Ok(())
}
