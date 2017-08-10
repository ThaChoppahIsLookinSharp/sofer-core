use rlua;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct Tree<T>{
    pub value: T,
    pub uuid: Uuid,
    pub first_child: Option<Box<Tree<T>>>,
    pub next_sibling: Option<Box<Tree<T>>>,
}

impl<T> Tree<T>
    where T: Clone {
    pub fn new_tree(value: T) -> Tree<T> {
        Tree {
            value,
            uuid: Uuid::nil(),
            first_child: None,
            next_sibling: None,
        }
    }

    pub fn new_child(value: T) -> Tree<T> {
        Tree {
            value,
            uuid: Uuid::new_v4(),
            first_child: None,
            next_sibling: None,
        }
    }

    pub fn insert(&mut self, parent_uuid: Uuid, new_node: Tree<T>) -> bool {
        if self.uuid == parent_uuid {
            match self.first_child {
                Some(ref mut n) => {
                    n.insert_to_sibling(new_node);
                    true
                }
                None => {
                    self.first_child = Some(Box::new(new_node));
                    true
                }
            }
        } else {
            let inserted_under_first_child =
                match self.first_child {
                    Some(ref mut n) => n.insert(parent_uuid, new_node.clone()),
                    None => false,
                };
            if inserted_under_first_child {
                true
            } else {
                match self.next_sibling {
                    Some(ref mut n) => n.insert(parent_uuid, new_node),
                    None => false,
                }
            }
        }
    }

    pub fn insert_to_sibling(&mut self, new_node: Tree<T>) {
        match self.next_sibling {
            Some(ref mut n) => n.insert_to_sibling(new_node),
            None => self.next_sibling = Some(Box::new(new_node)),
        }
    }

    pub fn find(&self, uuid: Uuid) -> Option<&Tree<T>> {
        if self.uuid == uuid {
            Some(self)
        } else {
            match (&self.first_child, &self.next_sibling) {
                (&Some(ref first_child), &Some(ref next_sibling)) =>
                    first_child.find(uuid).or(next_sibling.find(uuid)),

                (&Some(ref first_child), &None) =>
                    first_child.find(uuid),

                (&None, &Some(ref next_sibling)) =>
                    next_sibling.find(uuid),

                (&None, &None) => None,
            }
        }
    }

    pub fn traverse(&self) -> Vec<(i32, Tree<T>)> {
        let mut vec = vec![(0, self.clone())];
        let mut children = self.traverse_children();
        let mut siblings = self.traverse_siblings();
        vec.append(&mut children);
        vec.append(&mut siblings);
        vec
    }

    fn traverse_children(&self) -> Vec<(i32, Tree<T>)> {
        match self.first_child {
            Some(ref n) => {
                n.traverse()
                    .iter()
                    .map(|&(i, ref content)| (i+1, content.clone()))
                    .collect()
            }
            None => vec![],
        }
    }

    fn traverse_siblings(&self) -> Vec<(i32, Tree<T>)> {
        match self.next_sibling {
            Some(ref n) => n.traverse(),
            None => vec![],
        }
    }

    pub fn get_children(&self) -> Vec<Tree<T>> {
        match self.first_child {
            Some(ref first_child) => {
                let mut siblings = vec![*first_child.clone()];
                let mut more_siblings = first_child.get_siblings();
                siblings.append(&mut more_siblings);
                siblings
            }
            None => Vec::new(),
        }
    }

    pub fn get_siblings(&self) -> Vec<Tree<T>> {
        let mut current_sibling = &self.next_sibling;
        let mut siblings = Vec::new();
        loop {
            match current_sibling {
                &Some(ref sibling) => {
                    siblings.push(*sibling.clone());
                    current_sibling = &sibling.next_sibling;
                }
                &None => return siblings,
            }
        }
    }
}

impl<'lua, T> rlua::ToLua<'lua> for Tree<T>
    where T: rlua::ToLua<'lua>, T: Clone {
    fn to_lua(self, lua: &'lua rlua::Lua) -> rlua::LuaResult<rlua::LuaValue> {
        let table = lua.create_table();
        table.set("value", self.clone().value)?;
        table.set("uuid", self.uuid.simple().to_string())?;
        table.set("children", self.get_children())?;
        Ok(rlua::LuaValue::Table(table))
    }
}

