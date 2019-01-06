
use crate::errors::ChryError;

pub mod common;
pub mod impls;
pub mod user_switch;



pub trait OptionValue {
    fn toggle(&mut self) -> Result<(), ChryError> {
        let v = self.is_enabled()?;
        if v {
            self.disable()
        } else {
            self.enable()
        }
    }

    fn enable(&mut self) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("enable"))
    }

    fn disable(&mut self) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("disable"))
    }

    fn is_enabled(&self) -> Result<bool, ChryError> {
        Err(ChryError::NotSupported("is_enabled"))
    }

    fn set(&mut self, _: &str) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("set"))
    }

    fn unset(&mut self) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("unset"))
    }

    fn cycle(&mut self, _: bool, _: usize, _: &[String]) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("cycle"))
    }

    fn increment(&mut self, _: usize) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("increment"))
    }

    fn decrement(&mut self, _: usize) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("decrement"))
    }

    fn set_from_count(&mut self, _: Option<usize>) -> Result<(), ChryError> {
        Err(ChryError::NotSupported("set_from_count"))
    }
}
