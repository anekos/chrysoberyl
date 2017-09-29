
use std::cell::RefCell;
use std::env;
use std::error;
use std::rc::Rc;

use mrusty::{Mruby, MrubyImpl, Value};

use constant;



#[allow(unused_variables)]
pub fn generate(source: &str) -> Result<String, Box<error::Error>> {
    let mruby = Mruby::new();

    let chry_mod = mruby.def_class("Chrysoberyl");

    mruby.def_class_method(chry_mod.clone(), "env", mrfn!(|mruby, slf: Value, v: (&str)| {
        match env::var(v) {
            Ok(s) => mruby.string(&s),
            Err(_) => mruby.nil(),
        }
    }));

    mruby.def_class_method(chry_mod.clone(), "env", mrfn!(|mruby, slf: Value, v: (&str), def: (&str)| {
        match env::var(v) {
            Ok(s) => mruby.string(&s),
            Err(_) => mruby.string(def),
        }
    }));

    mruby.def_class_method(chry_mod, "method_missing", mrfn!(|mruby, slf: Value, name: Value| {
        fetch_var(&mruby, &name).unwrap_or_else(|_| mruby.nil())
    }));

    let source = format!("Chrysoberyl.instance_eval {{ {} }}", source);
    let result: Value = mruby.run(&source)?;
    let result = result.to_str()?;
    Ok(o!(result))
}


fn fetch_var(mruby: &Rc<RefCell<Mruby>>, name: &Value) -> Result<Value, Box<error::Error>> {
    let name = name.to_str()?;
    let name = constant::env_name(name);
    let value= env::var(name)?;
    Ok(mruby.string(&value))
}
