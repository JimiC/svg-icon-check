#[path = "main.rs"]
mod main;

mod constants {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/constants.rs"));
}

macro_rules! tested_module {
    ($name:ident) => {
        mod $name {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/",
                stringify!($name),
                ".rs"
            ));

            mod tests {
                include!(concat!(stringify!($name), ".rs"));
            }
        }
    };
}

tested_module!(cli);
tested_module!(jobs);
tested_module!(output);
tested_module!(render);
tested_module!(utils);
