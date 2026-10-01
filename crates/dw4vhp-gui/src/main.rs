//! The `dw4-veryhard-plus` binary.
//!
//! The window itself arrives in Task 13, which replaces this placeholder with
//! the `eframe` app. For now the binary only has to exist and link the same
//! crate the tests exercise.

fn main() {
    println!(
        "dw4-veryhard-plus {} — the window is not implemented yet",
        env!("CARGO_PKG_VERSION")
    );
}
