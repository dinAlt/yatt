use crate::*;

pub(crate) fn exec(ctx: &AppContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;

    if let Some((node, interval)) = res {
        let task = &ctx.db.ancestors(node.id)?;
        print_cmd("Running", &ctx.style.cmd);
        print_interval_info(&task, &interval, &ctx.style.task);
    } else {
        print_cmd("Stopped", &ctx.style.cmd);
        let last = ctx.db.last_running()?;
        if let Some((node, interval)) = last {
            let task = &ctx.db.ancestors(node.id)?;
            print_interval_info(&task, &interval, &ctx.style.task);
        }
    };

    Ok(())
}
