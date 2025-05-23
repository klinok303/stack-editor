pub type GraphemeIdx = usize;
pub type LineIdx = usize;
pub type ByteIdx = usize;
pub type Col = usize;
pub type Row = usize;

mod position;
pub use position::Position;
mod size;
pub use size::Size;
mod location;
pub use location::Location;
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const QUIT_TIMES: u8 = 3;
