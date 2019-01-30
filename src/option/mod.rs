use crate::errors::{AppResult, AppResultU, AppError};

pub mod impls;
pub mod common;
pub mod user_switch;




pub trait OptionValue {
    fn toggle(&mut self) -> AppResultU {
        let v = self.is_enabled()?;
        if v {
            self.disable()
        } else {
            self.enable()
        }
    }

    fn enable(&mut self) -> AppResultU {
        Err(AppError::NotSupported("enable"))
    }

    fn disable(&mut self) -> AppResultU {
        Err(AppError::NotSupported("disable"))
    }

    fn is_enabled(&self) -> AppResult<bool> {
        Err(AppError::NotSupported("is_enabled"))
    }

    fn set(&mut self, _: &str) -> AppResultU {
        Err(AppError::NotSupported("set"))
    }

    fn unset(&mut self) -> AppResultU {
        Err(AppError::NotSupported("unset"))
    }

    fn cycle(&mut self, _: bool, _: usize, _: &[String]) -> AppResultU {
        Err(AppError::NotSupported("cycle"))
    }

    fn increment(&mut self, _: usize) -> AppResultU {
        Err(AppError::NotSupported("increment"))
    }

    fn decrement(&mut self, _: usize) -> AppResultU {
        Err(AppError::NotSupported("decrement"))
    }

    fn set_from_count(&mut self, _: Option<usize>) -> AppResultU {
        Err(AppError::NotSupported("set_from_count"))
    }
}
