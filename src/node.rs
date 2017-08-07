use rlua;
use rlua::Lua;
use uuid::Uuid;

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

    pub fn export_to_sofer(&self) -> String {
        fn to_vec(n: &TreeNode) -> Vec<(Uuid, Uuid, String, String)> {
            let mut treenodes = Vec::new();
            treenodes.push((n.uuid, Uuid::nil(), n.export_attributes(), n.value.raw.clone()));
            treenodes.append(&mut to_vec_children(&n));

            for sibling in n.get_siblings() {
                treenodes.push((sibling.uuid, Uuid::nil(), sibling.export_attributes(), sibling.value.raw.clone()));
                treenodes.append(&mut to_vec_children(&sibling));
            }

            treenodes
        }

        fn to_vec_children(n: &TreeNode) -> Vec<(Uuid, Uuid, String, String)> {
            let mut treenodes = Vec::new();
            for child in n.get_children() {
                treenodes.push((child.uuid, n.uuid, child.export_attributes(), child.value.raw.clone()));
                treenodes.append(&mut to_vec_children(&child));
            }
            treenodes
        }

        let mut str = String::new();
        let mut vec = to_vec(self);
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
