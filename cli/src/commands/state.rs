use crate::*;

pub(crate) fn exec(ctx: &AppContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;

    if let Some((node, interval)) = res {
        let task = &ctx.db.ancestors(node.id)?;
        ctx.printer.interval_cmd(&IntervalCmdData {
            cmd_text: &"Running",
            interval: IntervalData {
                interval: &interval,
                task,
                title: IntervalData::default_title(),
            },
        });
    } else {
        let last = ctx.db.last_running()?;
        let cmd_text = &"Stopped";
        if let Some((node, interval)) = last {
            let task = &ctx.db.ancestors(node.id)?;
            ctx.printer.interval_cmd(&IntervalCmdData {
                cmd_text,
                interval: IntervalData {
                    interval: &interval,
                    task,
                    title: &"Previous interval:",
                },
            });
        } else {
            ctx.printer.cmd(cmd_text);
        }
    };

    Ok(())
}
