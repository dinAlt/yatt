use super::*;
use std::cell::RefCell;
use std::rc::Rc;
type ExecFn = fn(cmd: &mut Command, args: &ArgMatches) -> CliResult<()>;

pub(crate) struct Root<'a> {
    app: App<'a, 'a>,
}
pub(crate) struct Command<'a> {
    name: String,
    app: App<'a, 'a>,
    root: RefCell<Root<'a>>,
    run: ExecFn,
    subcommans: Vec<Command<'a>>,
    is_root: bool,
}

impl<'a> Command<'a> {
    pub fn root(n: impl Into<String>) -> Self {
        let n = n.into();
        // Command {
        //     app: &mut App::new(&n),
        //     name: n,
        //     run: Self::dummy,
        //     subcommans: Vec::new(),
        //     is_root: true,
        // }
        unimplemented!()
    }
    pub fn arg(mut self, a: Arg<'a, 'a>) -> Self {
        let app = self.root.into_inner();
        self.root = RefCell::new(app);
        self
    }
    // pub fn alias(mut self, name: impl Into<&'a str>) -> Self {
    //     self.app = &mut self.app.alias(name);
    //     self
    // }
    // pub fn run_fn(mut self, f: ExecFn) -> Self {
    //     self.run = f;
    //     self
    // }

    fn dummy(cmd: &mut Command, _args: &ArgMatches) -> CliResult<()> {
        if cmd.is_root {
            cmd.app
                .print_help()
                .map_err(|s| CliError::wrap(Box::new(s)))?;
        };

        Ok(())
    }
}

fn construct() {
    // let mut cmd = Command::root("vasa").arg(Arg::with_name("tent"));
}

