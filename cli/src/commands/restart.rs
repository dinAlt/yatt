use super::*;
use core::{Interval, Node};
use orm::filter::*;

pub(crate) fn exec(ctx: &AppContext, _ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;
    if let Some((node, interval)) = res {
        let message = format!(
            r#"Task *{}* already running (started at **{}**)."#,
            node.label,
            format_datetime(&interval.begin)
        );
        return Err(CliError::Cmd { message });
    };

    let inteval = ctx.db.intervals().with_max(&Interval::begin_n())?;
    if inteval.is_none() {
        let message = r#"there is no previos running task"#.to_string();
        return Err(CliError::Cmd { message });
    }

    let mut inteval = inteval.unwrap();
    let now = Utc::now();
    inteval.id = 0;
    inteval.begin = now;
    inteval.end = None;

    let node = ctx
        .db
        .nodes()
        .filter(eq(Node::id_n(), inteval.node_id.unwrap()))?;
    if node.is_empty() {
        return Err(CliError::Unexpected {
            message: format!("node with id {}", inteval.node_id.unwrap()),
        });
    };

    let node = node.first().unwrap();

    ctx.db.intervals().save(&inteval)?;

    let text = format!(
        r#"Task *"{}"* restarted at **{}**."#,
        node.label,
        format_datetime(&now)
    );

    ctx.skin.print_text(&text);

    Ok(())
}
