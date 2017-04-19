
use std::str::FromStr;
use std::fmt::Debug;


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum OptionUpdateMethod {
    Enable,
    Disable,
    Toggle,
    Cycle,
}


pub trait OptionValue : Sized + PartialEq + Clone + FromStr + Debug {
    /* [enabled, disabled, ...other...] */
    fn default_series<'a>() -> &'a [Self];

    fn or_default(series: &[Self]) -> &[Self] {
        if series.len() <= 1 {
            Self::default_series()
        } else {
            series
        }
    }

    fn update(&mut self, method: &OptionUpdateMethod) {
        use self::OptionUpdateMethod::*;

        match *method {
            Enable => self.enable(Self::default_series()),
            Disable => self.disable(Self::default_series()),
            Toggle => self.toggle(Self::default_series()),
            Cycle => self.cycle(Self::default_series()),
        }
    }

    fn cycled(&self, series: &[Self]) -> Self {
        let series = Self::or_default(series);
        if let Some(index) = series.iter().position(|it| it == self) {
            if let Some(result) = series.iter().nth(index + 1) {
                return result.clone()
            }
        }
        series[0].clone()
    }

    fn disable(&mut self, series: &[Self]) {
        let series = Self::or_default(series);
        *self = series[0].clone();
    }

    fn enable(&mut self, series: &[Self]) {
        let series = Self::or_default(series);
        *self = series[1].clone();
    }

    fn is_enabled(&self) -> bool {
        Self::default_series()[0] != *self
    }

    fn toggle(&mut self, series: &[Self]) {
        println!("toggle: self: {:?}", self);
        let series = Self::or_default(series);
        if series[0] == *self {
            self.enable(series);
        } else {
            self.disable(series);
        }
    }

    fn cycle(&mut self, series: &[Self]) {
        let series = Self::or_default(series);
        *self = self.cycled(series);
    }
}


macro_rules! boolean_option {
    ($name:ident, $default:ident) => {

        #[derive(PartialEq, Eq, Clone, Copy, Debug)]
        pub enum $name {
            Enabled,
            Disabled
        }

        const $default: &'static [$name] = &[$name::Enabled, $name::Disabled];

        impl option::OptionValue for $name {
            fn default_series() -> &'static [$name] {
                $default
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(src: &str) -> Result<$name, String> {
                match &*src.to_lowercase() {
                    "yes" | "enable" | "true" | "0" => Ok($name::Enabled),
                    "no" | "disable" | "false" | "1" => Ok($name::Disabled),
                    _ => Err(format!("Invalid value: {}", src))
                }
            }
        }
    }
}
