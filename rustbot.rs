extern mod std(vers = "0.5");

use std::*;
use getopts::*;

use io::println;
use io::*;

use ip = net::ip;
use socket = net::tcp;

use task;
use uv::iotask;
use uv::iotask::iotask;

use core::str;
use core::str::*;

use core::run;
use core::run::*;

use core::vec;
use core::vec::*;

use irc;
use irc::*;

use conf;
use conf::*;

fn usage(binary: &str) {
    io::println(fmt!("Usage: %s [options]\n", binary) +
            "
Options:

    -h --help           Show this helpful screen
");
}

fn handle(irc: @Irc, m: &IrcMsg) {
    match m.code {
        // Hook channel join here. Made sense at the time?
        ~"004" => {
            join(irc, "#madeoftree");
        }
        ~"JOIN" => {
            privmsg(irc, m.param, "yoyo the mighty rustbot has arrived!");
        }
        _ => (),
    }
}

fn nextep(m: &CmdMsg) -> ~str {
    let { status, out, err } = program_output("nextep", [~"--short", copy m.arg]);
    if (status == 0) {
        return move out;
    }
    else {
        return move err;
    }
}

fn register_callbacks(irc: &Irc) {
    // TODO suppress unused parameter error?
    register_bare_cmd(irc, ~"help", || ~"Prefix commands with a '.' and try '.cmds'");
    register_bare_cmd(irc, ~"about", || ~"I'm written in rust as a learning experience, " +
        "try http://www.rust-lang.org!");
    register_bare_cmd(irc, ~"botsnack", || ~":)");
    register_bare_cmd(irc, ~"status", || ~"Status: 418 I'm a teapot");
    register_bare_cmd(irc, ~"src", || ~"http://github.com/treeman/rustbot");

    register_cmd(irc, ~"insult", |m| fmt!("%s thinks rust is iron oxide.", m.arg));
    register_cmd(irc, ~"compliment", |m| fmt!("%s is best friends with rust.", m.arg));
    register_cmd(irc, ~"nextep", nextep);
}

fn main() {
    let mut args = os::args();
    let binary = args.shift();

    let opts = ~[
        getopts::optflag("h"),
        getopts::optflag("help"),
    ];

    let matches = match getopts(args, opts) {
        Ok(m) => copy m,
        Err(f) => {
            io::println(fmt!("Error: %s\n", getopts::fail_str(copy f)));
            usage(binary);
            return;
        }
    };

    if opt_present(copy matches, "h") || opt_present(copy matches, "help") {
        usage(binary);
        return;
    }

    let conf = load(~"rustbot.conf");
    let mut irc = connect(conf);

    identify(irc);
    register_callbacks(irc);

    // TODO remove handle from here
    run(irc, handle);
}

