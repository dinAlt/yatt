use crate::*;

pub(crate) fn exec(ctx: &AppContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;
    let (node, mut interval) = match res {
        Some((n, i)) => (n, i),
        None => {
            print_error("no task running.\n", &ctx.style.error);
            return Err(CliError::wrap(Box::new(TaskError::NotRunnint)));
        }
    };

    interval.end = Some(Utc::now());
    ctx.db.intervals().save(&interval)?;

    let task = &ctx.db.ancestors(node.id)?;

    print_cmd("Stopping...", &ctx.style.cmd);
    print_interval_info(&task, &interval, &ctx.style.task);

    Ok(())
}
