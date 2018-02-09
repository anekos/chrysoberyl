
#[derive(Default)]
pub struct Detector {
    previous_error: Option<String>,
    count: u8
}


const LIMIT: u8 = 10;


impl Detector {
    pub fn in_loop(&mut self, error: &str) -> bool {
        let new_value = match self.previous_error {
            Some(ref previous_error) => {
                if previous_error == error {
                    self.count += 1;
                    return LIMIT < self.count;
                }
                o!(error)
            },
            None => o!(error),
        };

        self.previous_error = Some(new_value);
        false
    }
}
