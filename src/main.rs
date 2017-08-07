extern crate rlua;
#[macro_use]
extern crate hlist_macro;
extern crate uuid;
extern crate clap;

mod config;
mod reader;
mod node;
mod tree;

use std::io::prelude::*;
use std::fs::File;
use clap::{Arg, App, SubCommand};
use uuid::Uuid;
use tree::Tree;
use node::Node;

fn main() {
    let matches = App::new("sofer")
        .version("0.0.0")
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .takes_value(true)
            .value_name("FILE")
            .required(false)
            .help("File to read from. If it isn't provided, sofer will read from stdio.")
        )
        .arg(Arg::with_name("from")
            .long("from")
            .takes_value(true)
            .help("Set importing format")
        )
        .arg(Arg::with_name("to")
            .long("to")
            .takes_value(true)
            .help("Set exporting format")
        )
        .subcommand(SubCommand::with_name("tree-node")
            .subcommand(SubCommand::with_name("eval")
                .arg(Arg::with_name("UUID").required(true))
            )
            .subcommand(SubCommand::with_name("eval-all"))
        )
        .subcommand(SubCommand::with_name("tree")
            .subcommand(SubCommand::with_name("insert")
                .arg(Arg::with_name("UUID").required(true))
                .arg(Arg::with_name("CONTENT").required(true))
            )
            .subcommand(SubCommand::with_name("insert-to-sibling"))
        )
        .subcommand(SubCommand::with_name("reader")
            .subcommand(SubCommand::with_name("read"))
        )
        .subcommand(SubCommand::with_name("uuid")
            .subcommand(SubCommand::with_name("new"))
        )
        .get_matches();

    let str = match matches.value_of("file") {
        Some(file_name) => {
            let mut f = match File::open(file_name) {
                Ok(f) => f,
                Err(err) => panic!("{}", err),
            };
            let mut buffer = Vec::new();
            let _ = f.read_to_end(&mut buffer);
            match String::from_utf8(Vec::from(buffer)) {
                Ok(string) => string,
                Err(err) => panic!("{}", err),
            }
        }
        None => {
            let mut stdio = std::io::stdin();
            let mut str = String::new();
            let _ = stdio.read_to_string(&mut str);
            str
        }
    };

    let mut treenode = match matches.value_of("from") {
        Some("lua") =>
            node::TreeNode::import_from_lua(&str),
        Some(x) =>
            panic!("Format \"{}\" not supported.", x),
        None =>
            node::TreeNode::import_from_sofer(&str),
    };

    let mut export = false;

    match matches.subcommand() {
        ("tree-node", Some(sub)) => {
            match sub.subcommand() {
                ("eval", Some(subsub)) => {
                    println!(
                        "{}",
                        treenode
                            .find(Uuid::parse_str(subsub.value_of("UUID").unwrap()).expect("Couldn't read UUID"))
                            .expect(&format!("Couldn't find node with UUID \"{}\"", subsub.value_of("UUID").unwrap()))
                            .eval()
                        );
                }
                ("eval-all", Some(_)) => {
                    treenode.eval_all();

                    export = true;
                }
                _ => (),
            }
        }
        ("tree", Some(sub)) => {
            match sub.subcommand() {
                ("insert", Some(subsub)) => {
                    let uuid = Uuid::parse_str(subsub.value_of("UUID").unwrap()).expect("Couldn't read UUID");
                    let content = subsub.value_of("CONTENT").unwrap();
                    treenode.insert(uuid, Tree::new_child(Node::new(content.into(), Vec::new())));

                    export = true;
                }
                _ => (),
            }
        }
        ("reader", Some(sub)) => {
            match sub.subcommand() {
                ("read", Some(_)) => {
                    export = true;
                }
                _ => (),
            }
        }
        ("uuid", Some(sub)) => {
            match sub.subcommand() {
                ("new", Some(_)) => {
                    println!("{}", Uuid::new_v4());
                }
                _ => (),
            }
        }
        (command, _) => println!("Command \"{}\" not recognized.", command),
    }

    if export {
        match matches.value_of("to") {
            Some("lua") =>
                println!("{}", treenode.export_to_lua()),
            Some(x) =>
                println!("Format \"{}\" not supported.", x),
            None =>
                println!("{}", treenode.export_to_sofer()),
        }
    }
}
