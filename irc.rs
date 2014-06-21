use std::io::*;
use connection::*;
use writer::*;
use regex::*;
use core::fmt::{Show, Formatter, Result};
use std::collections::hashmap::HashSet;

// Single server for now.
pub struct IrcConfig<'a> {
    pub host: &'a str,
    pub port: u16,
    pub channels: Vec<&'a str>,
    pub nick: &'a str,
    pub descr: &'a str,
    pub blacklist: Vec<&'a str>,
}

// A regular irc message sent from the server.
struct IrcMsg {
    orig: String,
    prefix: String,
    code: String,
    param: String,
}

impl IrcMsg {
    fn new(s: &str) -> Option<IrcMsg> {
        let re = regex!(r"^(:\S+)?\s*(\S+)\s+(.*)\r?$");
        let caps = re.captures(s);
        match caps {
            Some(x) => {
                Some(IrcMsg {
                    orig: s.to_string(),
                    prefix: x.at(1).to_string(),
                    code: x.at(2).to_string(),
                    param: x.at(3).to_string(),
                })
            },
            None => None
        }
    }
}

impl Show for IrcMsg {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "prefix: {} code: {} param: {}",
               self.prefix, self.code, self.param)
    }
}

// command callbacks?
// + register functionsto send to irc.
pub struct Irc<'a> {
    // Connections to irc server and over internal channel.
    conn: ServerConnection,
    // Cannot make this work.
    //tx: Sender<ConnectionEvent>,
    //rx: Receiver<ConnectionEvent>,

    // Bot info.
    nick: &'a str,
    descr: &'a str,
    channels: Vec<&'a str>,

    // General config
    //blacklist: Vec<&'a str>,
    blacklist: HashSet<String>, // String to avoid lifetime issues :)

    // Callbacks at received events
    raw_cb: Vec<|s: &str|:'a -> Option<String>>,
}

// Simple wrapper on top of regex replace matching.
// Used for the option type.
fn raw_replace(s: &str, re: Regex, res: &str) -> Option<String> {
    if re.is_match(s) {
        //println!("Matched {} with {}", s, re);
        Some(re.replace(s, res))
    }
    else {
        None
    }
}

// FIXME move?
fn ping(s: &str) -> Option<String> {
    raw_replace(s, regex!(r"^PING\s(.+)$"), "PONG $1")
}

