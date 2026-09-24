//! Process-wide launch choice; remote pages never receive this authority.
use std::{ffi::OsString, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewerMode { #[default] TwitchPage, Embedded }
static MODE: OnceLock<ViewerMode> = OnceLock::new();

fn parse(args: impl IntoIterator<Item = OsString>) -> ViewerMode {
    if args.into_iter().any(|arg| arg == "--embedded-viewer") { ViewerMode::Embedded }
    else { ViewerMode::TwitchPage }
}
pub fn initialize(args: impl IntoIterator<Item = OsString>) {
    let _ = MODE.set(parse(args));
}
pub fn selected() -> ViewerMode { *MODE.get_or_init(ViewerMode::default) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_is_default_and_only_exact_launch_flag_enables_embedded() {
        assert_eq!(parse([]), ViewerMode::TwitchPage);
        for arg in ["embedded", "--embedded-viewer=false", "--twitch-page", "--EMBEDDED-VIEWER"] {
            assert_eq!(parse([arg.into()]), ViewerMode::TwitchPage);
        }
        assert_eq!(parse(["--embedded-viewer".into()]), ViewerMode::Embedded);
        assert_eq!(parse(["--other".into(), "--embedded-viewer".into()]), ViewerMode::Embedded);
    }
}
