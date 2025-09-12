mod app;
mod combo;
mod compositor;
mod event;
mod notifications;
mod utils;
mod views;
mod widgets;

pub use app::App;
pub use event::Event;

/// call builder methods on a builder types if given condition
/// is true, the macro takes a pair of condition and the builder method
/// to call
///
/// ```
/// let block = conditional_build(
///     Block::bordered(),
///     (x < y, (title_top("y is bigger")) else title_top("x is bigger")),
///     (focused, (style(Style::new().yellow()))
/// )
/// ```
#[macro_export]
macro_rules! conditional_build {
    ($builder:expr, $(($condition:expr, ($($method:tt)*) $(else $($else_method:tt)*)?)),+) => {
        {
            let item = $builder;

            $(
                let item = if $condition {
                    item.$($method)*
                } else {
                    item$(.$($else_method)*)?
                };
            )*
            item
        }
    };
}
