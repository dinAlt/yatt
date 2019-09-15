use crate::*;
use crate::core::{Interval, Node};
use yatt_orm::statement::*;

pub(crate) fn exec(ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
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

    let interval = ctx.db.intervals().by_statement(
        filter(and(ne(Interval::deleted_n(), 1), ne(Interval::closed_n(), 1)))
        .sort(&Interval::end_n(), SortDir::Descend)
        .limit(1))?;

    if interval.is_empty() {
        return Err(CliError::Task {
            source: TaskError::Cmd {
                message: "There is no priviosly started tasks.".to_string(),
            },
        });
    }

    let mut interval = interval.first().unwrap().to_owned();
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

pub fn register<'a>(app: App<'a, 'a>) -> App {
    app.subcommand(SubCommand::with_name("restart").about("restart last task"))
}
