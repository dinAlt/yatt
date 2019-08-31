use crate::*;
use core::Interval;

pub(crate) fn exec(ctx: &AppContext, ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;
    if let Some((node, interval)) = res {
        let task = &ctx.db.ancestors(node.id)?;
        let name = format_task_name(task);
        print_error("task already running.", &ctx.style.error);
        print_interval_info(&task, &interval, &ctx.style.task);
        return Err(CliError::wrap(Box::new(TaskError::AlreadyRunning { name })));
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

    print_cmd("Starting...", &ctx.style.cmd);
    print_interval_info(&nodes, &interval, &ctx.style.task);

    Ok(())
}
