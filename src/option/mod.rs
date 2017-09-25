
use std::result;

use errors::ChryError;

pub mod impls;
pub mod user;



pub type StdResult<T, U> = result::Result<T, U>;
pub type Result = StdResult<(), ChryError>;


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
        chry_error!(o!("Not supported operation"))
    }

    fn disable(&mut self) -> Result {
        chry_error!(o!("Not supported operation"))
    }

    fn is_enabled(&self) -> StdResult<bool, String> {
        chry_error!(o!("Not supported operation"))
    }

    fn set(&mut self, _: &str) -> Result {
        chry_error!(o!("Not supported operation"))
    }

    fn unset(&mut self) -> Result {
        chry_error!(o!("Not supported operation"))
    }

    fn cycle(&mut self, _: bool) -> Result {
        chry_error!(o!("Not supported operation"))
    }
}