impl<'a> Irc<'a> {
    // Create a new irc instance and connect to the server, but don't act on it.
    pub fn connect<'b>(conf: IrcConfig<'b>) -> Irc<'b> {

        let mut blacklist = HashSet::new();
        for x in conf.blacklist.iter() {
            blacklist.insert(x.to_string());
        }
        let mut irc = Irc {
            conn: ServerConnection::new(conf.host, conf.port),
            //tx: tx,
            //rx: rx,
            nick: conf.nick,
            descr: conf.descr,
            channels: conf.channels,

            blacklist: blacklist,

            raw_cb: Vec::new(),
        };

        irc.raw_cb.push(ping);

        irc
    }

    // Construct a writer we can use to send things to irc.
    // Uses a channel transmitter with a process in the backround.
    //pub fn writer(&self) -> IrcWriter {
        //IrcWriter::new(self.tx.clone())
    //}

    // Called when we have a properly formatted irc message.
    fn handle_msg(&mut self, msg: &IrcMsg, writer: &IrcWriter) {
        // Print received message if it's not blacklisted.
        let code = msg.code.clone();
        if !self.blacklist.contains(&code) {
            println!("< {}", msg.orig);
        }

        // Join when we receive 004 because it's often there (or something)
        if code.as_slice() == "004" {
            for chan in self.channels.iter() {
                writer.join(*chan);
            }
        }
    }

    // Called when we receive a response from the server.
    fn handle_received(&mut self, line: &String, writer: &IrcWriter) {
        // Trim away newlines and unneeded spaces.
        let s = line.as_slice().trim();

        for cb in self.raw_cb.mut_iter() {
            // FIXME pass in writer to callback instead?
            match (*cb)(s) {
                Some(x) => writer.write_line(x),
                _ => (),
            }
        }

        match IrcMsg::new(s) {
            Some(msg) => {
                // Print inside here so we can skip certain codes.
                self.handle_msg(&msg, writer);
            },
            _ => {
                // Couldn't capture message, print it here.
                println!("<! {}", s);
            },
        }
    }

    // Run irc client and block until done.
    pub fn run(&mut self) {
        let (tx, rx) = channel();
        self.spawn_reader(tx.clone());
        self.run_handler(tx.clone(), rx);
    }

    // Spawn a proc reader which listens to incoming messages from irc.
    fn spawn_reader(&self, tx: Sender<ConnectionEvent>) {
        println!("Spawning irc reader");
        let tcp = self.conn.tcp.clone(); // Workaround to avoid irc capture
        spawn(proc() {
            let mut reader = BufferedReader::new(tcp);
            loop {
                match read_line(&mut reader) {
                    Some(x) => tx.send(Received(x)),
                    None => break,
                }
            }
            println!("Quitting irc reader");
        });
    }

    // FIXME spawn a thread instead?
    fn run_handler(&mut self, tx: Sender<ConnectionEvent>, rx: Receiver<ConnectionEvent>) {
        println!("Spawning event handler");
        let tcp = self.conn.tcp.clone();
        let mut stream = LineBufferedWriter::new(tcp.clone());
        let writer = IrcWriter::new(tx);

        // Start with identifying
        writer.identify(self.nick, self.descr);

        // Loop and handle in and output events.
        // Quit is a special case to allow us to close the program.
        for x in rx.iter() {
            match x {
                Output(ref s) => {
                    // FIXME method for this?
                    println!("> {}", s);
                    write_line(&mut stream, s.as_slice());
                },
                Received(ref s) => {
                    self.handle_received(s, &writer);
                },
                Quit => {
                    // FIXME close all things.
                    //self.conn.close();
                    break;
                },
            }
        }
        println!("Exiting irc writer");
    }
}

//struct IrcPrivMsg {
    //orig: String
    //prefix: String,
    //channel: String,
    //msg: String,
//}

// Commands to the bot
//struct IrcCmdMsg {
    //orig: String,
    //prefix: String,
    //channel: String,
    //cmd: String,
    //args: String,
//}

mod tests {
    // Test irc message matching
    #[test]
    fn msg() {
        some_msg(":pref 020 rustbot lblblb", ":pref", "020", "rustbot lblblb");
        some_msg("020 rustbot lblblb", "", "020", "rustbot lblblb");
        some_msg(":dreamhack.se.quakenet.org 376 rustbot :End of /MOTD command",
                 ":dreamhack.se.quakenet.org", "376", "rustbot :End of /MOTD command");
        none_msg("a");
    }

    // FIXME correct tests
    // Test callbacks
    //#[test]
    //fn ping() {
        //test_cb_match(super::ping, "PING :423131321", "PONG :423131321");
        //test_cb_none(super::ping, "JOIN :asdf");
    //}

    // IRC message parsing test functions
    #[cfg(test)]
    fn some_msg(s: &str, prefix: &str, code: &str, param: &str) {
        match super::IrcMsg::new(s) {
            Some(x) => {
                assert_eq!(x.prefix, prefix.to_string());
                assert_eq!(x.code, code.to_string());
                assert_eq!(x.param, param.to_string());
            },
            None => fail!("Did not match {}", s),
        }
    }

    #[cfg(test)]
    fn none_msg(s: &str) {
        match super::IrcMsg::new(s) {
            Some(_) => fail!("Matched {}, s"),
            None => (),
        }
    }

    // Raw callback test functions
    #[cfg(test)]
    fn test_cb_match(f: |String| -> Option<String>, s: &str, expected: &str) {
        match f(s.to_string()) {
            Some(got) => assert_eq!(got, expected.to_string()),
            None => fail!("None"),
        }
    }

    #[cfg(test)]
    fn test_cb_none(f: |String| -> Option<String>, s: &str) {
        match f(s.to_string()) {
            Some(_) => fail!("Some"),
            None => (),
        }
    }
}