impl<'lua, T> rlua::FromLua<'lua> for Tree<T>
    where T: rlua::FromLua<'lua>, T: Clone {
    fn from_lua(lua_value: rlua::LuaValue<'lua>, _: &'lua rlua::Lua) -> rlua::LuaResult<Tree<T>> {
        match lua_value {
            rlua::LuaValue::Table(table) => {
                let value: T = table.get("value")?;
                let uuid_string: String = table.get("uuid")?;
                let uuid = Uuid::parse_str(&uuid_string).unwrap();
                let children: Vec<Tree<T>> = table.get("children")?;
                let mut tree = Tree {
                    value,
                    uuid,
                    first_child: None,
                    next_sibling: None,
                };
                for child in children {
                    tree.insert(uuid, child);
                }
                Ok(tree)
            }
            x => Err(rlua::LuaError::FromLuaConversionError(format!("Can't convert {:?} to Tree", x))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_insert() {
        let mut tree: Tree<String> = Tree::new_tree("parent".into());

        let first = Tree::new_child("first child".into());
        let first_first = Tree::new_child("first first child".into());
        let first_second = Tree::new_child("first second child".into());
        let first_second_first = Tree::new_child("first second first child".into());
        let second = Tree::new_child("second child".into());
        let second_first = Tree::new_child("second first child".into());

        tree.insert(Uuid::nil(), first.clone());
        tree.insert(first.uuid, first_first.clone());
        tree.insert(first.uuid, first_second.clone());
        tree.insert(first_second.uuid, first_second_first.clone());
        tree.insert(Uuid::nil(), second.clone());
        tree.insert(second.uuid, second_first.clone());
        assert_eq!(
            Tree {
                value: "parent".into(),
                uuid: tree.uuid,
                next_sibling: None,
                first_child: Some(Box::new(Tree {
                    value: "first child".into(),
                    uuid: first.uuid,
                    first_child: Some(Box::new(Tree {
                        value: "first first child".into(),
                        uuid: first_first.uuid,
                        first_child: None,
                        next_sibling: Some(Box::new(Tree {
                            value: "first second child".into(),
                            uuid: first_second.uuid,
                            next_sibling: None,
                            first_child: Some(Box::new(Tree {
                                value: "first second first child".into(),
                                uuid: first_second_first.uuid,
                                first_child: None,
                                next_sibling: None,
                            })),
                        })),
                    })),
                    next_sibling: Some(Box::new(Tree {
                        value: "second child".into(),
                        uuid: second.uuid,
                        next_sibling: None,
                        first_child: Some(Box::new(Tree {
                            value: "second first child".into(),
                            uuid: second_first.uuid,
                            first_child: None,
                            next_sibling: None,
                        }))
                    })),
                })),
            },
            tree
        )
    }


    #[test]
    fn tree_find() {
        let mut tree: Tree<String> = Tree::new_tree("parent".into());

        let first = Tree::new_child("first child".into());
        let first_first = Tree::new_child("first first child".into());
        let first_second = Tree::new_child("first second child".into());
        let first_second_first = Tree::new_child("first second first child".into());
        let second = Tree::new_child("second child".into());
        let second_first = Tree::new_child("second first child".into());

        tree.insert(Uuid::nil(), first.clone());
        tree.insert(first.uuid, first_first.clone());
        tree.insert(first.uuid, first_second.clone());
        tree.insert(first_second.uuid, first_second_first.clone());
        tree.insert(Uuid::nil(), second.clone());
        tree.insert(second.uuid, second_first.clone());

        assert_eq!(tree.find(first.uuid).unwrap().value, "first child");
        assert_eq!(tree.find(first_first.uuid).unwrap().value, "first first child");
        assert_eq!(tree.find(first_second.uuid).unwrap().value, "first second child");
        assert_eq!(tree.find(first_second_first.uuid).unwrap().value, "first second first child");
        assert_eq!(tree.find(second.uuid).unwrap().value, "second child");
        assert_eq!(tree.find(second_first.uuid).unwrap().value, "second first child");

        for _ in 0..100 {
            let uuid  = Uuid::new_v4();
            for (_, n) in tree.traverse() {
                if n.uuid == uuid {
                    continue;
                }
            }
            assert!(tree.find(uuid).is_none())
        }
    }

    #[test]
    fn traverse() {
        let mut tree: Tree<String> = Tree::new_tree("parent".into());

        let first = Tree::new_child("first child".into());
        let first_first = Tree::new_child("first first child".into());
        let first_second = Tree::new_child("first second child".into());
        let first_second_first = Tree::new_child("first second first child".into());
        let second = Tree::new_child("second child".into());
        let second_first = Tree::new_child("second first child".into());

        tree.insert(Uuid::nil(), first.clone());
        tree.insert(first.uuid, first_first.clone());
        tree.insert(first.uuid, first_second.clone());
        tree.insert(first_second.uuid, first_second_first.clone());
        tree.insert(Uuid::nil(), second.clone());
        tree.insert(second.uuid, second_first.clone());

        assert_eq!(
            vec![
                (0 as i32, Tree { value: "parent".into(), uuid: tree.uuid, first_child: Some(Box::new(Tree { value: "first child".into(), uuid: first.uuid, first_child: Some(Box::new(Tree { value: "first first child".into(), uuid: first_first.uuid, first_child: None, next_sibling: Some(Box::new(Tree { value: "first second child".into(), uuid: first_second.uuid, first_child: Some(Box::new(Tree { value: "first second first child".into(), uuid: first_second_first.uuid, first_child: None, next_sibling: None })), next_sibling: None })) })), next_sibling: Some(Box::new(Tree { value: "second child".into(), uuid: second.uuid, first_child: Some(Box::new(Tree { value: "second first child".into(), uuid: second_first.uuid, first_child: None, next_sibling: None })), next_sibling: None })) })), next_sibling: None }),
                (1 as i32, Tree { value: "first child".into(), uuid: first.uuid, first_child: Some(Box::new(Tree { value: "first first child".into(), uuid: first_first.uuid, first_child: None, next_sibling: Some(Box::new(Tree { value: "first second child".into(), uuid: first_second.uuid, first_child: Some(Box::new(Tree { value: "first second first child".into(), uuid: first_second_first.uuid, first_child: None, next_sibling: None })), next_sibling: None })) })), next_sibling: Some(Box::new(Tree { value: "second child".into(), uuid: second.uuid, first_child: Some(Box::new(Tree { value: "second first child".into(), uuid: second_first.uuid, first_child: None, next_sibling: None })), next_sibling: None })) }),
                (2 as i32, Tree { value: "first first child".into(), uuid: first_first.uuid, first_child: None, next_sibling: Some(Box::new(Tree { value: "first second child".into(), uuid: first_second.uuid, first_child: Some(Box::new(Tree { value: "first second first child".into(), uuid: first_second_first.uuid, first_child: None, next_sibling: None })), next_sibling: None })) }),
                (2 as i32, Tree { value: "first second child".into(), uuid: first_second.uuid, first_child: Some(Box::new(Tree { value: "first second first child".into(), uuid: first_second_first.uuid, first_child: None, next_sibling: None })), next_sibling: None }),
                (3 as i32, Tree { value: "first second first child".into(), uuid: first_second_first.uuid, first_child: None, next_sibling: None }),
                (1 as i32, Tree { value: "second child".into(), uuid: second.uuid, first_child: Some(Box::new(Tree { value: "second first child".into(), uuid: second_first.uuid, first_child: None, next_sibling: None })), next_sibling: None }),
                (2 as i32, Tree { value: "second first child".into(), uuid: second_first.uuid, first_child: None, next_sibling: None })
            ],
            tree.traverse()
        )
    }

    #[test]
    fn tree_get_siblings() {
        let mut tree = Tree::new_tree("top");
        let first = Tree::new_child("first");
        let second = Tree::new_child("second");
        let third = Tree::new_child("third");
        let forth = Tree::new_child("forth");
        let fifth = Tree::new_child("fifth");
        tree.insert(Uuid::nil(), first.clone());
        tree.insert(Uuid::nil(), second.clone());
        tree.insert(Uuid::nil(), third.clone());
        tree.insert(Uuid::nil(), forth.clone());
        tree.insert(Uuid::nil(), fifth.clone());
        println!("tree = {:?}", tree);
        assert_eq!(
            tree.get_children().iter().map(|x| x.value).collect::<Vec<_>>(),
            vec!["first", "second", "third", "forth", "fifth"]
        )
    }

    #[test]
    fn tree_from_lua() {
        let lua_code = r#"
{
    value="",
    uuid="00000000-0000-0000-0000-000000000000",
    children={
        {
            value="caca de vaca @ function(node) return tostring(node.children[1].value.raw) end",
            uuid="00000000-0000-0000-0000-000000000001",
            children={}
        },
        {
            value="Esto es lo que he dicho: @ function(node) return node.value.raw end",
            uuid="00000000-0000-0000-0000-000000000002",
            children={}
        }
    }
}
        "#;
        let lua = rlua::Lua::new();
        let tree: Tree<String> = lua.eval(lua_code).unwrap();
        assert_eq!(
            tree,
            Tree {
                value: "".into(),
                uuid: Uuid::nil(),
                first_child: Some(
                    Box::new(
                        Tree {
                            value: "caca de vaca @ function(node) return tostring(node.children[1].value.raw) end".into(),
                            uuid: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                            first_child: None,
                            next_sibling: Some(
                                Box::new(
                                    Tree {
                                        value: "Esto es lo que he dicho: @ function(node) return node.value.raw end".into(),
                                        uuid: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                                        first_child: None,
                                        next_sibling: None,
                                    }
                                )
                            ),
                        }
                    )
                ),
                next_sibling: None,
            }
        )
    }
}
