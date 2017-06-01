
use std::result;

pub mod impls;
pub mod user;



pub type StdResult<T, U> = result::Result<T, U>;
pub type Result = StdResult<(), String>;


pub trait OptionValue {
    fn toggle(&mut self) -> Result {
        self.is_enabled().and_then(|v| {
            if v {
                self.disable()
            } else {
                self.enable()
            }
        })
    }

    fn enable(&mut self) -> Result {
        Err(o!("Not supported operation"))
    }

    fn disable(&mut self) -> Result {
        Err(o!("Not supported operation"))
    }

    fn is_enabled(&self) -> StdResult<bool, String> {
        Err(o!("Not supported operation"))
    }

    fn set(&mut self, _: &str) -> Result {
        Err(o!("Not supported operation"))
    }

    fn unset(&mut self) -> Result {
        Err(o!("Not supported operation"))
    }

    fn cycle(&mut self, _: bool) -> Result {
        Err(o!("Not supported operation"))
    }
}
