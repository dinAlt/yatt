use super::*;
use core::Interval;

pub(crate) fn exec(ctx: &CmdContext, ars: &ArgMatches) -> CliResult<()> {
    let res = ctx
        .db
        .cur_running()
        .map_err(|source| CliError::DB { source })?;
    if let Some((node, interval)) = res {
        let message = format!(
            r#"Task *"{}"* already running (started at **{}**)."#,
            node.label,
            format_datetime(&interval.begin)
        );
        return Err(CliError::Cmd { message });
    };

    let path: Vec<&str> = ars.values_of("task").unwrap().collect();
    let path = path.join(" ");
    let path: Vec<&str> = path.split("::").map(|t| t.trim()).collect();

    let nodes = ctx.db.create_path(&path)?;
    let now = Utc::now();
    ctx.db.intervals().save(&Interval {
        id: 0,
        node_id: Some(nodes.last().unwrap().id),
        begin: now,
        end: None,
        deleted: false,
    })?;

    let path = nodes
        .into_iter()
        .map(|n| n.label)
        .collect::<Vec<String>>()
        .join(" -> ");

    let text = format!(
        r#"Task *"{}"* started at **{}**."#,
        path,
        format_datetime(&now)
    );

    ctx.app.skin.print_text(&text);

    Ok(())
}
