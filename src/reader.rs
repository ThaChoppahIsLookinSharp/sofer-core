use std::fmt;
use uuid::Uuid;

use node;
use node::*;

#[derive(Debug, Clone)]
pub struct Node {
    content: String,
    attributes: Vec<Attribute>,
    uuid: Uuid,
    parent_uuid: Uuid,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.uuid, self.parent_uuid, self.content)
    }
}

pub fn read_nodes(str: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut chars = str.chars();

    let mut uuid_string = String::new();
    let mut parent_uuid_string = String::new();
    let mut attributes_string = String::new();
    let mut content = String::new();

    let mut reading = 0;
    /* 0 = uuid
     * 1 = parent_uuid
     * 2 = attributes
     * 3 = content
     */

    loop {
        match chars.next() {
            Some(' ') => {
                if reading < 3 {
                    reading += 1;
                    continue;
                } else {
                    content.push(' ');
                }
            }
            Some('\n') => {
                let uuid = match Uuid::parse_str(&uuid_string) {
                    Ok(uuid) => uuid,
                    Err(_) => panic!("Wrong UUID: {}", uuid_string),
                };

                let parent_uuid = match Uuid::parse_str(&parent_uuid_string) {
                    Ok(uuid) => uuid,
                    Err(_) => panic!("Wrong UUID: {}", parent_uuid_string),
                };

                let attributes = read_attributes(&attributes_string);

                nodes.push(Node {
                    content: content,
                    attributes,
                    uuid,
                    parent_uuid,
                });

                uuid_string.clear();
                parent_uuid_string.clear();
                attributes_string.clear();
                content = String::new();
                reading = 0;
            }
            Some(c) => {
                match reading {
                    0 => uuid_string.push(c),
                    1 => parent_uuid_string.push(c),
                    2 => attributes_string.push(c),
                    3 => content.push(c),
                    _ => panic!("this should not have happened"),
                }
            }
            None => {
                sort_nodes(&mut nodes);
                return nodes;
            }
        }
    }
}

fn read_attributes(attributes_string: &str) -> Vec<Attribute> {
    let mut attributes = Vec::new();
    let mut iter = attributes_string.chars().peekable();
    let mut reading = 0;
    /* 0 = field
     * 1 = value
     */
    let mut reading_string = false;
    let mut field = String::new();
    let mut value = String::new();
    loop {
        if !reading_string {
            match iter.next() {
                Some('=') => {
                    reading = 1;
                    if iter.peek() == Some(&'"') {
                        iter.next();
                        reading_string = true;
                        value.push('"');
                    }
                }
                Some(';') => {
                    {
                        let mut chars = value.chars();
                        match chars.nth(0) {
                            Some('"') => {
                                attributes.push(
                                    Attribute::String(
                                        field,
                                        chars.filter(|&c| c != '"').collect::<String>()
                                    )
                                );
                            }
                            Some('T') => {
                                if chars.nth(1) == None {
                                    attributes.push(
                                        Attribute::Boolean(field, true)
                                    )
                                } else { panic!(); }
                            }
                            Some('F') => {
                                if chars.nth(1) == None {
                                    attributes.push(
                                        Attribute::Boolean(field, false)
                                    )
                                } else { panic!(); }
                            }
                            Some(_) => {
                                match value.parse() {
                                    Ok(num) =>
                                        attributes.push(
                                            Attribute::Number(field, num)
                                        ),
                                    Err(_) => panic!(),
                                }
                            }
                            None => panic!(),
                        }
                    }
                    field = String::new();
                    value = String::new();
                    reading = 0;
                }
                Some(c) => {
                    match reading {
                        0 => field.push(c),
                        1 => value.push(c),
                        _ => panic!(),
                    }
                }
                None => break attributes,
            }
        } else {
            match iter.next() {
                Some('"') => {
                    reading_string = false;
                    value.push('"');
                }
                Some(c) => {
                    value.push(c);
                },
                None => panic!(),
            }
        }
    }
}

pub fn sort_nodes(nodes: &mut Vec<Node>) {
    nodes.sort_by(|n1, n2| n1.parent_uuid.cmp(&n2.parent_uuid))
}

pub fn nodes_to_tree_node(nodes: Vec<Node>) -> TreeNode {
    let mut tree_nodes: Vec<TreeNode> = Vec::new();
    nodes
        .iter()
        .inspect(|n| {
            match nodes.iter().position(|n_| n_.uuid == n.parent_uuid) {
                Some(idx) => {
                    match tree_nodes.get_mut(idx) {
                        Some(parent) => {
                            parent.insert(
                                n.parent_uuid,
                                nodes_to_one_tree_node(
                                    &nodes,
                                    TreeNode {
                                        value: node::Node::new(n.content.clone(), n.attributes.clone()),
                                        uuid: n.uuid,
                                        first_child: None,
                                        next_sibling: None,
                                    }
                                )
                            );
                        }
                        None => (),
                    }
                }

                None => {
                    tree_nodes.push(TreeNode {
                        value: node::Node::new(n.content.clone(), n.attributes.clone()),
                        uuid: n.uuid,
                        first_child: None,
                        next_sibling: None,
                    });
                }
            }
        })
        .collect::<Vec<&Node>>();

    let mut treenode = TreeNode::new_tree(node::Node::new("".into(), Vec::new()));

    for tn in tree_nodes {
        treenode.insert(Uuid::nil(), tn);
    }

    treenode
}

fn nodes_to_one_tree_node(nodes: &Vec<Node>, tree_node: TreeNode) -> TreeNode {
    let mut tree_node = tree_node;

    nodes
        .iter()
        .inspect(|n| {
            tree_node.insert(
                n.parent_uuid,
                TreeNode {
                    value: node::Node::new(n.content.clone(), n.attributes.clone()),
                    uuid: n.uuid,
                    first_child: None,
                    next_sibling: None,
                });
        })
        .collect::<Vec<&Node>>();

    tree_node
}
