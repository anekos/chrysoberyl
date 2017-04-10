
use std::collections::HashMap;
use std::str::FromStr;

use operation::{self, Operation};



pub struct MouseMapping {
    table: HashMap<u32, Vec<WithArea>>
}

pub struct WithArea {
    operation: Vec<String>,
    area: Option<Area>
}

#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64
}


impl MouseMapping {
    pub fn new() -> MouseMapping {
        MouseMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, button: u32, area: Option<Area>, operation: &[String]) {
        let entry = WithArea { operation: operation.to_vec(), area: area.clone() };
        if area.is_some() {
            if let Some(mut entries) = self.table.get_mut(&button) {
                entries.retain(|it| it.area != area);
                entries.push(entry);
                return;
            }
        }
        self.table.insert(button, vec![entry]);
    }

    pub fn matched(&self, button: u32, x: i32, y: i32, width: i32, height: i32) -> Option<Result<Operation, String>> {
        self.table.get(&button).and_then(|entries| {
            let mut found = None;

            for entry in entries.iter() {
                if let Some(area) = entry.area.clone() {
                    if area.contains(x, y, width, height) {
                        found = Some(operation::parse_from_vec(&entry.operation));
                        break;
                    }
                } else if found.is_none() {
                    found = Some(operation::parse_from_vec(&entry.operation));
                }
            }

            found
        })
    }
}

impl Area {
    pub fn new(left: f64, top: f64, right: f64, bottom: f64) -> Area {
        Area { left: left, top: top, right: right, bottom: bottom }
    }

    fn contains(&self, x: i32, y: i32, width: i32, height: i32) -> bool {
        let l = (width as f64 * self.left) as i32;
        let r = (width as f64 * self.right) as i32;
        let t = (height as f64 * self.top) as i32;
        let b = (height as f64 * self.bottom) as i32;
        (l <= x && x <= r && t <= y && y <= b)
    }
}

impl FromStr for Area {
    type Err = String;

    fn from_str(src: &str) -> Result<Area, String> {
        let err = Err(o!("Invalid format (e.g. 0.0x0.0-1.0x1.0)"));

        let hyphen: Vec<&str> = src.split_terminator('-').collect();
        if hyphen.len() != 2 {
            return err;
        }

        let xs_from: Vec<&str> = hyphen[0].split_terminator('x').collect();
        let xs_to: Vec<&str> = hyphen[1].split_terminator('x').collect();

        if xs_from.len() != 2 || xs_to.len() != 2 {
            return err
        }

        if let (Ok(left), Ok(top), Ok(right), Ok(bottom)) = (xs_from[0].parse(), xs_from[1].parse(), xs_to[0].parse(), xs_to[1].parse()) {
            Ok(Area::new(left, top, right, bottom))
        } else {
            err
        }
    }
}
