use crate::*;

pub(crate) fn exec(ctx: &AppContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;
    let (node, mut interval) = match res {
        Some((n, i)) => (n, i),
        None => {
            ctx.printer.error("no task running.");
            return Err(CliError::wrap(Box::new(TaskError::NotRunnint)));
        }
    };

    interval.end = Some(Utc::now());
    ctx.db.intervals().save(&interval)?;

    let task = &ctx.db.ancestors(node.id)?;

    ctx.printer.interval_cmd(&IntervalCmdData {
        cmd_text: "Stopping...",
        interval: IntervalData {
            interval: &interval,
            task,
            title: IntervalData::default_title(),
        },
    });

    Ok(())
}
