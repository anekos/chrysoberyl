
use std::cell::RefCell;
use std::env;
use std::rc::Rc;

use marksman_escape::Escape;
use mrusty::{Mruby, MrubyImpl, Value, MrubyFile};
use regex::{Regex, Captures};
use xml;

use crate::app::info::AppInfo;
use crate::constant;
use crate::errors::AppResult;



pub struct MRubyEnv {
    mruby: Rc<RefCell<Mruby>>,
}


impl MRubyEnv {
    pub fn generate_string(source: &str) -> AppResult<String> {
        let instance = MRubyEnv::new();
        let result = instance.eval(source)?;
        Ok(o!(result.to_str()?))
    }

    pub fn generate_string_from_template(source: &str) -> AppResult<String> {
        let instance = MRubyEnv::new();
        let re = Regex::new(r"\$\{(.*?)\}\$").unwrap();

        let result = re.replace_all(source, |caps: &Captures| {
            let source = format!("proc {{ {} }}[].to_s", &caps[1]);
            match instance.eval(&source) {
                Ok(result) => match result.to_str() {
                    Ok(result) => s!(xml::escape::escape_str_attribute(result)),
                    Err(err) => s!(err),
                },
                Err(err) => s!(err),
            }
        });

        Ok(s!(result))
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

        MRubyEnv { mruby }
    }


    pub fn eval(&self, source: &str) -> AppResult<Value> {
        let result = self.mruby.run(source);
        Ok(o!(result?))
    }

    #[allow(dead_code)]
    pub fn eval_as_bool(&self, source: &str) -> AppResult<bool> {
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

fn fetch_escaped_var(mruby: &Rc<RefCell<Mruby>>, name: &Value) -> AppResult<Value> {
    let name = name.to_str()?;
    let name = constant::env_name(name);
    let value = env::var(name)?;
    let escaped = String::from_utf8(Escape::new(value.bytes()).collect())?;
    Ok(mruby.string(&escaped))
}
