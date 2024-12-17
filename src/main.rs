use std::{env, num::NonZeroUsize};

use bfa::{Program, Table};

fn main() -> Result<(), String> {
    let mut args = env::args();
    if args.len() != 3 {
        return Err(format!(
            "Usage: {} <cell-count> <program>",
            args.next().unwrap_or_default()
        ));
    }

    args.next();

    let cell_count = args
        .next()
        .unwrap()
        .parse::<NonZeroUsize>()
        .map_err(|e| format!("Invalid cell count: {e}"))?;

    let program_text = args.next().unwrap();
    let program = Program::new(&program_text, cell_count);

    let mut table = Table::build(&program);
    table.minimize();
    println!("{}", table.dot());

    Ok(())
}
