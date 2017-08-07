use std::collections::BTreeMap;

use rlua;

#[derive(Clone, Debug)]
pub struct Config<'a> {
    pub keybindings: BTreeMap<i32, rlua::LuaFunction<'a>>,
}

pub fn read_config<'a>(str: String, lua: &'a rlua::Lua) -> Result<Config<'a>, rlua::LuaError> {
    let _ = lua.exec::<()>(&str, None);

    let globals = lua.globals();

    let config: rlua::LuaTable = globals.get("config")?;

    let mut keybindings = BTreeMap::new();

    let keybindings_lua: rlua::LuaTable = config.get("keybindings")?;
    let mut pairs = keybindings_lua.pairs::<i32, rlua::LuaFunction>();
    loop {
        match pairs.next() {
            Some(Ok((k, v))) => { keybindings.insert(k, v); },
            None => break,
            _ => {},
        }
    }

    Ok(Config { keybindings })
}

#[cfg(test)]
mod tests {
    use rlua;
    use std::collections::BTreeMap;

    #[test]
    fn read_lua_table() {
        macro_rules! map(
            { $($key:expr => $value:expr),+ } => {
                {
                    let mut m = ::std::collections::BTreeMap::new();
                    $(
                        m.insert($key, $value);
                    )+
                    m
                }
            };
        );

        let lua = rlua::Lua::new();
        let _ = lua.exec::<()>("table = { 1, 2, a = 2, b = 3 }", None);

        let mut table_fields: BTreeMap<String, i32> = BTreeMap::new();
        let mut table_array: Vec<i32> = Vec::new();
        let table_lua = lua.globals().get::<_, rlua::LuaTable>("table").expect("reading table");
        let mut table_lua_iter = table_lua.pairs::<rlua::LuaValue, i32>();
        loop {
            match table_lua_iter.next() {
                Some(Ok((rlua::LuaValue::String(k), v))) => {
                    table_fields.insert(String::from(k.to_str().unwrap()), v);
                }
                Some(Ok((rlua::LuaValue::Integer(_), v))) => {
                    table_array.push(v);
                }
                None | Some(Err(_)) => break,
                _ => {},
            }
        }

        assert_eq!(table_fields, map![String::from("a") => 2, String::from("b") => 3]);
        assert_eq!(table_array, vec![1, 2]);
    }

    #[test]
    fn function_from_table() {
        let lua = rlua::Lua::new();
        let _ = lua.exec::<()>(
            r#"
                table = {
                    add = function (x, y) return x+y end,
                    pow = function (x) return x*x end
                }
            "#,
            None
        );

        let table = lua.globals().get::<&str, rlua::LuaTable>("table").unwrap();

        let add: rlua::LuaFunction = table.get("add").unwrap();
        let pow: rlua::LuaFunction = table.get("pow").unwrap();

        assert_eq!(add.call::<_ ,i32>(hlist![1, 2]).unwrap(), 1 + 2);
        assert_eq!(pow.call::<_ ,i32>(17).unwrap(), 17 * 17);
    }

    #[test]
    fn iterate_over_table() {
        let lua = rlua::Lua::new();
        let _ = lua.exec::<()>(
            r#"
                table = {
                    a = "a",
                    b = "b"
                }
            "#,
            None
        );

        let table = lua.globals().get::<&str, rlua::LuaTable>("table").unwrap();
        let values = table.pairs::<String, String>().map(|x| x.unwrap()).collect::<Vec<_>>();

        assert!(
            values == vec![("a".into(), "a".into()), ("b".into(), "b".into())]
                || values == vec![("b".into(), "b".into()), ("a".into(), "a".into())]
        );
    }
}
