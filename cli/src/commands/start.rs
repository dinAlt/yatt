use crate::*;
use core::Interval;

pub(crate) fn exec(ctx: &AppContext, ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;
    if let Some((node, interval)) = res {
        let task = ctx.db.ancestors(node.id)?;
        return Err(CliError::Task{ source: TaskError::CmdTaskInterval{
            message: "Interval already running.".to_string(),
            interval,
            task,
        }});
    };

    let path: Vec<&str> = ars.values_of("task").unwrap().collect();
    let path = path.join(" ");
    let path: Vec<&str> = path.split("::").map(|t| t.trim()).collect();

    let nodes = ctx.db.create_path(&path)?;
    let interval = Interval {
        id: 0,
        node_id: Some(nodes.last().unwrap().id),
        begin: Utc::now(),
        end: None,
        deleted: false,
    };
    ctx.db.intervals().save(&interval)?;

    ctx.printer.interval_cmd(&IntervalCmdData {
        cmd_text: &"Starting...",
        interval: IntervalData {
            interval: &interval,
            task: &nodes,
            title: IntervalData::default_title(),
        },
    });

    Ok(())
}
