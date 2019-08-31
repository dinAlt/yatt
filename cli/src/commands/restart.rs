use crate::*;
use core::{Interval, Node};
use orm::filter::*;

pub(crate) fn exec(ctx: &AppContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;

    if let Some((node, interval)) = res {
        let task = &ctx.db.ancestors(node.id)?;
        let name = format_task_name(&task);
        print_error("task already running.", &ctx.style.error);
        print_interval_info(&task, &interval, &ctx.style.task);
        return Err(CliError::wrap(Box::new(TaskError::AlreadyRunning { name })));
    };

    let inteval = ctx.db.intervals().with_max(&Interval::begin_n())?;
    if inteval.is_none() {
        print_error("there is no priviosly started tasks.\n", &ctx.style.error);
        return Err(CliError::wrap(Box::new(TaskError::NoPrivios)));
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
    print_cmd("Restarting...", &ctx.style.cmd);
    print_interval_info(&task, &interval, &ctx.style.task);

    Ok(())
}
