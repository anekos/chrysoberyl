

use std::default::Default;
use std::time::Duration;

use curl::easy::Easy as EasyCurl;



macro_rules! set {
    ($self:ident, $curl:ident, $property:ident) => {
        if let Some(v) = $self.$property {
            $curl.$property(v).unwrap();
        }
    };
    ($self:ident, $curl:ident, $property:ident, $modifier:ident) => {
        if let Some(v) = $self.$property {
            $curl.$property($modifier(v)).unwrap();
        }
    }
}

// http://php.net/manual/ja/function.curl-setopt.php
#[derive(Clone, Debug, PartialEq)]
pub struct CurlOptions {
    pub connect_timeout: Option<u64>,
    pub follow_location: bool,
    pub low_speed_limit: Option<u32>,
    pub low_speed_time: Option<u64>,
    pub timeout: Option<u64>,
}


impl CurlOptions {
    pub fn generate(&self) -> EasyCurl {
        let mut curl = EasyCurl::new();
        self.apply(&mut curl);
        println!("curl: {:?}", self);
        curl
    }

    fn apply(&self, curl: &mut EasyCurl) {
        set!(self, curl, connect_timeout, sec);
        set!(self, curl, low_speed_limit);
        set!(self, curl, low_speed_time, sec);
        set!(self, curl, timeout, sec);
        curl.follow_location(self.follow_location).unwrap();
    }
}

impl Default for CurlOptions {
    fn default() -> Self {
        CurlOptions {
            connect_timeout: Some(10),
            follow_location: true,
            low_speed_limit: Some(1024),
            low_speed_time: Some(10),
            timeout: None,
        }
    }
}

fn sec(v: u64) -> Duration {
    Duration::from_secs(v)
}
