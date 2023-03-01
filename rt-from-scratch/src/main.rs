pub mod timer;
pub mod yield_now;

mod block_on;
mod mt;
mod st;

fn main() {
    // block_on::main();
    // st::main();
    mt::main();
}
