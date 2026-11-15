mod footer;
mod header;
mod help;
mod tabs;

pub(crate) use footer::render_footer;
pub(crate) use header::{render_header, FULL_HEADER_HEIGHT};
pub(super) use help::render_help;
pub(crate) use tabs::render_tabs;

#[cfg(test)]
mod tests {
    include!("tests.rs");
}
