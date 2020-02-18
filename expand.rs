#![feature(prelude_import)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
use chrono::prelude::*;
use clap;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use config::{Config, File};
use crossterm_style::Color::*;
use dirs;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use termimad::*;
mod commands {
    use crate::*;
    mod cancel {
        use crate::*;
        pub(crate) fn exec(ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
            let res = ctx
                .db
                .cur_running()
                .map_err(|source| CliError::DB { source })?;
            if res.is_none() {
                return Err(CliError::Cmd {
                    message: "no interval running.".into(),
                });
            }
            let (node, mut interval) = res.unwrap();
            let nodes = ctx.db.ancestors(node.id)?;
            interval.end = Some(Utc::now());
            interval.deleted = true;
            ctx.db.intervals().save(&interval)?;
            ctx.printer.interval_cmd(&IntervalCmdData {
                cmd_text: &"Current interval canceled...",
                interval: IntervalData {
                    interval: &interval,
                    task: &nodes,
                    title: IntervalData::default_title(),
                },
            });
            Ok(())
        }
        pub fn register<'a>(app: App<'a, 'a>) -> App {
            app.subcommand(SubCommand::with_name("cancel").about("Cancel current interval."))
        }
    }
    mod delete {
        use crate::*;
        mod interval {
            use crate::core::*;
            use crate::*;
            use crossterm_input::input;
            use std::convert::TryInto;
            use yatt_orm::{statement::*, DBError, FieldVal};
            pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
                let no_prompt = args.is_present("yes");
                let id: i64 =
                    args.value_of("ID")
                        .unwrap()
                        .parse()
                        .map_err(|_| CliError::Parse {
                            message: "Unable to parse interval ID".into(),
                        })?;
                let mut interval = if id < 0 {
                    let offset: usize = (-id).try_into().unwrap();
                    let intervals = ctx.db.intervals().by_statement(
                        filter(and(
                            ne(Interval::deleted_n(), 1),
                            ne(Interval::end_n(), FieldVal::Null),
                        ))
                        .sort(&Interval::begin_n(), SortDir::Descend)
                        .limit(offset),
                    )?;
                    if intervals.len() < offset {
                        return Err(CliError::Cmd {
                            message: "There is no interval with given offset".into(),
                        });
                    }
                    intervals.first().unwrap().to_owned()
                } else {
                    let id: usize = id.try_into().unwrap();
                    ctx.db.intervals().by_id(id).map_err(|source| {
                        if let DBError::IsEmpty { .. } = source {
                            return CliError::Cmd {
                                message: "There is no interval with given ID".into(),
                            };
                        }
                        CliError::DB { source }
                    })?
                };
                let task = ctx
                    .db
                    .nodes()
                    .by_id(interval.node_id.unwrap())
                    .map_err(|source| CliError::DB { source })?;
                let task = ctx
                    .db
                    .ancestors(task.id)
                    .map_err(|source| CliError::DB { source })?;
                interval.deleted = true;
                let interval_data = IntervalData {
                    interval: &interval,
                    title: IntervalData::default_title(),
                    task: &task,
                };
                if no_prompt {
                    ctx.db
                        .intervals()
                        .save(&interval)
                        .map_err(|source| CliError::DB { source })?;
                    ctx.printer.interval_cmd(&IntervalCmdData {
                        cmd_text: &"Successfully deleted...",
                        interval: interval_data,
                    });
                } else {
                    ctx.printer.interval_cmd(&IntervalCmdData {
                        cmd_text: &"Are you sure, you want to delete interval? [y/n]",
                        interval: interval_data,
                    });
                    let input = input();
                    if input.read_line()?.trim().to_lowercase() == "y" {
                        ctx.db
                            .intervals()
                            .save(&interval)
                            .map_err(|source| CliError::DB { source })?;
                        ctx.printer.cmd("Successfully deleted...")
                    } else {
                        ctx.printer.cmd("Cancelled...")
                    }
                }
                Ok(())
            }
            pub fn register<'a>(app: App<'a, 'a>) -> App {
                app.subcommand(
                    SubCommand::with_name("interval")
                        .setting(AppSettings::AllowNegativeNumbers)
                        .about("Deletes interval")
                        .arg(
                            Arg::with_name("ID")
                                .help("[ID] or -[offset] from NOW (starting with -1)")
                                .required(true),
                        ),
                )
            }
        }
        mod root {
            use crate::*;
            pub(crate) fn exec(_ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
                Ok(())
            }
            pub fn register<'a>(app: App<'a, 'a>) -> App {
                app
            }
        }
        pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
            match args.subcommand() {
                ("interval", Some(m)) => interval::exec(ctx, m),
                _ => root::exec(ctx, args),
            }
        }
        pub fn register<'a>(app: App<'a, 'a>) -> App {
            let sub = SubCommand::with_name("delete")
                .setting(AppSettings::ArgRequiredElseHelp)
                .alias("remove")
                .alias("rm")
                .about("Delete entity")
                .arg(
                    Arg::with_name("yes")
                        .short("y")
                        .help("Delete with no prompt"),
                );
            let sub = root::register(sub);
            let sub = interval::register(sub);
            app.subcommand(sub)
        }
    }
    mod reports {
        use crate::*;
        mod root {
            use crate::*;
            pub(crate) fn exec(_ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
                Ok(())
            }
            pub fn register<'a>(app: App<'a, 'a>) -> App {
                app
            }
        }
        mod total {
            use crate::core::*;
            use crate::parse::*;
            use crate::report::*;
            use crate::*;
            use chrono::Duration;
            use std::cmp::Ordering;
            use yatt_orm::statement::*;
            use yatt_orm::FieldVal;
            pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
                let (start, end) = if let Some(v) = args.values_of("period") {
                    parse_period(&v.collect::<Vec<_>>().join(" "), &PeriodOpts::default())?
                } else {
                    (Local::today().and_hms(0, 0, 0).into(), Local::now().into())
                };
                let mut intervals = ctx.db.intervals().by_statement(
                    filter(and(
                        and(
                            or(
                                gt(Interval::end_n(), start),
                                eq(Interval::end_n(), FieldVal::Null),
                            ),
                            lt(Interval::begin_n(), end),
                        ),
                        not(gt(Interval::deleted_n(), 0)),
                    ))
                    .sort(&Interval::begin_n(), SortDir::Ascend),
                )?;
                if !intervals.is_empty() {
                    if intervals[0].begin < start {
                        intervals[0].begin = start.to_owned();
                    }
                    let high = intervals.len() - 1;
                    if intervals[high].end.is_none() || intervals[high].end.unwrap() > end {
                        intervals[high].end = Some(end.to_owned());
                    }
                }
                let ids = intervals
                    .iter()
                    .fold(<[_]>::into_vec(box []), |mut acc, v| {
                        if acc.iter().find(|&&n| n == v.node_id.unwrap()).is_none() {
                            acc.push(v.node_id.unwrap());
                        };
                        acc
                    });
                let mut nodes = <[_]>::into_vec(box []);
                for id in ids {
                    let node = ctx.db.ancestors(id)?;
                    if !node.iter().any(|v| v.deleted) {
                        nodes.push(node);
                    }
                }
                nodes.sort_by(|a, b| {
                    let high = {
                        if a.len() > b.len() {
                            b.len()
                        } else {
                            a.len()
                        }
                    } - 1;
                    for i in 0..=high {
                        if a[i].label == b[i].label {
                            continue;
                        }
                        return a[i].label.cmp(&(b[i].label));
                    }
                    let res = a[high].label.cmp(&b[high].label);
                    if let Ordering::Equal = res {
                        return a.len().cmp(&b.len());
                    }
                    res
                });
                let mut r = Report::new();
                r.push("Total time.");
                r.push((start, end));
                let mut old_path: &[Node] = &[];
                let mut sub_total = Duration::zero();
                let mut total = Duration::zero();
                let mut round = 0;
                for node in &nodes {
                    for i in 0.. {
                        if i == old_path.len() || old_path[i].id != node[i].id {
                            old_path = &node[..];
                            if i == 0 {
                                if round > 1 && !sub_total.is_zero() {
                                    r.push(Row::SubTotal(<[_]>::into_vec(box [Cell::Duration(
                                        sub_total,
                                    )])));
                                }
                                sub_total = Duration::zero();
                                round = 0;
                            }
                            push_path(
                                &node[i..],
                                &mut r,
                                &intervals,
                                i,
                                &mut sub_total,
                                &mut total,
                            );
                            round += 1;
                            break;
                        }
                    }
                }
                if !sub_total.is_zero() && round > 1 {
                    r.push(Row::SubTotal(<[_]>::into_vec(box [Cell::Duration(
                        sub_total,
                    )])));
                }
                if !total.is_zero() {
                    r.push(Row::Total(<[_]>::into_vec(box [Cell::Duration(total)])));
                }
                ctx.printer.report(&r);
                Ok(())
            }
            fn push_path(
                pth: &[Node],
                rep: &mut Report,
                ints: &[Interval],
                pad: usize,
                sub_total: &mut Duration,
                total: &mut Duration,
            ) {
                let mut pad = pad;
                for n in pth {
                    let wh = Duration::seconds(
                        ints.iter()
                            .filter(|v| v.node_id.unwrap() == n.id)
                            .fold(Duration::zero(), |acc, v| acc + (v.end.unwrap() - v.begin))
                            .num_seconds(),
                    );
                    *sub_total = *sub_total + wh;
                    *total = *total + wh;
                    let mut row = <[_]>::into_vec(box []);
                    if pad == 0 {
                        row.push(Cell::String(n.label.to_owned()));
                    } else {
                        row.push(Cell::Nested(
                            Box::new(Cell::String(n.label.to_owned())),
                            pad,
                        ));
                    };
                    if !wh.is_zero() {
                        row.push(Cell::Duration(wh));
                    }
                    if pad == 0 {
                        rep.push(row);
                    } else {
                        rep.push(Row::Nested(row));
                    }
                    pad += 1;
                }
            }
            pub fn register<'a>(app: App<'a, 'a>) -> App {
                app.subcommand(
                    SubCommand::with_name("total")
                        .about("Total time for period (default - currernt day).")
                        .arg(
                            Arg::with_name("period")
                                .short("p")
                                .long("period")
                                .help("report period")
                                .takes_value(true)
                                .multiple(true),
                        ),
                )
            }
        }
        pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
            match args.subcommand() {
                ("total", Some(m)) => total::exec(ctx, m),
                _ => root::exec(ctx, args),
            }
        }
        pub fn register<'a>(app: App<'a, 'a>) -> App {
            let sub = SubCommand::with_name("report")
                .setting(AppSettings::ArgRequiredElseHelp)
                .about("show selected report");
            let sub = root::register(sub);
            let sub = total::register(sub);
            app.subcommand(sub)
        }
    }
    mod restart {
        use crate::core::{Interval, Node};
        use crate::*;
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
                filter(and(
                    ne(Interval::deleted_n(), 1),
                    ne(Interval::closed_n(), 1),
                ))
                .sort(&Interval::end_n(), SortDir::Descend)
                .limit(1),
            )?;
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
                    message: {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["node with id "],
                            &match (&interval.node_id.unwrap(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    },
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
    }
    mod root {
        use crate::*;
        pub(crate) fn exec(_ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
            Ok(())
        }
        pub fn register<'a>(app: App<'a, 'a>) -> App {
            app
        }
    }
    mod start {
        use crate::core::Interval;
        use crate::*;
        pub(crate) fn exec(ctx: &AppContext, args: &ArgMatches) -> CliResult<()> {
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
            let path: Vec<&str> = args.values_of("TASK").unwrap().collect();
            let path = path.join(" ");
            let path: Vec<&str> = path.split("::").map(|t| t.trim()).collect();
            let nodes = ctx.db.create_path(&path)?;
            let interval = Interval {
                id: 0,
                node_id: Some(nodes.last().unwrap().id),
                begin: Utc::now(),
                end: None,
                deleted: false,
                closed: false,
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
        pub fn register<'a>(app: App<'a, 'a>) -> App {
            app.subcommand(
                SubCommand::with_name("start")
                    .alias("run")
                    .about("Starts new task, or continues existing")
                    .setting(AppSettings::ArgRequiredElseHelp)
                    .arg(
                        Arg::with_name("TASK")
                            .help("Task name with nested tasks, delimited by \"::\"")
                            .required(true)
                            .multiple(true),
                    ),
            )
        }
    }
    mod state {
        use crate::*;
        pub(crate) fn exec(ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
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
        pub fn register<'a>(app: App<'a, 'a>) -> App {
            app.subcommand(
                SubCommand::with_name("state")
                    .alias("status")
                    .about("show running state"),
            )
        }
    }
    mod stop {
        use crate::*;
        pub(crate) fn exec(ctx: &AppContext, _args: &ArgMatches) -> CliResult<()> {
            let res = ctx
                .db
                .cur_running()
                .map_err(|source| CliError::DB { source })?;
            if res.is_none() {
                return Err(CliError::Task {
                    source: TaskError::Cmd {
                        message: "No task running.".to_string(),
                    },
                });
            }
            let (node, mut interval) = res.unwrap();
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
        pub fn register<'a>(app: App<'a, 'a>) -> App {
            app.subcommand(SubCommand::with_name("stop").about("stops running task"))
        }
    }
    pub fn exec(ctx: &AppContext) -> CliResult<()> {
        match ctx.args.subcommand() {
            ("start", Some(m)) => start::exec(ctx, m),
            ("stop", Some(m)) => stop::exec(ctx, m),
            ("restart", Some(m)) => restart::exec(ctx, m),
            ("state", Some(m)) => state::exec(ctx, m),
            ("report", Some(m)) => reports::exec(ctx, m),
            ("cancel", Some(m)) => cancel::exec(ctx, m),
            ("delete", Some(m)) => delete::exec(ctx, m),
            _ => root::exec(ctx, &ctx.args),
        }
    }
    pub fn register<'a>(app: App<'a, 'a>) -> App {
        let app = root::register(app);
        let app = start::register(app);
        let app = stop::register(app);
        let app = restart::register(app);
        let app = state::register(app);
        let app = cancel::register(app);
        let app = reports::register(app);
        delete::register(app)
    }
}
mod core {
    use crate::history::LocalUnique;
    use chrono::prelude::*;
    use std::convert::TryFrom;
    use yatt_orm::errors::{DBError, DBResult};
    use yatt_orm::statement::*;
    use yatt_orm::FieldVal;
    use yatt_orm::{BoxStorage, Identifiers};
    pub trait DBRoot {
        fn nodes(&self) -> BoxStorage<Node>;
        fn intervals(&self) -> BoxStorage<Interval>;
    }
    impl dyn DBRoot {
        pub fn cur_running(&self) -> DBResult<Option<(Node, Interval)>> {
            let intrval = self
                .intervals()
                .filter(eq(Interval::end_n(), FieldVal::Null))?;
            if intrval.is_empty() {
                return Ok(None);
            }
            let interval = intrval[0].clone();
            let node = self
                .nodes()
                .filter(eq(Node::id_n(), interval.node_id.unwrap()))?;
            if node.is_empty() {
                return Err(DBError::Unexpected {
                    message: {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Task with id=", " for interval with id=", ", not exists"],
                            &match (&interval.node_id.unwrap_or(0), &interval.id) {
                                (arg0, arg1) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    },
                });
            }
            let node = node[0].clone();
            Ok(Some((node, interval)))
        }
        pub fn last_running(&self) -> DBResult<Option<(Node, Interval)>> {
            let interval = self.intervals().by_statement(
                filter(ne(Interval::deleted_n(), 1))
                    .sort(&Interval::end_n(), SortDir::Descend)
                    .limit(1),
            )?;
            if interval.is_empty() {
                return Ok(None);
            }
            let interval = interval.first().unwrap();
            let node = self
                .nodes()
                .filter(eq(Node::id_n(), interval.node_id.unwrap()))?;
            if node.is_empty() {
                return Err(DBError::Unexpected {
                    message: {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Task with id=", " for interval with id=", ", not exists"],
                            &match (&interval.node_id.unwrap_or(0), &interval.id) {
                                (arg0, arg1) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    },
                });
            }
            let node = node[0].clone();
            Ok(Some((node, interval.to_owned())))
        }
        pub fn find_path(&self, path: &[&str]) -> DBResult<Vec<Node>> {
            let mut parent = FieldVal::Null;
            let mut res = Vec::new();
            for p in path.iter() {
                let node = self.find_path_part(&p, &parent)?;
                if let Some(node) = node {
                    parent = FieldVal::Usize(node.id);
                    res.push(node);
                } else {
                    return Ok(res);
                }
            }
            Ok(res)
        }
        #[doc = " Crates all not exist node of path, and returns all nodes."]
        pub fn create_path(&self, path: &[&str]) -> DBResult<Vec<Node>> {
            if path.is_empty() {
                return Err(DBError::Unexpected {
                    message: "provided value for path is empty".to_string(),
                });
            }
            let mut nodes = self.find_path(path)?;
            let p_len = path.len();
            let n_len = nodes.len();
            let high = p_len - (p_len - n_len);
            if high == p_len {
                return Ok(nodes);
            }
            let high = usize::try_from(high).unwrap();
            let mut parent_id = None;
            if !nodes.is_empty() {
                parent_id = Some(nodes.last().unwrap().id)
            }
            let mut node;
            for n in path.iter().take(p_len).skip(high) {
                node = Node {
                    id: 0,
                    parent_id,
                    label: n.to_string(),
                    created: Utc::now(),
                    closed: false,
                    deleted: false,
                };
                let id = self.nodes().save(&node)?;
                node.id = id;
                parent_id = Some(id);
                nodes.push(node.clone());
            }
            Ok(nodes)
        }
        #[doc = " Returns ancestors of node with givent id, inluding"]
        #[doc = " the node with given id itself."]
        pub fn ancestors(&self, id: usize) -> DBResult<Vec<Node>> {
            let mut res = Vec::new();
            let mut next = Some(id);
            while next.is_some() {
                let node = self.nodes().by_id(next.unwrap())?;
                next = node.parent_id;
                res.push(node);
            }
            res.reverse();
            Ok(res)
        }
        fn find_path_part(&self, name: &str, parent_id: &FieldVal) -> DBResult<Option<Node>> {
            let nodes = self.nodes().filter(and(
                eq(Node::parent_id_n(), parent_id),
                eq(Node::label_n(), name),
            ))?;
            if nodes.is_empty() {
                return Ok(None);
            };
            if nodes.len() > 1 {
                return Err(DBError::Unexpected {
                    message: "query return multiple rows".to_string(),
                });
            };
            Ok(Some(nodes[0].clone()))
        }
    }
    pub struct Node {
        pub id: usize,
        pub parent_id: Option<usize>,
        pub label: String,
        pub created: DateTime<Utc>,
        pub closed: bool,
        pub deleted: bool,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Node {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Node {
                    id: ref __self_0_0,
                    parent_id: ref __self_0_1,
                    label: ref __self_0_2,
                    created: ref __self_0_3,
                    closed: ref __self_0_4,
                    deleted: ref __self_0_5,
                } => {
                    let mut debug_trait_builder = f.debug_struct("Node");
                    let _ = debug_trait_builder.field("id", &&(*__self_0_0));
                    let _ = debug_trait_builder.field("parent_id", &&(*__self_0_1));
                    let _ = debug_trait_builder.field("label", &&(*__self_0_2));
                    let _ = debug_trait_builder.field("created", &&(*__self_0_3));
                    let _ = debug_trait_builder.field("closed", &&(*__self_0_4));
                    let _ = debug_trait_builder.field("deleted", &&(*__self_0_5));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Node {
        #[inline]
        fn clone(&self) -> Node {
            match *self {
                Node {
                    id: ref __self_0_0,
                    parent_id: ref __self_0_1,
                    label: ref __self_0_2,
                    created: ref __self_0_3,
                    closed: ref __self_0_4,
                    deleted: ref __self_0_5,
                } => Node {
                    id: ::core::clone::Clone::clone(&(*__self_0_0)),
                    parent_id: ::core::clone::Clone::clone(&(*__self_0_1)),
                    label: ::core::clone::Clone::clone(&(*__self_0_2)),
                    created: ::core::clone::Clone::clone(&(*__self_0_3)),
                    closed: ::core::clone::Clone::clone(&(*__self_0_4)),
                    deleted: ::core::clone::Clone::clone(&(*__self_0_5)),
                },
            }
        }
    }
    impl Node {
        pub fn id_n() -> &str {
            &"id"
        }
        pub fn parent_id_n() -> &str {
            &"parent_id"
        }
        pub fn label_n() -> &str {
            &"label"
        }
        pub fn created_n() -> &str {
            &"created"
        }
        pub fn closed_n() -> &str {
            &"closed"
        }
        pub fn deleted_n() -> &str {
            &"deleted"
        }
    }
    impl yatt_orm::StoreObject for Node {
        fn get_type_name(&self) -> &'static str {
            &"Node"
        }
        fn get_field_val(&self, field_name: &str) -> yatt_orm::FieldVal {
            match field_name {
                "id" => self.id.clone().into(),
                "parent_id" => self.parent_id.clone().into(),
                "label" => self.label.clone().into(),
                "created" => self.created.clone().into(),
                "closed" => self.closed.clone().into(),
                "deleted" => self.deleted.clone().into(),
                _ => ::std::rt::begin_panic({
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["there is no field ", " in struct "],
                        &match (&field_name, &"Node") {
                            (arg0, arg1) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                            ],
                        },
                    ));
                    res
                }),
            }
        }
        fn get_fields_list(&self) -> &'static [&'static str] {
            &[
                &"id",
                &"parent_id",
                &"label",
                &"created",
                &"closed",
                &"deleted",
            ]
        }
    }
    impl ToString for Node {
        fn to_string(&self) -> String {
            self.label.to_owned()
        }
    }
    impl LocalUnique for Node {
        fn get_local_id(&self) -> usize {
            self.id
        }
    }
    pub struct Interval {
        pub id: usize,
        pub node_id: Option<usize>,
        pub begin: DateTime<Utc>,
        pub end: Option<DateTime<Utc>>,
        pub deleted: bool,
        pub closed: bool,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Interval {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Interval {
                    id: ref __self_0_0,
                    node_id: ref __self_0_1,
                    begin: ref __self_0_2,
                    end: ref __self_0_3,
                    deleted: ref __self_0_4,
                    closed: ref __self_0_5,
                } => {
                    let mut debug_trait_builder = f.debug_struct("Interval");
                    let _ = debug_trait_builder.field("id", &&(*__self_0_0));
                    let _ = debug_trait_builder.field("node_id", &&(*__self_0_1));
                    let _ = debug_trait_builder.field("begin", &&(*__self_0_2));
                    let _ = debug_trait_builder.field("end", &&(*__self_0_3));
                    let _ = debug_trait_builder.field("deleted", &&(*__self_0_4));
                    let _ = debug_trait_builder.field("closed", &&(*__self_0_5));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for Interval {
        #[inline]
        fn clone(&self) -> Interval {
            match *self {
                Interval {
                    id: ref __self_0_0,
                    node_id: ref __self_0_1,
                    begin: ref __self_0_2,
                    end: ref __self_0_3,
                    deleted: ref __self_0_4,
                    closed: ref __self_0_5,
                } => Interval {
                    id: ::core::clone::Clone::clone(&(*__self_0_0)),
                    node_id: ::core::clone::Clone::clone(&(*__self_0_1)),
                    begin: ::core::clone::Clone::clone(&(*__self_0_2)),
                    end: ::core::clone::Clone::clone(&(*__self_0_3)),
                    deleted: ::core::clone::Clone::clone(&(*__self_0_4)),
                    closed: ::core::clone::Clone::clone(&(*__self_0_5)),
                },
            }
        }
    }
    impl Interval {
        pub fn id_n() -> &str {
            &"id"
        }
        pub fn node_id_n() -> &str {
            &"node_id"
        }
        pub fn begin_n() -> &str {
            &"begin"
        }
        pub fn end_n() -> &str {
            &"end"
        }
        pub fn deleted_n() -> &str {
            &"deleted"
        }
        pub fn closed_n() -> &str {
            &"closed"
        }
    }
    impl yatt_orm::StoreObject for Interval {
        fn get_type_name(&self) -> &'static str {
            &"Interval"
        }
        fn get_field_val(&self, field_name: &str) -> yatt_orm::FieldVal {
            match field_name {
                "id" => self.id.clone().into(),
                "node_id" => self.node_id.clone().into(),
                "begin" => self.begin.clone().into(),
                "end" => self.end.clone().into(),
                "deleted" => self.deleted.clone().into(),
                "closed" => self.closed.clone().into(),
                _ => ::std::rt::begin_panic({
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["there is no field ", " in struct "],
                        &match (&field_name, &"Interval") {
                            (arg0, arg1) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                            ],
                        },
                    ));
                    res
                }),
            }
        }
        fn get_fields_list(&self) -> &'static [&'static str] {
            &[&"id", &"node_id", &"begin", &"end", &"deleted", &"closed"]
        }
    }
    impl ToString for Interval {
        fn to_string(&self) -> String {
            let end = match self.end {
                Some(d) => d.to_rfc3339(),
                None => "never".to_string(),
            };
            {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["[started: ", " stopped: ", "]"],
                    &match (&self.begin, &end) {
                        (arg0, arg1) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                        ],
                    },
                ));
                res
            }
        }
    }
    impl LocalUnique for Interval {
        fn get_local_id(&self) -> usize {
            self.id
        }
    }
}
mod errors {
    use super::format::*;
    use crate::core::*;
    use config::ConfigError;
    use custom_error::*;
    use std::error::Error;
    use std::io;
    use yatt_orm::errors::*;
    pub type CliResult<T> = std::result::Result<T, CliError>;
    pub enum CliError {
        DB { source: DBError },
        Config { source: ConfigError },
        Io { source: io::Error },
        AppDir { message: String },
        Cmd { message: String },
        Unexpected { message: String },
        Wrapped { source: Box<dyn Error> },
        Task { source: TaskError },
        Parse { message: String },
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for CliError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&CliError::DB {
                    source: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("DB");
                    let _ = debug_trait_builder.field("source", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::Config {
                    source: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Config");
                    let _ = debug_trait_builder.field("source", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::Io {
                    source: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Io");
                    let _ = debug_trait_builder.field("source", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::AppDir {
                    message: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("AppDir");
                    let _ = debug_trait_builder.field("message", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::Cmd {
                    message: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Cmd");
                    let _ = debug_trait_builder.field("message", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::Unexpected {
                    message: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Unexpected");
                    let _ = debug_trait_builder.field("message", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::Wrapped {
                    source: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Wrapped");
                    let _ = debug_trait_builder.field("source", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::Task {
                    source: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Task");
                    let _ = debug_trait_builder.field("source", &&(*__self_0));
                    debug_trait_builder.finish()
                }
                (&CliError::Parse {
                    message: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Parse");
                    let _ = debug_trait_builder.field("message", &&(*__self_0));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    impl std::error::Error for CliError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            #[allow(unused_variables, unreachable_code)]
            match self {
                CliError::DB { source } => {
                    {
                        return Some(std::borrow::Borrow::borrow(source));
                    };
                    None
                }
                CliError::Config { source } => {
                    {
                        return Some(std::borrow::Borrow::borrow(source));
                    };
                    None
                }
                CliError::Io { source } => {
                    {
                        return Some(std::borrow::Borrow::borrow(source));
                    };
                    None
                }
                CliError::AppDir { message } => None,
                CliError::Cmd { message } => None,
                CliError::Unexpected { message } => None,
                CliError::Wrapped { source } => {
                    {
                        return Some(std::borrow::Borrow::borrow(source));
                    };
                    None
                }
                CliError::Task { source } => {
                    {
                        return Some(std::borrow::Borrow::borrow(source));
                    };
                    None
                }
                CliError::Parse { message } => None,
            }
        }
    }
    impl From<DBError> for CliError {
        fn from(source: DBError) -> Self {
            CliError::DB { source }
        }
    }
    impl From<ConfigError> for CliError {
        fn from(source: ConfigError) -> Self {
            CliError::Config { source }
        }
    }
    impl From<io::Error> for CliError {
        fn from(source: io::Error) -> Self {
            CliError::Io { source }
        }
    }
    impl From<Box<dyn Error>> for CliError {
        fn from(source: Box<dyn Error>) -> Self {
            CliError::Wrapped { source }
        }
    }
    impl From<TaskError> for CliError {
        fn from(source: TaskError) -> Self {
            CliError::Task { source }
        }
    }
    impl std::fmt::Display for CliError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                CliError::DB { source } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["Storage error: ", ""],
                        &match (&source.to_string(),) {
                            (arg0,) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt),
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 1usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::Config { source } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["Config parse error : ", ""],
                        &match (&source.to_string(),) {
                            (arg0,) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt),
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 1usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::Io { source } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["IO error: ", ""],
                        &match (&source.to_string(),) {
                            (arg0,) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt),
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 1usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::AppDir { message } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["Application directory locate error: ", ""],
                        &match (&message.to_string(),) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::Cmd { message } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["", ""],
                        &match (&message.to_string(),) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::Unexpected { message } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["Unexpected behavior: ", ""],
                        &match (&message.to_string(),) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::Wrapped { source } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["", ""],
                        &match (&source.to_string(),) {
                            (arg0,) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt),
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 1usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::Task { source } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["Task error: ", ""],
                        &match (&source.to_string(),) {
                            (arg0,) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Debug::fmt),
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 1usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
                CliError::Parse { message } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["Parse error: ", ""],
                        &match (&message.to_string(),) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
            }
        }
    }
    impl CliError {
        pub fn wrap(e: Box<dyn Error>) -> CliError {
            CliError::Wrapped { source: e }
        }
    }
    pub enum TaskError {
        CmdTaskInterval {
            message: String,
            interval: Interval,
            task: Vec<Node>,
        },
        Cmd {
            message: String,
        },
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for TaskError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&TaskError::CmdTaskInterval {
                    message: ref __self_0,
                    interval: ref __self_1,
                    task: ref __self_2,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("CmdTaskInterval");
                    let _ = debug_trait_builder.field("message", &&(*__self_0));
                    let _ = debug_trait_builder.field("interval", &&(*__self_1));
                    let _ = debug_trait_builder.field("task", &&(*__self_2));
                    debug_trait_builder.finish()
                }
                (&TaskError::Cmd {
                    message: ref __self_0,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("Cmd");
                    let _ = debug_trait_builder.field("message", &&(*__self_0));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    impl std::error::Error for TaskError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            #[allow(unused_variables, unreachable_code)]
            match self {
                TaskError::CmdTaskInterval {
                    message,
                    interval,
                    task,
                } => None,
                TaskError::Cmd { message } => None,
            }
        }
    }
    impl std::fmt::Display for TaskError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                TaskError::CmdTaskInterval {
                    message,
                    interval,
                    task,
                } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1(
                        &[""],
                        &match (&({
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["Error: ", ", task: ", ", interval: "],
                                &match (&message, &format_task_name(&task), &interval.to_string()) {
                                    (arg0, arg1, arg2) => [
                                        ::core::fmt::ArgumentV1::new(
                                            arg0,
                                            ::core::fmt::Display::fmt,
                                        ),
                                        ::core::fmt::ArgumentV1::new(
                                            arg1,
                                            ::core::fmt::Display::fmt,
                                        ),
                                        ::core::fmt::ArgumentV1::new(
                                            arg2,
                                            ::core::fmt::Display::fmt,
                                        ),
                                    ],
                                },
                            ));
                            res
                        }),)
                        {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ))?;
                    Ok(())
                }
                TaskError::Cmd { message } => {
                    formatter.write_fmt(::core::fmt::Arguments::new_v1_formatted(
                        &["", ""],
                        &match (&message.to_string(),) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                        &[
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Implied,
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::core::fmt::rt::v1::Argument {
                                position: 0usize,
                                format: ::core::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::core::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::core::fmt::rt::v1::Count::Is(0usize),
                                    width: ::core::fmt::rt::v1::Count::Implied,
                                },
                            },
                        ],
                    ))?;
                    Ok(())
                }
            }
        }
    }
}
mod format {
    use crate::core::Node;
    use chrono::prelude::*;
    use chrono::Duration;
    pub(crate) fn format_task_name(t: &[Node]) -> String {
        t.iter()
            .map(|n| n.label.clone())
            .collect::<Vec<String>>()
            .join(" -> ")
            .to_string()
    }
    pub struct DateTimeOpts {
        pub olways_long: bool,
        pub no_string_now: bool,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for DateTimeOpts {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                DateTimeOpts {
                    olways_long: ref __self_0_0,
                    no_string_now: ref __self_0_1,
                } => {
                    let mut debug_trait_builder = f.debug_struct("DateTimeOpts");
                    let _ = debug_trait_builder.field("olways_long", &&(*__self_0_0));
                    let _ = debug_trait_builder.field("no_string_now", &&(*__self_0_1));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for DateTimeOpts {
        #[inline]
        fn default() -> DateTimeOpts {
            DateTimeOpts {
                olways_long: ::core::default::Default::default(),
                no_string_now: ::core::default::Default::default(),
            }
        }
    }
    pub(crate) fn format_datetime(dt: &DateTime<Utc>) -> String {
        format_datetime_opts(dt, &DateTimeOpts::default())
    }
    pub(crate) fn format_datetime_opts(dt: &DateTime<Utc>, opts: &DateTimeOpts) -> String {
        let dt: DateTime<Local> = DateTime::from(*dt);
        let delta = Local::now() - dt;
        let pattern = if delta < Duration::seconds(2) && !opts.no_string_now {
            "just now"
        } else if dt.date() == Local::today() && !opts.olways_long {
            "%H:%M:%S"
        } else {
            "%Y-%m-%d %H:%M:%S"
        };
        dt.format(pattern).to_string()
    }
    #[allow(clippy::many_single_char_names)]
    pub(crate) fn format_duration(dur: &Duration) -> String {
        let mut res = Vec::new();
        if dur.is_zero() {
            return "".to_string();
        }
        let (w, d, h, m, s) = (
            dur.num_weeks(),
            (*dur - Duration::weeks(dur.num_weeks())).num_days(),
            (*dur - Duration::days(dur.num_days())).num_hours(),
            (*dur - Duration::hours(dur.num_hours())).num_minutes(),
            (*dur - Duration::minutes(dur.num_minutes())).num_seconds(),
        );
        if w > 0 {
            res.push(format_duration_part(w, "week"));
        }
        if d > 0 {
            res.push(format_duration_part(d, "day"));
        }
        if h > 0 {
            res.push(format_duration_part(h, "hour"));
        }
        if m > 0 {
            res.push(format_duration_part(m, "minute"));
        }
        if s > 0 {
            res.push(format_duration_part(s, "second"));
        }
        let res: Vec<&String> = res.iter().take(3).collect();
        match res.len() {
            1 => res.first().unwrap().to_string(),
            2 => {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["", " and "],
                    &match (&res[0], &res[1]) {
                        (arg0, arg1) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                        ],
                    },
                ));
                res
            }
            3 => {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["", ", ", " and "],
                    &match (&res[0], &res[1], &res[2]) {
                        (arg0, arg1, arg2) => [
                            ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                            ::core::fmt::ArgumentV1::new(arg2, ::core::fmt::Display::fmt),
                        ],
                    },
                ));
                res
            }
            _ => "".to_string(),
        }
    }
    fn format_duration_part(p: i64, w: &str) -> String {
        let mut s = {
            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                &["", " "],
                &match (&p, &w) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                    ],
                },
            ));
            res
        };
        if p > 1 {
            s.push('s');
        }
        s
    }
}
mod history {
    use crate::core::{DBRoot, Interval, Node};
    use chrono::Utc;
    use std::rc::Rc;
    use uuid::Uuid;
    use yatt_orm::statement::Statement;
    use yatt_orm::{
        BoxStorage, DBError, DBResult, HistoryRecord, HistoryRecordType, HistoryStorage, Storage,
    };
    pub(crate) struct DBWatcher {
        db: Box<dyn DBRoot>,
        history_storage: Rc<dyn HistoryStorage>,
    }
    pub(crate) trait LocalUnique {
        fn get_local_id(&self) -> usize;
    }
    impl DBWatcher {
        pub fn new(db: Box<dyn DBRoot>, history_storage: Rc<dyn HistoryStorage>) -> Self {
            DBWatcher {
                db,
                history_storage,
            }
        }
    }
    impl DBRoot for DBWatcher {
        fn nodes(&self) -> BoxStorage<Node> {
            Box::new(StorageWatcher::new(
                "nodes",
                self.db.nodes(),
                Rc::clone(&self.history_storage),
            ))
        }
        fn intervals(&self) -> BoxStorage<Interval> {
            Box::new(StorageWatcher::new(
                "intervals",
                self.db.intervals(),
                Rc::clone(&self.history_storage),
            ))
        }
    }
    struct StorageWatcher<T: LocalUnique> {
        entity_type: &'static str,
        storage: BoxStorage<T>,
        history_storage: Rc<dyn HistoryStorage>,
    }
    impl<T: LocalUnique> StorageWatcher<T> {
        fn new(
            entity_type: &'static str,
            storage: BoxStorage<T>,
            history_storage: Rc<dyn HistoryStorage>,
        ) -> Self {
            StorageWatcher {
                entity_type,
                storage,
                history_storage,
            }
        }
    }
    impl<T: LocalUnique> Storage for StorageWatcher<T> {
        type Item = T;
        fn save(&self, item: &Self::Item) -> DBResult<usize> {
            let entity_id = self.storage.save(item)?;
            let uid = self
                .history_storage
                .get_entity_guid(item.get_local_id(), self.entity_type);
            let (uid, is_new) = match uid {
                Ok(uid) => (uid, false),
                Err(e) => {
                    if let DBError::IsEmpty { message: _ } = e {
                        (Uuid::new_v4(), true)
                    } else {
                        return Err(e);
                    }
                }
            };
            let record_type = if is_new {
                HistoryRecordType::Create
            } else {
                HistoryRecordType::Update
            };
            self.history_storage.push_record(HistoryRecord {
                date: Utc::now(),
                uuid: uid,
                record_type,
                entity_type: self.entity_type.to_string(),
                entity_id,
            })?;
            Ok(entity_id)
        }
        fn all(&self) -> DBResult<Vec<Self::Item>> {
            self.storage.all()
        }
        fn remove(&self, id: usize) -> DBResult<()> {
            self.storage.remove(id)?;
            let uid = self.history_storage.get_entity_guid(id, self.entity_type)?;
            self.history_storage.push_record(HistoryRecord {
                date: Utc::now(),
                uuid: uid,
                record_type: HistoryRecordType::Delete,
                entity_type: self.entity_type.to_string(),
                entity_id: id,
            })
        }
        fn by_statement(&self, s: Statement) -> DBResult<Vec<Self::Item>> {
            self.storage.by_statement(s)
        }
    }
}
mod history_storage {
    pub mod sqlite {
        use rusqlite::{params, Connection, Result as SQLITEResult, NO_PARAMS};
        use std::convert::TryFrom;
        use std::path::Path;
        use std::rc::Rc;
        use uuid::Uuid;
        use yatt_orm::{DBError, DBResult, HistoryRecord, HistoryStorage};
        pub(crate) struct DB {
            con: Rc<Connection>,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for DB {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match *self {
                    DB {
                        con: ref __self_0_0,
                    } => {
                        let mut debug_trait_builder = f.debug_struct("DB");
                        let _ = debug_trait_builder.field("con", &&(*__self_0_0));
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl DB {
            pub fn new<P: AsRef<Path>>(path: P) -> DBResult<DB> {
                let exists = path.as_ref().exists();
                let con = Connection::open(path).map_err(|s| DBError::wrap(Box::new(s)))?;
                let res = DB { con: Rc::new(con) };
                if !exists {
                    res.init().map_err(|s| DBError::wrap(Box::new(s)))?;
                }
                Ok(res)
            }
            fn init(&self) -> SQLITEResult<()> {
                self.con.execute(
                    "create table history_records (
            date INTEGER NOT NULL,
            uuid TEXT NOT NULL,
            record_type TEXT NOT NULL,
            entyty_type INTEGER NOT NULL,
            entity_id INTEGER NOT NULL
            )",
                    NO_PARAMS,
                )?;
                Ok(())
            }
        }
        impl HistoryStorage for DB {
            fn push_record(&self, r: HistoryRecord) -> DBResult<()> {
                self.con
                    .execute(
                        "insert into history_records (
                date,
                uuid,
                record_type,
                entity_type,
                entity_id
        ) values (?1, ?2, ?3, ?4, ?5)",
                        &[
                            &r.date as &dyn::rusqlite::ToSql,
                            &r.uuid.to_string() as &dyn::rusqlite::ToSql,
                            &isize::from(r.record_type) as &dyn::rusqlite::ToSql,
                            &r.entity_type as &dyn::rusqlite::ToSql,
                            &isize::try_from(r.entity_id).unwrap() as &dyn::rusqlite::ToSql,
                        ],
                    )
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(())
            }
            fn get_entity_guid(&self, id: usize, entity_type: &str) -> DBResult<Uuid> {
                match self
                    .con
                    .prepare(
                        "select uuid from history_records
                where entity_id = ?1 and entity_type = ?2 limit 1",
                    )
                    .map_err(|s| DBError::wrap(Box::new(s)))?
                    .query(&[
                        &isize::try_from(id).unwrap() as &dyn::rusqlite::ToSql,
                        &entity_type as &dyn::rusqlite::ToSql,
                    ])
                    .map_err(|s| DBError::wrap(Box::new(s)))?
                    .next()
                    .map_err(|s| DBError::wrap(Box::new(s)))?
                {
                    Some(row) => {
                        let str_row: String = row.get(0).map_err(|s| DBError::wrap(Box::new(s)))?;
                        Ok(Uuid::parse_str(str_row.as_str())
                            .map_err(|s| DBError::wrap(Box::new(s)))?)
                    }
                    None => Err(DBError::IsEmpty {
                        message: {
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["no entity found for id=", " and entity_type="],
                                &match (&id, &entity_type) {
                                    (arg0, arg1) => [
                                        ::core::fmt::ArgumentV1::new(
                                            arg0,
                                            ::core::fmt::Display::fmt,
                                        ),
                                        ::core::fmt::ArgumentV1::new(
                                            arg1,
                                            ::core::fmt::Display::fmt,
                                        ),
                                    ],
                                },
                            ));
                            res
                        },
                    }),
                }
            }
        }
    }
}
mod parse {
    use super::*;
    use chrono::prelude::*;
    use chrono::Duration;
    use regex::*;
    use std::convert::{TryFrom, TryInto};
    pub struct PeriodOpts {
        pub week_starts_from_sunday: bool,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for PeriodOpts {
        #[inline]
        fn default() -> PeriodOpts {
            PeriodOpts {
                week_starts_from_sunday: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for PeriodOpts {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                PeriodOpts {
                    week_starts_from_sunday: ref __self_0_0,
                } => {
                    let mut debug_trait_builder = f.debug_struct("PeriodOpts");
                    let _ = debug_trait_builder.field("week_starts_from_sunday", &&(*__self_0_0));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    pub fn parse_period(s: &str, opts: &PeriodOpts) -> CliResult<(DateTime<Utc>, DateTime<Utc>)> {
        let parts: Vec<&str> = s.split("::").collect();
        match parts.len() {
            1 => {
                let d = try_parse_date_time(parts[0]);
                if d.is_ok() {
                    return Ok((d.unwrap(), Utc::now()));
                }
                try_parse_period(parts[0], opts)
            }
            2 => {
                let mut end = try_parse_date_time(parts[1])?;
                if end.date().and_hms(0, 0, 0) == end {
                    end = end + Duration::days(1);
                }
                Ok((try_parse_date_time(parts[0])?, end))
            }
            _ => Err(CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse period from string \""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            }),
        }
    }
    fn try_parse_date_time(s: &str) -> CliResult<DateTime<Utc>> {
        let parts: Vec<&str> = s.split(' ').collect();
        match parts.len() {
            1 => {
                let d = try_parse_date_part(s);
                if d.is_ok() {
                    return Ok(d.unwrap().and_hms(0, 0, 0).into());
                };
                try_parse_time_part(s, Local::today())
            }
            2 => {
                let d = try_parse_date_part(parts[0])?;
                try_parse_time_part(parts[1], d)
            }
            _ => Err(CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse date and time from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            }),
        }
    }
    fn try_parse_date_part(s: &str) -> CliResult<Date<Local>> {
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        struct RE_PARSE_DATE_PART {
            __private_field: (),
        }
        #[doc(hidden)]
        static RE_PARSE_DATE_PART: RE_PARSE_DATE_PART = RE_PARSE_DATE_PART {
            __private_field: (),
        };
        impl ::lazy_static::__Deref for RE_PARSE_DATE_PART {
            type Target = Regex;
            fn deref(&self) -> &Regex {
                #[inline(always)]
                fn __static_ref_initialize() -> Regex {
                    Regex :: new ( r"^((?P<y>\d{4})-)?(?P<m>\d{1,2})-(?P<d>\d{1,2})|(?P<dr>\d{1,2})\.(?P<mr>\d{1,2})(\.(?P<yr>\d{4}))?$" ) . unwrap ( )
                }
                #[inline(always)]
                fn __stability() -> &'static Regex {
                    static LAZY: ::lazy_static::lazy::Lazy<Regex> = ::lazy_static::lazy::Lazy::INIT;
                    LAZY.get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl ::lazy_static::LazyStatic for RE_PARSE_DATE_PART {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
        let caps = RE_PARSE_DATE_PART.captures(s);
        if caps.is_none() {
            return Err(CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse date from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            });
        }
        let caps = caps.unwrap();
        let day: u32 = caps
            .name("d")
            .unwrap_or_else(|| caps.name("dr").unwrap())
            .as_str()
            .parse()
            .map_err(|_| CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse date from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            })?;
        let month: u32 = caps
            .name("m")
            .unwrap_or_else(|| caps.name("mr").unwrap())
            .as_str()
            .parse()
            .map_err(|_| CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse date from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            })?;
        let year: i32 = if let Some(y) = caps.name("y") {
            y.as_str().parse().map_err(|_| CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse date from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            })?
        } else if let Some(y) = caps.name("yr") {
            y.as_str().parse().map_err(|_| CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse date from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            })?
        } else {
            Local::today().year()
        };
        Ok(Local.ymd(year, month, day))
    }
    fn try_parse_time_part(s: &str, d: Date<Local>) -> CliResult<DateTime<Utc>> {
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        struct RE_PARSE_TIME_PART {
            __private_field: (),
        }
        #[doc(hidden)]
        static RE_PARSE_TIME_PART: RE_PARSE_TIME_PART = RE_PARSE_TIME_PART {
            __private_field: (),
        };
        impl ::lazy_static::__Deref for RE_PARSE_TIME_PART {
            type Target = Regex;
            fn deref(&self) -> &Regex {
                #[inline(always)]
                fn __static_ref_initialize() -> Regex {
                    Regex::new(r"^(?P<h>\d{1,2}):(?P<m>\d{1,2})(:(?P<s>\d{1,2}))?$").unwrap()
                }
                #[inline(always)]
                fn __stability() -> &'static Regex {
                    static LAZY: ::lazy_static::lazy::Lazy<Regex> = ::lazy_static::lazy::Lazy::INIT;
                    LAZY.get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl ::lazy_static::LazyStatic for RE_PARSE_TIME_PART {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
        let caps = RE_PARSE_TIME_PART.captures(s);
        if caps.is_none() {
            return Err(CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse time from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            });
        }
        let caps = caps.unwrap();
        let hour: u32 = caps["h"].parse().map_err(|_| CliError::Parse {
            message: {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["can\'t parse time from string \"", "\""],
                    &match (&s,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            },
        })?;
        let minute: u32 = caps["m"].parse().map_err(|_| CliError::Parse {
            message: {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["can\'t parse time from string \"", "\""],
                    &match (&s,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            },
        })?;
        let second: u32 = if let Some(sec) = caps.name("s") {
            sec.as_str().parse().map_err(|_| CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse time from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            })?
        } else {
            0
        };
        Ok(d.and_hms(hour, minute, second).into())
    }
    fn try_parse_period(s: &str, opts: &PeriodOpts) -> CliResult<(DateTime<Utc>, DateTime<Utc>)> {
        if s.starts_with('l') && s.len() < 2 {
            return Err(CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse last from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            });
        }
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        struct RE_PARSE_DURATION {
            __private_field: (),
        }
        #[doc(hidden)]
        static RE_PARSE_DURATION: RE_PARSE_DURATION = RE_PARSE_DURATION {
            __private_field: (),
        };
        impl ::lazy_static::__Deref for RE_PARSE_DURATION {
            type Target = Regex;
            fn deref(&self) -> &Regex {
                #[inline(always)]
                fn __static_ref_initialize() -> Regex {
                    Regex::new(r"^(?P<o>[lp])?(?P<n>\d+)?(?P<p>[ymwdh])$").unwrap()
                }
                #[inline(always)]
                fn __stability() -> &'static Regex {
                    static LAZY: ::lazy_static::lazy::Lazy<Regex> = ::lazy_static::lazy::Lazy::INIT;
                    LAZY.get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl ::lazy_static::LazyStatic for RE_PARSE_DURATION {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
        let caps = RE_PARSE_DURATION.captures(s);
        if caps.is_none() {
            return Err(CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse last from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            });
        }
        let caps = caps.unwrap();
        let o: u32 = if let Some(o) = caps.name("o") {
            if o.as_str() == "p" {
                1
            } else {
                0
            }
        } else {
            0
        };
        let p = &caps["p"];
        let n: u32 = if let Some(n) = caps.name("n") {
            n.as_str().parse().map_err(|_| CliError::Parse {
                message: {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["can\'t parse last from string \"", "\""],
                        &match (&s,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                },
            })?
        } else {
            1
        };
        let now = Local::now();
        let (begin, end) = match p {
            "y" => (
                Local
                    .ymd(
                        now.year()
                            - i32::try_from(o + n - 1).map_err(|_| CliError::Parse {
                                message: {
                                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                        &["can\'t parse last from string \"", "\""],
                                        &match (&s,) {
                                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                                arg0,
                                                ::core::fmt::Display::fmt,
                                            )],
                                        },
                                    ));
                                    res
                                },
                            })?,
                        1,
                        1,
                    )
                    .and_hms(0, 0, 0),
                if o == 1 {
                    Local.ymd(now.year(), 1, 1).and_hms(0, 0, 0)
                } else {
                    now
                },
            ),
            "m" => {
                let mo = o + n - 1;
                let mut yo = mo / 12;
                let mut mo = mo - yo * 12;
                let mut month = if mo > now.month() {
                    yo += 1;
                    mo -= now.month();
                    12 - mo
                } else {
                    now.month() - mo
                };
                let mut year = now.year()
                    - i32::try_from(yo).map_err(|_| CliError::Parse {
                        message: {
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["can\'t parse last from string \"", "\""],
                                &match (&s,) {
                                    (arg0,) => [::core::fmt::ArgumentV1::new(
                                        arg0,
                                        ::core::fmt::Display::fmt,
                                    )],
                                },
                            ));
                            res
                        },
                    })?;
                if month == 0 {
                    month = 12;
                    year -= 1;
                };
                (
                    Local.ymd(year, month, 1).and_hms(0, 0, 0),
                    if o == 1 {
                        Local.ymd(now.year(), now.month(), 1).and_hms(0, 0, 0)
                    } else {
                        now
                    },
                )
            }
            "w" => {
                let first_dow = (if opts.week_starts_from_sunday {
                    now - Duration::days(now.weekday().number_from_sunday().try_into().unwrap())
                } else {
                    now - Duration::days(now.weekday().number_from_monday().try_into().unwrap())
                })
                .date()
                    + Duration::days(1);
                let wo = o + n - 1;
                (
                    (first_dow - Duration::weeks(wo.try_into().unwrap())).and_hms(0, 0, 0),
                    if o == 1 {
                        first_dow.and_hms(0, 0, 0)
                    } else {
                        now
                    },
                )
            }
            "d" => {
                let today = Local::today();
                let dayo = o + n - 1;
                (
                    (today - Duration::days(dayo.try_into().unwrap())).and_hms(0, 0, 0),
                    if o == 1 { today.and_hms(0, 0, 0) } else { now },
                )
            }
            "h" => {
                let hour = Local::today().and_hms(now.hour(), 0, 0);
                let ho = o + n - 1;
                (
                    (hour - Duration::hours(ho.try_into().unwrap())),
                    if o == 1 { hour } else { now },
                )
            }
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        };
        Ok((begin.into(), end.into()))
    }
}
mod print {
    use self::core::*;
    use super::*;
    use crate::report::*;
    const DEFAULT_INTERVAL_INFO_TITLE: &str = "Interval info:";
    pub struct IntervalData<'a> {
        pub interval: &'a Interval,
        pub task: &'a [Node],
        pub title: &'a str,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<'a> ::core::fmt::Debug for IntervalData<'a> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                IntervalData {
                    interval: ref __self_0_0,
                    task: ref __self_0_1,
                    title: ref __self_0_2,
                } => {
                    let mut debug_trait_builder = f.debug_struct("IntervalData");
                    let _ = debug_trait_builder.field("interval", &&(*__self_0_0));
                    let _ = debug_trait_builder.field("task", &&(*__self_0_1));
                    let _ = debug_trait_builder.field("title", &&(*__self_0_2));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<'a> ::core::clone::Clone for IntervalData<'a> {
        #[inline]
        fn clone(&self) -> IntervalData<'a> {
            match *self {
                IntervalData {
                    interval: ref __self_0_0,
                    task: ref __self_0_1,
                    title: ref __self_0_2,
                } => IntervalData {
                    interval: ::core::clone::Clone::clone(&(*__self_0_0)),
                    task: ::core::clone::Clone::clone(&(*__self_0_1)),
                    title: ::core::clone::Clone::clone(&(*__self_0_2)),
                },
            }
        }
    }
    impl IntervalData<'_> {
        pub fn default_title() -> &'static str {
            DEFAULT_INTERVAL_INFO_TITLE
        }
    }
    impl ToString for IntervalData<'_> {
        fn to_string(&self) -> String {
            {
                ::std::rt::begin_panic("not implemented")
            }
        }
    }
    pub struct IntervalCmdData<'a> {
        pub cmd_text: &'a str,
        pub interval: IntervalData<'a>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<'a> ::core::fmt::Debug for IntervalCmdData<'a> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                IntervalCmdData {
                    cmd_text: ref __self_0_0,
                    interval: ref __self_0_1,
                } => {
                    let mut debug_trait_builder = f.debug_struct("IntervalCmdData");
                    let _ = debug_trait_builder.field("cmd_text", &&(*__self_0_0));
                    let _ = debug_trait_builder.field("interval", &&(*__self_0_1));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<'a> ::core::clone::Clone for IntervalCmdData<'a> {
        #[inline]
        fn clone(&self) -> IntervalCmdData<'a> {
            match *self {
                IntervalCmdData {
                    cmd_text: ref __self_0_0,
                    interval: ref __self_0_1,
                } => IntervalCmdData {
                    cmd_text: ::core::clone::Clone::clone(&(*__self_0_0)),
                    interval: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    pub struct IntervalError<'a> {
        pub err_text: &'a str,
        pub interval: IntervalData<'a>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<'a> ::core::fmt::Debug for IntervalError<'a> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                IntervalError {
                    err_text: ref __self_0_0,
                    interval: ref __self_0_1,
                } => {
                    let mut debug_trait_builder = f.debug_struct("IntervalError");
                    let _ = debug_trait_builder.field("err_text", &&(*__self_0_0));
                    let _ = debug_trait_builder.field("interval", &&(*__self_0_1));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl<'a> ::core::clone::Clone for IntervalError<'a> {
        #[inline]
        fn clone(&self) -> IntervalError<'a> {
            match *self {
                IntervalError {
                    err_text: ref __self_0_0,
                    interval: ref __self_0_1,
                } => IntervalError {
                    err_text: ::core::clone::Clone::clone(&(*__self_0_0)),
                    interval: ::core::clone::Clone::clone(&(*__self_0_1)),
                },
            }
        }
    }
    pub trait Printer {
        fn interval_cmd(&self, d: &IntervalCmdData);
        fn error(&self, e: &str);
        fn interval_error(&self, d: &IntervalData, e: &str);
        fn cmd(&self, d: &str);
        fn report(&self, r: &Report);
        fn prompt(&self, p: &str);
    }
    pub trait Markdown {
        fn markdown(&self) -> String;
    }
    pub struct TermPrinter {
        style: AppStyle,
    }
    impl Default for TermPrinter {
        fn default() -> Self {
            TermPrinter {
                style: Default::default(),
            }
        }
    }
    impl Printer for TermPrinter {
        fn interval_cmd(&self, d: &IntervalCmdData) {
            self.cmd(d.cmd_text);
            ::std::io::_print(::core::fmt::Arguments::new_v1(
                &["\n"],
                &match () {
                    () => [],
                },
            ));
            print_interval_info(&d.interval, &self.style.task);
        }
        fn error(&self, e: &str) {
            {
                ::std::io::_print(::core::fmt::Arguments::new_v1(
                    &["Error: ", "\n"],
                    &match (&&self.style.error.apply_to(e),) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
            };
        }
        fn interval_error(&self, d: &IntervalData, e: &str) {
            self.error(e);
            ::std::io::_print(::core::fmt::Arguments::new_v1(
                &["\n"],
                &match () {
                    () => [],
                },
            ));
            print_interval_info(d, &self.style.task);
        }
        fn cmd(&self, d: &str) {
            {
                ::std::io::_print(::core::fmt::Arguments::new_v1(
                    &["", "\n"],
                    &match (&&self.style.cmd.apply_to(d),) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
            };
        }
        fn report(&self, r: &Report) {
            {
                ::std::io::_print(::core::fmt::Arguments::new_v1(
                    &["", "\n"],
                    &match (&self
                        .style
                        .report
                        .text(&r.markdown(), self.style.screen_width),)
                    {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
            };
        }
        fn prompt(&self, p: &str) {
            {
                ::std::io::_print(::core::fmt::Arguments::new_v1(
                    &["", "\n"],
                    &match (&p,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
            };
        }
    }
    fn print_interval_info(d: &IntervalData, s: &TaskStyle) {
        {
            ::std::io::_print(::core::fmt::Arguments::new_v1(
                &["", "\n"],
                &match (&d.title,) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            ));
        };
        ::std::io::_print(::core::fmt::Arguments::new_v1(
            &["  Task: "],
            &match () {
                () => [],
            },
        ));
        for (i, t) in d.task.iter().enumerate() {
            ::std::io::_print(::core::fmt::Arguments::new_v1(
                &[""],
                &match (&s.name.apply_to(&t.label),) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            ));
            if i < d.task.len() - 1 {
                ::std::io::_print(::core::fmt::Arguments::new_v1(
                    &[" > "],
                    &match () {
                        () => [],
                    },
                ));
            }
        }
        ::std::io::_print(::core::fmt::Arguments::new_v1(
            &["\n"],
            &match () {
                () => [],
            },
        ));
        ::std::io::_print(::core::fmt::Arguments::new_v1(
            &["  Started: "],
            &match (&s.start_time.apply_to(format_datetime(&d.interval.begin)),) {
                (arg0,) => [::core::fmt::ArgumentV1::new(
                    arg0,
                    ::core::fmt::Display::fmt,
                )],
            },
        ));
        let dur = Utc::now() - d.interval.begin;
        if dur.num_seconds() > 2 {
            ::std::io::_print(::core::fmt::Arguments::new_v1(
                &[" (", " ago)"],
                &match (&s.time_span.apply_to(format_duration(&dur)),) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            ));
        }
        if d.interval.end.is_some() {
            let e = d.interval.end.unwrap();
            ::std::io::_print(::core::fmt::Arguments::new_v1(
                &["\n  Stopped: "],
                &match (&s.end_time.apply_to(format_datetime(&e)),) {
                    (arg0,) => [::core::fmt::ArgumentV1::new(
                        arg0,
                        ::core::fmt::Display::fmt,
                    )],
                },
            ));
            let dur = Utc::now() - e;
            if dur.num_seconds() > 2 {
                ::std::io::_print(::core::fmt::Arguments::new_v1(
                    &[" (", " ago)"],
                    &match (&s.time_span.apply_to(format_duration(&dur)),) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
            }
        }
        ::std::io::_print(::core::fmt::Arguments::new_v1(
            &["\n"],
            &match () {
                () => [],
            },
        ));
    }
}
mod report {
    use crate::format::*;
    use crate::print::Markdown;
    use chrono::prelude::*;
    use chrono::Duration;
    pub struct Report {
        rows: Vec<Row>,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::default::Default for Report {
        #[inline]
        fn default() -> Report {
            Report {
                rows: ::core::default::Default::default(),
            }
        }
    }
    impl Report {
        pub fn new() -> Self {
            Report {
                rows: <[_]>::into_vec(box []),
            }
        }
        pub fn push(&mut self, r: impl Into<Row>) {
            self.rows.push(r.into());
        }
        pub fn rows(&self) -> &Vec<Row> {
            &self.rows
        }
    }
    pub enum Row {
        Header(String),
        Interval(DateTime<Utc>, DateTime<Utc>),
        Table(Vec<Cell>),
        TableHeader(Vec<String>),
        SubTotal(Vec<Cell>),
        Total(Vec<Cell>),
        Nested(Vec<Cell>),
        Span,
    }
    impl From<String> for Row {
        fn from(v: String) -> Self {
            Row::Header(v)
        }
    }
    impl From<&str> for Row {
        fn from(v: &str) -> Self {
            Row::Header(v.into())
        }
    }
    impl From<(DateTime<Utc>, DateTime<Utc>)> for Row {
        fn from(v: (DateTime<Utc>, DateTime<Utc>)) -> Self {
            Row::Interval(v.0, v.1)
        }
    }
    impl From<Vec<Cell>> for Row {
        fn from(v: Vec<Cell>) -> Self {
            Row::Table(v)
        }
    }
    impl From<Vec<String>> for Row {
        fn from(v: Vec<String>) -> Self {
            Row::TableHeader(v)
        }
    }
    pub enum Cell {
        Usize(usize),
        Isize(isize),
        String(String),
        DateTime(DateTime<Utc>),
        Duration(Duration),
        Nested(Box<Cell>, usize),
        Span,
    }
    impl From<usize> for Cell {
        fn from(v: usize) -> Self {
            Cell::Usize(v)
        }
    }
    impl From<isize> for Cell {
        fn from(v: isize) -> Self {
            Cell::Isize(v)
        }
    }
    impl From<String> for Cell {
        fn from(v: String) -> Self {
            Cell::String(v)
        }
    }
    impl From<&str> for Cell {
        fn from(v: &str) -> Self {
            Cell::String(v.into())
        }
    }
    impl From<DateTime<Utc>> for Cell {
        fn from(v: DateTime<Utc>) -> Self {
            Cell::DateTime(v)
        }
    }
    impl From<Duration> for Cell {
        fn from(v: Duration) -> Self {
            Cell::Duration(v)
        }
    }
    impl Markdown for Report {
        fn markdown(&self) -> String {
            self.rows()
                .iter()
                .map(|r| r.markdown())
                .collect::<Vec<String>>()
                .join("\n")
        }
    }
    impl Markdown for Row {
        fn markdown(&self) -> String {
            match self {
                Row::Header(v) => {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["Report: **", "**"],
                        &match (&v,) {
                            (arg0,) => [::core::fmt::ArgumentV1::new(
                                arg0,
                                ::core::fmt::Display::fmt,
                            )],
                        },
                    ));
                    res
                }
                Row::Interval(b, e) => {
                    let dtopts = DateTimeOpts {
                        olways_long: true,
                        no_string_now: true,
                    };
                    {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["Period: *", " - ", "*"],
                            &match (
                                &format_datetime_opts(b, &dtopts),
                                &format_datetime_opts(e, &dtopts),
                            ) {
                                (arg0, arg1) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    }
                }
                Row::Table(v) => format_cells(&v),
                Row::TableHeader(v) => format_header(&v),
                Row::SubTotal(v) => format_subtotal(&v),
                Row::Total(v) => format_total(&v),
                Row::Nested(v) => v
                    .iter()
                    .map(|c| {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["|"],
                            &match (&c.markdown(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    })
                    .collect::<Vec<String>>()
                    .join(""),
                Row::Span => "|-".to_string(),
            }
        }
    }
    fn format_subtotal(cells: &[Cell]) -> String {
        let mut aligns = "|".to_string();
        let mut cols = "|".to_string();
        for c in cells {
            aligns += "|";
            cols += &{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["|*", "*"],
                    &match (&c.markdown(),) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            };
        }
        {
            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                &["", "\n"],
                &match (&aligns, &cols) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                    ],
                },
            ));
            res
        }
    }
    fn format_total(cells: &[Cell]) -> String {
        let mut aligns = "|-:".to_string();
        let mut cols = "|Total".to_string();
        for c in cells {
            aligns += match c {
                Cell::String(_) | Cell::Duration(_) | Cell::DateTime(_) | Cell::Nested(_, _) => {
                    "|-"
                }
                _ => "|-:",
            };
            cols += &{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["|**", "**\n|-"],
                    &match (&c.markdown(),) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            };
        }
        {
            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                &["", "\n"],
                &match (&aligns, &cols) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                    ],
                },
            ));
            res
        }
    }
    fn format_header(cells: &[String]) -> String {
        let mut aligns = String::new();
        let mut cols = String::new();
        for c in cells {
            aligns += "|:-:";
            cols += &{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["|"],
                    &match (&c,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            };
        }
        {
            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                &["", "\n"],
                &match (&aligns, &cols) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                    ],
                },
            ));
            res
        }
    }
    fn format_cells(cells: &[Cell]) -> String {
        let mut aligns = String::new();
        let mut cols = String::new();
        for c in cells {
            aligns += match c {
                Cell::String(_) | Cell::Duration(_) | Cell::DateTime(_) | Cell::Nested(_, _) => {
                    "|-"
                }
                _ => "|-:",
            };
            cols += &{
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["|"],
                    &match (&c.markdown(),) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            };
        }
        {
            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                &["", "\n"],
                &match (&aligns, &cols) {
                    (arg0, arg1) => [
                        ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                        ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                    ],
                },
            ));
            res
        }
    }
    impl Markdown for Cell {
        fn markdown(&self) -> String {
            match self {
                Cell::Usize(v) => v.to_string(),
                Cell::Isize(v) => v.to_string(),
                Cell::String(v) => v.to_owned(),
                Cell::DateTime(v) => format_datetime(&v),
                Cell::Duration(v) => format_duration(&v),
                Cell::Nested(v, p) => {
                    let mut pad = "".to_string();
                    let mark = match p {
                        0 => "",
                        1 => "\u{2023}",
                        _ => "\u{2219}",
                    };
                    for _ in 0..*p {
                        pad += "  ";
                    }
                    {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["`", "", " ", "`"],
                            &match (&pad, &mark, &v.markdown()) {
                                (arg0, arg1, arg2) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg2, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    }
                }
                Cell::Span => "".to_string(),
            }
        }
    }
}
mod storage {
    pub mod sqlite {
        use crate::core::*;
        use chrono::prelude::*;
        use rusqlite::{params, Connection, Result as SQLITEResult, NO_PARAMS};
        use std::convert::TryFrom;
        use std::path::Path;
        use std::rc::Rc;
        use yatt_orm::errors::*;
        use yatt_orm::statement::*;
        use yatt_orm::*;
        pub struct DB {
            con: Rc<Connection>,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for DB {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match *self {
                    DB {
                        con: ref __self_0_0,
                    } => {
                        let mut debug_trait_builder = f.debug_struct("DB");
                        let _ = debug_trait_builder.field("con", &&(*__self_0_0));
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl DB {
            pub fn new<P: AsRef<Path>>(path: P) -> DBResult<DB> {
                let exists = path.as_ref().exists();
                let con = Connection::open(path).map_err(|s| DBError::wrap(Box::new(s)))?;
                let res = DB { con: Rc::new(con) };
                if !exists {
                    res.init().map_err(|s| DBError::wrap(Box::new(s)))?;
                }
                Ok(res)
            }
            fn init(&self) -> SQLITEResult<()> {
                self.con.execute(
                    "create table nodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            label TEXT NOT NULL,
            parent_id INTEGER,
            created INTEGER NOT NULL,
            closed INTEGER DEFAULT 0,
            deleted integer default 0
            )",
                    NO_PARAMS,
                )?;
                self.con.execute(
                    "create table intervals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id integer,
             begin integer NOT NULL,
             end integer,
             deleted integer default 0,
             closed integer default 0
             )",
                    NO_PARAMS,
                )?;
                Ok(())
            }
        }
        impl DBRoot for DB {
            fn nodes(&self) -> BoxStorage<Node> {
                Box::new(Nodes::new(Rc::clone(&self.con)))
            }
            fn intervals(&self) -> BoxStorage<Interval> {
                Box::new(Intervals::new(Rc::clone(&self.con)))
            }
        }
        pub struct Nodes {
            con: Rc<Connection>,
        }
        impl Nodes {
            pub fn new(con: Rc<Connection>) -> Nodes {
                Nodes { con }
            }
            fn select(&self, select_str: &str, where_str: &str) -> SQLITEResult<Vec<Node>> {
                let sql = {
                    let res = :: alloc :: fmt :: format ( :: core :: fmt :: Arguments :: new_v1 ( & [ "" , "\n        id,\n        parent_id,\n        label,\n        closed,\n        created,\n        deleted\n            from nodes " ] , & match ( & select_str , & where_str ) { ( arg0 , arg1 ) => [ :: core :: fmt :: ArgumentV1 :: new ( arg0 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg1 , :: core :: fmt :: Display :: fmt ) ] , } ) ) ;
                    res
                };
                let mut stmt = self.con.prepare(&sql)?;
                let mut rows = stmt.query(NO_PARAMS)?;
                let mut res = Vec::new();
                while let Some(row) = rows.next()? {
                    let id: isize = row.get(0)?;
                    let id = usize::try_from(id).unwrap();
                    let parent_id: Option<isize> = row.get(1)?;
                    let parent_id = match parent_id {
                        Some(v) => Some(usize::try_from(v).unwrap()),
                        None => None,
                    };
                    res.push(Node {
                        id,
                        parent_id,
                        label: row.get(2)?,
                        closed: row.get(3)?,
                        created: row.get(4)?,
                        deleted: row.get(5)?,
                    })
                }
                SQLITEResult::Ok(res)
            }
        }
        impl Storage for Nodes {
            type Item = Node;
            fn save(&self, node: &Node) -> DBResult<usize> {
                let parent_id = node.parent_id.map(|r| isize::try_from(r).unwrap());
                if node.id > 0 {
                    let id = isize::try_from(node.id).unwrap();
                    self.con
                        .execute(
                            "update nodes
                set label = ?1,
                closed = ?2,
                parent_id = ?3,
                deleted = ?4
                where id = ?5",
                            &[
                                &node.label as &dyn::rusqlite::ToSql,
                                &node.closed as &dyn::rusqlite::ToSql,
                                &parent_id as &dyn::rusqlite::ToSql,
                                &id as &dyn::rusqlite::ToSql,
                                &node.deleted as &dyn::rusqlite::ToSql,
                            ],
                        )
                        .map_err(|s| DBError::wrap(Box::new(s)))?;
                    return Ok(node.id);
                };
                self.con
                    .execute(
                        "insert into nodes (
                        label,
                        parent_id,
                        created, 
                        deleted) values (?1, ?2, ?3, ?4)",
                        &[
                            &node.label as &dyn::rusqlite::ToSql,
                            &parent_id as &dyn::rusqlite::ToSql,
                            &Utc::now() as &dyn::rusqlite::ToSql,
                            &node.deleted as &dyn::rusqlite::ToSql,
                        ],
                    )
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(usize::try_from(self.con.last_insert_rowid()).unwrap())
            }
            fn all(&self) -> DBResult<Vec<Self::Item>> {
                let res = self
                    .select("select", "where deleted = 0")
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(res)
            }
            fn remove(&self, id: usize) -> DBResult<()> {
                let id = isize::try_from(id).unwrap();
                self.con
                    .execute(
                        "update nodes set deleted = 1 where id = ?1",
                        &[&id as &dyn::rusqlite::ToSql],
                    )
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(())
            }
            fn by_statement(&self, statement: Statement) -> DBResult<Vec<Self::Item>> {
                let res = self
                    .select(&statement.build_select(), &statement.build_where())
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(res)
            }
        }
        pub struct Intervals {
            con: Rc<Connection>,
        }
        impl Intervals {
            pub fn new(con: Rc<Connection>) -> Intervals {
                Intervals { con }
            }
            fn select(&self, select_str: &str, where_str: &str) -> SQLITEResult<Vec<Interval>> {
                let sql = {
                    let res = :: alloc :: fmt :: format ( :: core :: fmt :: Arguments :: new_v1 ( & [ "" , "\n        id,\n        node_id,\n        begin,\n        end,\n        deleted,\n        closed\n            from intervals " ] , & match ( & select_str , & where_str ) { ( arg0 , arg1 ) => [ :: core :: fmt :: ArgumentV1 :: new ( arg0 , :: core :: fmt :: Display :: fmt ) , :: core :: fmt :: ArgumentV1 :: new ( arg1 , :: core :: fmt :: Display :: fmt ) ] , } ) ) ;
                    res
                };
                let mut stmt = self.con.prepare(&sql)?;
                let mut rows = stmt.query(NO_PARAMS)?;
                let mut res = Vec::new();
                while let Some(row) = rows.next()? {
                    let id: isize = row.get(0)?;
                    let id = usize::try_from(id).unwrap();
                    let node_id: Option<isize> = row.get(1)?;
                    let node_id = match node_id {
                        Some(v) => Some(usize::try_from(v).unwrap()),
                        None => None,
                    };
                    res.push(Interval {
                        id,
                        node_id,
                        begin: row.get(2)?,
                        end: row.get(3)?,
                        deleted: row.get(4)?,
                        closed: row.get(5)?,
                    })
                }
                SQLITEResult::Ok(res)
            }
        }
        impl Storage for Intervals {
            type Item = Interval;
            fn save(&self, interval: &Interval) -> DBResult<usize> {
                let node_id = interval.node_id.map(|r| isize::try_from(r).unwrap());
                if interval.id > 0 {
                    let id = isize::try_from(interval.id).unwrap();
                    self.con
                        .execute(
                            "update intervals
                set node_id = ?1,
                begin = ?2,
                end = ?3,
                deleted = ?4,
                closed = ?5
                where id = ?6",
                            &[
                                &node_id as &dyn::rusqlite::ToSql,
                                &interval.begin as &dyn::rusqlite::ToSql,
                                &interval.end as &dyn::rusqlite::ToSql,
                                &interval.deleted as &dyn::rusqlite::ToSql,
                                &interval.closed as &dyn::rusqlite::ToSql,
                                &id as &dyn::rusqlite::ToSql,
                            ],
                        )
                        .map_err(|s| DBError::wrap(Box::new(s)))?;
                    return Ok(interval.id);
                };
                self.con
                    .execute(
                        "insert into intervals (node_id, begin, end, deleted, closed) 
                values (?1, ?2, ?3, ?4, ?5)",
                        &[
                            &node_id as &dyn::rusqlite::ToSql,
                            &interval.begin as &dyn::rusqlite::ToSql,
                            &interval.end as &dyn::rusqlite::ToSql,
                            &interval.deleted as &dyn::rusqlite::ToSql,
                            &interval.closed as &dyn::rusqlite::ToSql,
                        ],
                    )
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(usize::try_from(self.con.last_insert_rowid()).unwrap())
            }
            fn all(&self) -> DBResult<Vec<Self::Item>> {
                let res = self
                    .select("select", "where deleted = 0")
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(res)
            }
            fn remove(&self, id: usize) -> DBResult<()> {
                let id = isize::try_from(id).unwrap();
                self.con
                    .execute(
                        "update intervals set deleted = 1 where id = ?1",
                        &[&id as &dyn::rusqlite::ToSql],
                    )
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(())
            }
            fn by_statement(&self, statement: Statement) -> DBResult<Vec<Self::Item>> {
                let res = self
                    .select(&statement.build_select(), &statement.build_where())
                    .map_err(|s| DBError::wrap(Box::new(s)))?;
                Ok(res)
            }
        }
        trait BuildSelect {
            fn build_select(&self) -> String;
        }
        impl BuildSelect for Statement {
            fn build_select(&self) -> String {
                if self.distinct {
                    return "select distinct".to_string();
                }
                "select".to_string()
            }
        }
        trait BuildWhere {
            fn build_where(&self) -> String;
        }
        impl BuildWhere for Statement {
            fn build_where(&self) -> String {
                let mut res = String::new();
                if self.filter.is_some() {
                    res += &{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["where "],
                            &match (&self.filter.as_ref().unwrap().build_where(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    };
                }
                if self.sorts.is_some() {
                    res += " order by ";
                    res += &self
                        .sorts
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|s| s.build_where())
                        .collect::<Vec<String>>()
                        .join(", ")
                }
                if self.limit.is_some() {
                    res += &{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[" limit "],
                            &match (&self.limit.unwrap(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    };
                }
                if self.offset.is_some() {
                    res += &{
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[" offset "],
                            &match (&self.offset.unwrap(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    };
                }
                res
            }
        }
        impl BuildWhere for SortItem {
            fn build_where(&self) -> String {
                {
                    let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                        &["", " "],
                        &match (&self.0, &self.1.build_where()) {
                            (arg0, arg1) => [
                                ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                            ],
                        },
                    ));
                    res
                }
            }
        }
        impl BuildWhere for SortDir {
            fn build_where(&self) -> String {
                match self {
                    SortDir::Ascend => "asc".to_string(),
                    SortDir::Descend => "desc".to_string(),
                }
            }
        }
        impl BuildWhere for FieldVal {
            fn build_where(&self) -> String {
                match self {
                    FieldVal::Usize(u) => u.to_string(),
                    FieldVal::DateTime(d) => {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["\"", "\""],
                            &match (&d.to_rfc3339(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    }
                    FieldVal::String(s) => {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["\"", "\""],
                            &match (&s.to_string(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    }
                    FieldVal::Bool(b) => (if *b { 1 } else { 0 }).to_string(),
                    FieldVal::Null => String::from("null"),
                }
            }
        }
        impl BuildWhere for CmpOp {
            fn build_where(&self) -> String {
                match self {
                    CmpOp::Eq(s, v) => {
                        let sign = if let FieldVal::Null = v { "is" } else { "=" };
                        {
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["", " ", " "],
                                &match (&s, &sign, &v.build_where()) {
                                    (arg0, arg1, arg2) => [
                                        ::core::fmt::ArgumentV1::new(
                                            arg0,
                                            ::core::fmt::Display::fmt,
                                        ),
                                        ::core::fmt::ArgumentV1::new(
                                            arg1,
                                            ::core::fmt::Display::fmt,
                                        ),
                                        ::core::fmt::ArgumentV1::new(
                                            arg2,
                                            ::core::fmt::Display::fmt,
                                        ),
                                    ],
                                },
                            ));
                            res
                        }
                    }
                    CmpOp::Ne(s, v) => {
                        let sign = if let FieldVal::Null = v {
                            "is not"
                        } else {
                            "<>"
                        };
                        {
                            let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                                &["", " ", " "],
                                &match (&s, &sign, &v.build_where()) {
                                    (arg0, arg1, arg2) => [
                                        ::core::fmt::ArgumentV1::new(
                                            arg0,
                                            ::core::fmt::Display::fmt,
                                        ),
                                        ::core::fmt::ArgumentV1::new(
                                            arg1,
                                            ::core::fmt::Display::fmt,
                                        ),
                                        ::core::fmt::ArgumentV1::new(
                                            arg2,
                                            ::core::fmt::Display::fmt,
                                        ),
                                    ],
                                },
                            ));
                            res
                        }
                    }
                    CmpOp::Gt(s, v) => {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["", " > "],
                            &match (&s, &v.build_where()) {
                                (arg0, arg1) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    }
                    CmpOp::Lt(s, v) => {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["", " < "],
                            &match (&s, &v.build_where()) {
                                (arg0, arg1) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    }
                }
            }
        }
        impl BuildWhere for Filter {
            fn build_where(&self) -> String {
                match self {
                    Filter::LogOp(lo) => lo.build_where(),
                    Filter::CmpOp(co) => co.build_where(),
                }
            }
        }
        impl BuildWhere for LogOp {
            fn build_where(&self) -> String {
                match self {
                    LogOp::Or(f1, f2) => {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["(", " or ", ")"],
                            &match (&f1.build_where(), &f2.build_where()) {
                                (arg0, arg1) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    }
                    LogOp::And(f1, f2) => {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["(", " and ", ")"],
                            &match (&f1.build_where(), &f2.build_where()) {
                                (arg0, arg1) => [
                                    ::core::fmt::ArgumentV1::new(arg0, ::core::fmt::Display::fmt),
                                    ::core::fmt::ArgumentV1::new(arg1, ::core::fmt::Display::fmt),
                                ],
                            },
                        ));
                        res
                    }
                    LogOp::Not(f) => {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &["(not ", ")"],
                            &match (&f.build_where(),) {
                                (arg0,) => [::core::fmt::ArgumentV1::new(
                                    arg0,
                                    ::core::fmt::Display::fmt,
                                )],
                            },
                        ));
                        res
                    }
                }
            }
        }
    }
}
mod style {
    use crossterm_style::{Color, ObjectStyle};
    use std::convert::TryInto;
    use termimad::*;
    pub struct TaskStyle {
        pub name: ObjectStyle,
        pub start_time: ObjectStyle,
        pub end_time: ObjectStyle,
        pub time_span: ObjectStyle,
    }
    impl Default for TaskStyle {
        fn default() -> Self {
            let name = ObjectStyle::default().fg(Color::Yellow);
            let start_time = ObjectStyle::default().fg(Color::Magenta);
            let end_time = ObjectStyle::default().fg(Color::Magenta);
            let time_span = ObjectStyle::default().fg(Color::Green);
            TaskStyle {
                name,
                start_time,
                end_time,
                time_span,
            }
        }
    }
    pub struct AppStyle {
        pub task: TaskStyle,
        pub error: ObjectStyle,
        pub cmd: ObjectStyle,
        pub report: MadSkin,
        pub screen_width: Option<usize>,
    }
    impl Default for AppStyle {
        fn default() -> Self {
            let cmd = ObjectStyle::default();
            let (width, _) = terminal_size();
            let area: Option<usize> = if width < 4 {
                Some(120)
            } else {
                Some(width.try_into().unwrap())
            };
            let mut report = MadSkin::default();
            report.paragraph.align = Alignment::Center;
            report.table.align = Alignment::Center;
            report.bold.set_fg(Color::Yellow);
            report.italic.object_style = ObjectStyle::default();
            report.italic.set_fg(Color::Magenta);
            report.inline_code.set_fgbg(Color::Reset, Color::Reset);
            AppStyle {
                task: TaskStyle::default(),
                error: ObjectStyle::default().fg(Color::Red),
                cmd,
                report,
                screen_width: area,
            }
        }
    }
}
use errors::*;
pub(crate) use format::*;
use history::DBWatcher;
pub use print::*;
use storage::sqlite::DB;
pub(crate) use style::*;
pub struct CrateInfo<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub authors: &'a str,
    pub description: &'a str,
}
pub struct AppContext<'a> {
    pub args: ArgMatches<'a>,
    pub conf: AppConfig,
    pub root: PathBuf,
    pub printer: Box<dyn Printer>,
    pub db: Box<dyn core::DBRoot>,
}
pub struct AppConfig {
    pub db_path: String,
    pub history_db_path: String,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            AppConfig {
                db_path: ref __self_0_0,
                history_db_path: ref __self_0_1,
            } => {
                let mut debug_trait_builder = f.debug_struct("AppConfig");
                let _ = debug_trait_builder.field("db_path", &&(*__self_0_0));
                let _ = debug_trait_builder.field("history_db_path", &&(*__self_0_1));
                debug_trait_builder.finish()
            }
        }
    }
}
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_AppConfig: () = {
    #[allow(unknown_lints)]
    #[allow(rust_2018_idioms)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for AppConfig {
        fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum __Field {
                __field0,
                __field1,
                __ignore,
            }
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "field identifier")
                }
                fn visit_u64<__E>(self, __value: u64) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::export::Ok(__Field::__field0),
                        1u64 => _serde::export::Ok(__Field::__field1),
                        _ => _serde::export::Err(_serde::de::Error::invalid_value(
                            _serde::de::Unexpected::Unsigned(__value),
                            &"field index 0 <= i < 2",
                        )),
                    }
                }
                fn visit_str<__E>(self, __value: &str) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "db_path" => _serde::export::Ok(__Field::__field0),
                        "history_db_path" => _serde::export::Ok(__Field::__field1),
                        _ => _serde::export::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::export::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"db_path" => _serde::export::Ok(__Field::__field0),
                        b"history_db_path" => _serde::export::Ok(__Field::__field1),
                        _ => _serde::export::Ok(__Field::__ignore),
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(__deserializer: __D) -> _serde::export::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                }
            }
            struct __Visitor<'de> {
                marker: _serde::export::PhantomData<AppConfig>,
                lifetime: _serde::export::PhantomData<&'de ()>,
            }
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = AppConfig;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::export::Formatter,
                ) -> _serde::export::fmt::Result {
                    _serde::export::Formatter::write_str(__formatter, "struct AppConfig")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::export::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 =
                        match match _serde::de::SeqAccess::next_element::<String>(&mut __seq) {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        } {
                            _serde::export::Some(__value) => __value,
                            _serde::export::None => {
                                return _serde::export::Err(_serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct AppConfig with 2 elements",
                                ));
                            }
                        };
                    let __field1 =
                        match match _serde::de::SeqAccess::next_element::<String>(&mut __seq) {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        } {
                            _serde::export::Some(__value) => __value,
                            _serde::export::None => {
                                return _serde::export::Err(_serde::de::Error::invalid_length(
                                    1usize,
                                    &"struct AppConfig with 2 elements",
                                ));
                            }
                        };
                    _serde::export::Ok(AppConfig {
                        db_path: __field0,
                        history_db_path: __field1,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::export::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::export::Option<String> = _serde::export::None;
                    let mut __field1: _serde::export::Option<String> = _serde::export::None;
                    while let _serde::export::Some(__key) =
                        match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        }
                    {
                        match __key {
                            __Field::__field0 => {
                                if _serde::export::Option::is_some(&__field0) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "db_path",
                                        ),
                                    );
                                }
                                __field0 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<String>(&mut __map) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field1 => {
                                if _serde::export::Option::is_some(&__field1) {
                                    return _serde::export::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "history_db_path",
                                        ),
                                    );
                                }
                                __field1 = _serde::export::Some(
                                    match _serde::de::MapAccess::next_value::<String>(&mut __map) {
                                        _serde::export::Ok(__val) => __val,
                                        _serde::export::Err(__err) => {
                                            return _serde::export::Err(__err);
                                        }
                                    },
                                );
                            }
                            _ => {
                                let _ = match _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)
                                {
                                    _serde::export::Ok(__val) => __val,
                                    _serde::export::Err(__err) => {
                                        return _serde::export::Err(__err);
                                    }
                                };
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::export::Some(__field0) => __field0,
                        _serde::export::None => match _serde::private::de::missing_field("db_path")
                        {
                            _serde::export::Ok(__val) => __val,
                            _serde::export::Err(__err) => {
                                return _serde::export::Err(__err);
                            }
                        },
                    };
                    let __field1 = match __field1 {
                        _serde::export::Some(__field1) => __field1,
                        _serde::export::None => {
                            match _serde::private::de::missing_field("history_db_path") {
                                _serde::export::Ok(__val) => __val,
                                _serde::export::Err(__err) => {
                                    return _serde::export::Err(__err);
                                }
                            }
                        }
                    };
                    _serde::export::Ok(AppConfig {
                        db_path: __field0,
                        history_db_path: __field1,
                    })
                }
            }
            const FIELDS: &'static [&'static str] = &["db_path", "history_db_path"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "AppConfig",
                FIELDS,
                __Visitor {
                    marker: _serde::export::PhantomData::<AppConfig>,
                    lifetime: _serde::export::PhantomData,
                },
            )
        }
    }
};
impl Default for AppConfig {
    fn default() -> Self {
        let db_path = String::from("yatt.db");
        let history_db_path = String::from("yatt_history.db");
        AppConfig {
            db_path,
            history_db_path,
        }
    }
}
fn parse_config(base_path: &PathBuf) -> CliResult<AppConfig> {
    let mut s = Config::new();
    let path = base_path.join("config");
    if s.merge(File::with_name(path.to_str().unwrap())).is_err() {
        return Ok(AppConfig::default());
    }
    match s.try_into() {
        Ok(res) => Ok(res),
        Err(e) => Err(CliError::Config { source: e }),
    }
}
fn make_args<'a>(info: &CrateInfo<'a>) -> ArgMatches<'a> {
    let app = App::new(info.name)
        .version(info.version)
        .author(info.authors)
        .about(info.description)
        .setting(AppSettings::ArgRequiredElseHelp);
    commands::register(app).get_matches()
}
fn app_dir(name: &str) -> CliResult<PathBuf> {
    if let Some(p) = dirs::config_dir() {
        return Ok(p.join(name));
    }
    Err(CliError::AppDir {
        message: "Unable to resolve os config directory path".to_string(),
    })
}
pub fn run(info: CrateInfo) -> CliResult<()> {
    let base_path = app_dir(info.name)?;
    if !base_path.exists() {
        if let Err(e) = fs::create_dir_all(&base_path) {
            return Err(CliError::Io { source: e });
        }
    } else if !base_path.is_dir() {
        return Err(CliError::AppDir {
            message: {
                let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                    &["", " is not a directory"],
                    &match (&base_path.to_str().unwrap_or(""),) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                ));
                res
            },
        });
    }
    let mut skin = MadSkin::default();
    skin.set_headers_fg(rgb(255, 187, 0));
    skin.bold.set_fg(Yellow);
    skin.italic.set_fgbg(Magenta, rgb(30, 30, 40));
    skin.bullet = StyledChar::from_fg_char(Yellow, '⟡');
    skin.quote_mark.set_fg(Yellow);
    let mut conf = parse_config(&base_path)?;
    #[cfg(debug_assertions)]
    debug_config(&mut conf);
    let mut db: Box<dyn core::DBRoot> = Box::new({
        match DB::new(base_path.join(&conf.db_path)) {
            Ok(db) => db,
            Err(e) => return Err(CliError::DB { source: e }),
        }
    });
    let history_db_path = base_path.join(&conf.history_db_path);
    if history_db_path.exists() {
        let hs = Rc::new({
            match history_storage::sqlite::DB::new(history_db_path) {
                Ok(db) => db,
                Err(e) => return Err(CliError::DB { source: e }),
            }
        });
        db = Box::new(DBWatcher::new(db, hs));
    }
    let app = AppContext {
        args: make_args(&info),
        conf,
        root: base_path,
        printer: Box::new(TermPrinter::default()),
        db,
    };
    let res = commands::exec(&app);
    if res.is_err() {
        print_error(res.as_ref().unwrap_err(), app.printer);
    }
    res
}
fn print_error(e: &CliError, p: Box<dyn Printer>) {
    if let CliError::Task { source } = e {
        match source {
            TaskError::Cmd { message } => p.error(message),
            TaskError::CmdTaskInterval {
                message,
                interval,
                task,
            } => p.interval_error(
                &IntervalData {
                    interval,
                    task,
                    title: IntervalData::default_title(),
                },
                message,
            ),
        }
        return;
    }
    p.error(&e.to_string());
}
fn debug_config(conf: &mut AppConfig) {
    conf.db_path = "yatt_debug.db".to_string();
}
