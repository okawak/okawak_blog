//! Application-owned browser assets registered with the framework bundle.

use topcoat::asset::{Asset, asset};

pub const STYLESHEET: Asset = topcoat::tailwind::stylesheet!();
pub const FAVICON: Asset = asset!("../public/favicon.ico", rename: "favicon");
