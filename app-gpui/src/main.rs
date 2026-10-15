use beacn_linx_core::worker::Connection;
use std::path::PathBuf;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "LinX - GPUI Kit preview\nRead-only application discovery; no USB or desktop apply.\nOptional: --private-runtime PATH --core NAME"
        );
        return;
    }
    let connection = match args.as_slice() {
        [] => Connection::Desktop,
        [a, r, b, c] if a == "--private-runtime" && b == "--core" => Connection::Private {
            runtime: PathBuf::from(r),
            core: c.clone(),
        },
        _ => {
            eprintln!("Unknown arguments. Use --help.");
            std::process::exit(2);
        }
    };
    beacn_linx_gpui::ui::run(connection);
}
