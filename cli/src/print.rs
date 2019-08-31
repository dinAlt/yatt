use core::*;

use crossterm_style::ObjectStyle;

use crate::*;

pub(crate) fn print_cmd(cmd: &str, s: &ObjectStyle) {
    println!("{}\n", s.apply_to(cmd));
}

pub(crate) fn print_interval_info(task: &[Node], i: &Interval, s: &TaskStyle) {
    println!("Interval info:");
    print!("  Task: ");
    for (i, t) in task.iter().enumerate() {
       print!("{}", s.name.apply_to(&t.label));
       if i < task.len() - 1 {
           print!(" > ");
       }
    }
    println!();
    print!(
        "  Started: {}",
        s.start_time.apply_to(format_datetime(&i.begin))
    );

    let dur = Utc::now() - i.begin;

    if dur.num_seconds() > 2 {
        print!(" ({} ago)", s.time_span.apply_to(format_duration(&dur)));
    }

    if i.end.is_some() {
        let e = i.end.unwrap();
        print!("\n  Stopped: {}", s.end_time.apply_to(format_datetime(&e)));
        let dur = Utc::now() - e;
        if dur.num_seconds() > 2 {
            print!(" ({} ago)", s.time_span.apply_to(format_duration(&dur)));
        }
    }

    println!(".");
}

pub(crate) fn print_error(message: &str, s: &ObjectStyle) {
    println!("Error: {}\n", s.apply_to(message));
}
