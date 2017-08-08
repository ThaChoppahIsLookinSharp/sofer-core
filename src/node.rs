use rlua;
use rlua::Lua;
use uuid::Uuid;
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;

use reader;
use tree;

#[derive(Clone, Debug)]
pub enum Attribute {
    String(String, String),
    Number(String, f32),
    Boolean(String, bool),
}

fn attributes_from_lua<'lua>(lua_value: rlua::LuaValue<'lua>) -> rlua::LuaResult<Vec<Attribute>> {
    let mut attrs = Vec::new();
    match lua_value {
        rlua::LuaValue::Table(table) => {
            let mut pairs = table.pairs();
            loop {
                match pairs.next() {
                    Some(Ok((attr_name, rlua::LuaValue::String(str)))) =>
                        attrs.push(Attribute::String(attr_name, String::from(str.to_str()?))),

                    Some(Ok((attr_name, rlua::LuaValue::Number(x)))) =>
                        attrs.push(Attribute::Number(attr_name, x as f32)),

                    Some(Ok((attr_name, rlua::LuaValue::Boolean(b)))) =>
                        attrs.push(Attribute::Boolean(attr_name, b)),

                    None => break Ok(attrs),

                    _ => (),
                }
            }
        }
        x => Err(rlua::LuaError::FromLuaConversionError(format!("Can't convert {:?} to a list of attributes", x))),
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub raw: String,
    pub evaled: Option<String>,
    pub attributes: Vec<Attribute>,
}

impl Node {
    pub fn new(raw: String, attributes: Vec<Attribute>) -> Node {
        Node {
            raw,
            evaled: None,
            attributes,
        }
    }
}

impl<'lua> rlua::ToLua<'lua> for Node {
    fn to_lua(self, lua: &'lua rlua::Lua) -> rlua::LuaResult<rlua::LuaValue> {
        let table = lua.create_table();
        table.set("raw", self.raw)?;
        Ok(rlua::LuaValue::Table(table))
    }
}

impl<'lua> rlua::FromLua<'lua> for Node {
    fn from_lua(lua_value: rlua::LuaValue<'lua>, _: &'lua rlua::Lua) -> rlua::LuaResult<Node> {
        match lua_value {
            rlua::LuaValue::Table(table) => {
                let raw: String = table.get("raw")?;

                let evaled: Option<String> = match table.get("evaled")? {
                    rlua::LuaValue::String(str) => Some(str.to_str()?.into()),
                    rlua::LuaValue::Nil => None,
                    x => return Err(rlua::LuaError::FromLuaConversionError(format!("Can't convert {:?} to String", x))),
                };

                let attributes = attributes_from_lua(table.get("attributes")?)?;

                Ok(Node {
                    raw,
                    evaled,
                    attributes
                })
            }
            x => Err(rlua::LuaError::FromLuaConversionError(format!("Can't convert {:?} to Tree", x))),
        }
    }
}

pub type TreeNode = tree::Tree<Node>;

impl TreeNode {
    pub fn eval(&self) -> String {
        let lua = Lua::new();

        let mut text = self.value.raw.chars().take_while(|&c| c != '@').collect::<String>();
        let lua_code = self.value.raw.chars().skip_while(|&c| c != '@').skip(1).collect::<String>();

        let result = if !lua_code.is_empty() {
            match lua.eval(&lua_code) {
                Ok(rlua::LuaValue::Function(f)) =>
                    f.call::<TreeNode, String>(self.clone()).unwrap_or(String::from("error function")),
                Ok(x) => format!("{:?}", x),
                Err(err) => format!("{:?}", err),
            }
        } else {
            String::from("")
        };

        text.push_str(&result);
        text
    }

    pub fn eval_all(&mut self) {
        self.value.evaled = Some(self.eval());

        match self.first_child {
            Some(ref mut first_child) => first_child.eval_all(),
            None => (),
        }

        match self.next_sibling {
            Some(ref mut next_sibling) => next_sibling.eval_all(),
            None => (),
        }
    }

    pub fn import_from_sofer(str: &str) -> TreeNode {
        reader::nodes_to_tree_node(reader::read_nodes(str))
    }

    pub fn import_from_lua(lua_code: &str) -> TreeNode {
        let lua = rlua::Lua::new();
        let treenode: TreeNode = lua.eval(lua_code).unwrap();
        treenode
    }

    pub fn import_from_opml(str: &str) -> TreeNode {
        let parser = EventReader::from_str(str);
        let mut reading = false;
        let mut ids = vec![];
        let mut tree = tree::Tree::new_tree(Node::new("".into(), vec![]));
        ids.push(tree.uuid);
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    if reading {
                        let new_uuid = {
                            let parent_uuid = ids.last().unwrap();
                            let text = {
                                let mut text = String::from("");
                                for OwnedAttribute { name, value } in attributes {
                                    if name.local_name == "text" {
                                        text = value;
                                        break;
                                    }
                                }
                                text
                            };
                            let new = tree::Tree::new_child(Node::new(text, vec![]));
                            let new_uuid = new.uuid;
                            tree.insert(parent_uuid.clone(), new);
                            new_uuid
                        };
                        ids.push(new_uuid);
                    }
                    if name.local_name == "body" {
                        reading = true;
                    }
                }
                Ok(XmlEvent::EndElement { name }) => {
                    if name.local_name == "body" {
                        reading = false;
                    }
                    if reading {
                        ids.pop();
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        tree
    }

    pub fn export_to_sofer(&self, evaled: bool) -> String {
        fn to_vec(n: &TreeNode, evaled: bool) -> Vec<(Uuid, Uuid, String, String)> {
            let mut treenodes = Vec::new();

            let text = if evaled {
                n.value.evaled.clone().unwrap_or(n.value.raw.clone())
            } else {
                n.value.raw.clone()
            };

            treenodes.push((n.uuid, Uuid::nil(), n.export_attributes(), text));
            treenodes.append(&mut to_vec_children(&n, evaled));

            for sibling in n.get_siblings() {
                let text = if evaled {
                    sibling.value.evaled.clone().unwrap_or(sibling.value.raw.clone())
                } else {
                    sibling.value.raw.clone()
                };

                treenodes.push((sibling.uuid, Uuid::nil(), sibling.export_attributes(), text));
                treenodes.append(&mut to_vec_children(&sibling, evaled));
            }

            treenodes
        }

        fn to_vec_children(n: &TreeNode, evaled: bool) -> Vec<(Uuid, Uuid, String, String)> {
            let mut treenodes = Vec::new();
            for child in n.get_children() {
                let text = if evaled {
                    child.value.evaled.clone().unwrap_or(child.value.raw.clone())
                } else {
                    child.value.raw.clone()
                };

                treenodes.push((child.uuid, n.uuid, child.export_attributes(), text));
                treenodes.append(&mut to_vec_children(&child, evaled));
            }
            treenodes
        }

        let mut str = String::new();
        let mut vec = to_vec(self, evaled);
        vec.sort_by_key(|k| k.0);
        vec
            .iter()
            .skip(1)
            .inspect(|x| str.push_str(&format!("{} {} {} {}\n", x.0, x.1, x.2, x.3)))
            .collect::<Vec<_>>();
        str
    }

    pub fn export_to_lua(&self) -> String {
        let mut str = String::new();
        str.push('{');

        str.push_str("value={");

        str.push_str("raw=");
        str.push_str(&format!("{:?}", self.value.raw));
        str.push(',');

        str.push_str("evaled=");
        match self.value.evaled {
            Some(ref evaled) => str.push_str(&format!("{:?}", evaled)),
            None => str.push_str("nil"),
        }
        str.push(',');

        str.push_str("attributes={");
        for attr in &self.value.attributes {
            use node::Attribute::*;
            match attr {
                &String(ref k, ref v) => str.push_str(&format!("[\"{}\"]={:?};", k, v)),
                &Number(ref k, ref v) => str.push_str(&format!("[\"{}\"]={};", k, v)),
                &Boolean(ref k, true) => str.push_str(&format!("[\"{}\"]=true;", k)),
                &Boolean(ref k, false) => str.push_str(&format!("[\"{}\"]=true;", k)),
            }
        }
        str.push('}');

        str.push_str("},");

        str.push_str("uuid=\"");
        str.push_str(&self.uuid.hyphenated().to_string());
        str.push_str("\",");

        str.push_str("children={");
        let mut non_empty_list = false;
        for child in self.get_children() {
            str.push_str(&child.export_to_lua());
            str.push(',');
            non_empty_list = true;
        }
        if non_empty_list { str.pop(); }
        str.push('}');

        str.push('}');

        str

        /*
            {
                value = {
                    raw = %,
                    evaled = %,
                    attributes = {
                        ["%"] = %,
                        ["%"] = %
                    }
                },
                uuid = %,
                children = {
                    %,
                    %
                }
            }
        */
    }

    pub fn print(&self, evaled: bool) -> String {
        fn repeat(n: i32, str: String) -> String {
            if n > 0 {
                format!("{}{}", str.clone(), repeat(n-1, str.clone()))
            } else {
                String::from("")
            }
        }

        let mut str = String::new();

        for (indent, node) in self.traverse() {
            let text = if evaled {
                node.value.evaled.clone().unwrap_or(node.value.raw.clone())
            } else {
                node.value.raw.clone()
            };

            str.push_str(&format!("{}{}\n", repeat(indent, String::from("    ")), text));
        }

        str
    }

    fn export_attributes(&self) -> String {
        let mut str = String::new();

        for attr in &self.value.attributes {
            use node::Attribute::*;
            match attr {
                &String(ref k, ref v) => str = format!("{}{}={:?};", str, k, v),
                &Number(ref k, ref v) => str = format!("{}{}={};", str, k, v),
                &Boolean(ref k, true) => str = format!("{}{}=T;", str, k),
                &Boolean(ref k, false) => str = format!("{}{}=F;", str, k),
            }
        }

        str
    }
}
