
use std::cell::RefCell;
use std::env;
use std::error;
use std::rc::Rc;

use marksman_escape::Escape;
use mrusty::{Mruby, MrubyImpl, Value, MrubyFile};

use app::info::AppInfo;
use constant;



pub struct MRubyEnv {
    mruby: Rc<RefCell<Mruby>>,
}


impl MRubyEnv {
    pub fn generate_string(source: &str) -> Result<String, Box<error::Error>> {
        let instance = MRubyEnv::new();
        let result = instance.eval(source)?;
        Ok(o!(result.to_str()?))
    }

    #[allow(unused_variables)]
    pub fn new() -> Self {
        let mruby = Mruby::new();

        mruby_class!(mruby, "Chry", {
            def!("initialize", |mruby, slf: Value| {
                slf
            });

            def!("app", |mruby, slf: Value| {
                slf.get_var("app").unwrap_or_else(|| mruby.nil())
            });

            def_self!("const_missing", |mruby, slf: Value, name: Value| {
                fetch_escaped_var(&mruby, &name).unwrap_or_else(|_| mruby.nil())
            });
        });

        mruby_class!(mruby, "Env", {
            def_self!("const_missing", |mruby, slf: Value, name: Value| {
                let name = name.to_str().unwrap();
                match env::var(name) {
                    Ok(s) => mruby.string(&s),
                    Err(_) => mruby.nil(),
                }
            });
        });

        mrusty_class!(AppInfo, "AppInfo", {
            def!("pages", |mruby, slf: (&AppInfo)| {
                mruby.fixnum(slf.pages as i32)
            });

            def!("empty?", |mruby, slf: (&AppInfo)| {
                mruby.bool(slf.is_empty())
            });
            // TODO more methods
        });
        AppInfo::require(Rc::clone(&mruby));

        // TODO EntryInfo

        MRubyEnv { mruby: mruby }
    }


    pub fn eval(&self, source: &str) -> Result<Value, Box<error::Error>> {
        let result = self.mruby.run(source);
        Ok(o!(result?))
    }

    #[allow(dead_code)]
    pub fn eval_as_bool(&self, source: &str) -> Result<bool, Box<error::Error>> {
        let result = self.eval(source);
        let result = result?.to_bool()?;
        Ok(o!(result))
    }

    #[allow(dead_code)]
    pub fn set_app_info(&self, app_info: AppInfo) {
        let object = self.mruby.get_class("Object").unwrap();
        object.def_const("App", self.mruby.obj(app_info));
    }
}

fn fetch_escaped_var(mruby: &Rc<RefCell<Mruby>>, name: &Value) -> Result<Value, Box<error::Error>> {
    let name = name.to_str()?;
    let name = constant::env_name(name);
    let value = env::var(name)?;
    let escaped = String::from_utf8(Escape::new(value.bytes()).collect())?;
    Ok(mruby.string(&escaped))
}
