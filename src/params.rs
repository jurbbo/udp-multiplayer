use std::env;

pub struct Params {
    args: Vec<String>,
    has_valid_args: bool,
}

impl Params {
    pub fn new() -> Params {
        Params {
            args: env::args().collect(),
            has_valid_args: false,
        }
    }
    pub fn has_param(&mut self, param: String) -> bool {
        let param_first_letter_maybe = param.get(0..1);
        if param_first_letter_maybe.is_none() {
            return false;
        }

        let param_first_letter = param_first_letter_maybe.unwrap();

        let param_dashes = "--".to_string() + &param;
        let param_dash = "-".to_string() + &param_first_letter;
        if self.args.contains(&param)
            || self.args.contains(&param_dashes)
            || self.args.contains(&param_dash)
        {
            self.has_valid_args = true;
            return true;
        }
        return false;
    }

    pub fn display_help(&self) {
        if !self.has_valid_args {
            println!("");
            println!("Usage: cargo run [options]");
            println!("server    Run as server.");
            println!("client    Run as client.");
        }
    }
}
