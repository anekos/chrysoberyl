
use std::result;

use errors::ChryError;

pub mod common;
pub mod impls;
pub mod user;



pub type StdResult<T, U> = result::Result<T, U>;
pub type Result = StdResult<(), ChryError>;


pub trait OptionValue {
    fn toggle(&mut self) -> Result {
        let v = self.is_enabled()?;
        if v {
            self.disable()
        } else {
            self.enable()
        }
    }

    fn enable(&mut self) -> Result {
        Err(ChryError::NotSupported("enable"))
    }

    fn disable(&mut self) -> Result {
        Err(ChryError::NotSupported("disable"))
    }

    fn is_enabled(&self) -> StdResult<bool, ChryError> {
        Err(ChryError::NotSupported("is_enabled"))
    }

    fn set(&mut self, _: &str) -> Result {
        Err(ChryError::NotSupported("set"))
    }

    fn unset(&mut self) -> Result {
        Err(ChryError::NotSupported("unset"))
    }

    fn cycle(&mut self, _: bool) -> Result {
        Err(ChryError::NotSupported("cycle"))
    }
}
