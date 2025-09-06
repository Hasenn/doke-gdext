// doke_importer.rs
// GDExtension class to hold Rust Markdown parsers and provide a method
// to parse markdown files into Godot resources using previously defined import logic.
mod import;
use doke::{DokeParser, GodotValue};
use godot::prelude::*;

use std::sync::Arc;

// -----------------------
// NativeClass for Godot
// -----------------------
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DokeImporter {
    parsers: Vec<Arc<dyn DokeParser>>,
}

#[godot_api]
impl DokeImporter {

    /// Parse markdown file at `md_path` and returns a Resource to godot
    #[func]
    fn import_doke(&self, md_path : String) -> Gd<Resource> {
        
    }
}
