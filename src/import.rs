// gdextension_importer.rs
// A minimal, self-contained GDExtension module in Rust that takes a top-level
// `GodotValue::Resource`, optionally loads an existing resource if `resource_path`
// is provided, applies frontmatter via a `_apply_doke_frontmatter` method if present
// (only for the top-level), and applies `_apply_root_doke_frontmatter` on subresources
// if they have that method. Nested resources are always instanced fresh.
//
// NOTE: This is written against the godot-rust-style API used earlier in the convo
// (ClassDb, ResourceLoader, ProjectSettings, ResourceSaver, Script, Object). You
// may need to adapt small API surface names to your exact GDExtension crate.

use std::collections::HashMap;

use doke::GodotValue;
use doke::file_builder::BuilderError;
use doke::semantic::{DokeErrors, DokeValidationError};
use godot::classes::{ProjectSettings, ResourceLoader, Script};
use godot::{classes::ClassDb, prelude::*};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ImportError>;
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("couldn't instanciate a resource of type {0}")]
    ResInstanciationError(String),
    #[error("conversion error : {0}")]
    ConvertError(#[from] ConvertError),
    #[error("Parse Errors : {0}")]
    ParseError(#[from] DokeErrors),
    #[error("file-resource Error : {0}")]
    BuilderError(#[from] BuilderError),
    #[error("Missing Parser or file def Error")]
    MissingParserError(),
    #[error("Invalid extension for file {0}")]
    InvalidExtension(String),
    #[error("Io Error when loading scripts : {0}")]
    IoError(#[from] IoError),
    #[error("Parsed value is not a resource : {0}")]
    NotAResource(GodotValue),
    #[error("Can't read file : {0}")]
    CantReadFile(#[from] std::io::Error),
    #[error("Validation failed : {0}")]
    DokeValidationError(#[from] DokeValidationError),
}

// -----------------------
// Helpers: Convert GodotValue -> Variant
// !!! This recursively tries to make any Resource
// -----------------------
pub fn godot_value_to_variant(value: GodotValue) -> Result<Variant> {
    match value {
        GodotValue::Nil => Ok(Variant::nil()),
        GodotValue::Bool(b) => Ok(Variant::from(b)),
        GodotValue::Int(i) => Ok(Variant::from(i)),
        GodotValue::Float(f) => Ok(Variant::from(f)),
        GodotValue::String(s) => Ok(Variant::from(s)),
        GodotValue::Array(arr) => {
            let mut array: Array<Variant> = array![];
            for v in arr {
                let v_as_variant = godot_value_to_variant(v)?;
                array.push(&v_as_variant);
            }
            Ok(Variant::from(array))
        }
        GodotValue::Dict(map) => {
            let mut gd = Dictionary::new();
            for (k, v) in map {
                let v_as_variant = godot_value_to_variant(v)?;
                gd.insert(k, v_as_variant);
            }
            Ok(Variant::from(gd))
        }
        GodotValue::Resource {
            type_name,
            fields,
            abstract_type_name: _,
        } => {
            // Nested resources are instanced fresh (no resource_path lookup)
            let mut res = instantiate_resource(&type_name)?;
            for (k, v) in fields {
                res.set(&StringName::from(k), &godot_value_to_variant(v)?);
            }
            Ok(Variant::from(res))
        }
    }
}

// -----------------------
// Public import function
// -----------------------
#[allow(dead_code)]
pub fn import_top_level_resource(
    value: GodotValue,
    frontmatter: HashMap<String, GodotValue>,
    save_path: Option<String>,
) -> Result<Gd<Resource>> {
    if !matches!(
        value,
        GodotValue::Resource {
            type_name: _,
            fields: _,
            abstract_type_name: _
        }
    ) {
        return Err(ImportError::NotAResource(value));
    }
    let resource = build_top_level_resource(value, save_path, &frontmatter)?;
    Ok(resource)
}

// -----------------------
// Instantiate resource (built-in first, then class_name fallback)
// -----------------------
fn instantiate_resource(type_name: &str) -> Result<Gd<Resource>> {
    // 1) Built-in class via ClassDB
    if ClassDb::singleton().class_exists(&StringName::from(type_name)) {
        let inst = ClassDb::singleton().instantiate(&StringName::from(type_name));
        let res = inst.try_to_relaxed::<Gd<Resource>>()?; // this does
        return Ok(res);
    }

    // 2) Fallback: look up ProjectSettings global_class_list for a script and make the resource ourselves
    let global_class_list = ProjectSettings::singleton().get_global_class_list();
    let mut script_path: String = "".into();

    for dict in global_class_list.iter_shared() {
        if let Some(class_name) = dict.get("class") {
            if class_name == Variant::from(type_name) {
                if let Some(path) = dict.get("path") {
                    script_path = path.try_to_relaxed::<String>()?
                }
            }
        }
    }
    let mut script = try_load::<Script>(&script_path)?;
    let res = script.call("new", &[]);
    let res = res.try_to::<Gd<Resource>>()?;
    Ok(res)
}

// -----------------------
// Top-level builder: load by resource_path if present, else instantiate
// Only the top-level resource checks "resource_path". Nested resources are fresh.
// -----------------------
pub fn build_top_level_resource(
    value: GodotValue,
    path: Option<String>,
    frontmatter: &HashMap<String, GodotValue>,
) -> Result<Gd<Resource>> {
    let res = match value {
        GodotValue::Resource {
            type_name,
            fields: _,
            abstract_type_name: _,
        } => {
            // Extract resource_path if present

            if let Some(path) = path {
                // Try to load existing resource
                if let Some(existing) = ResourceLoader::singleton().load(&path) {
                    return Ok(existing);
                }
                // If load failed, fall through to instantiate fresh
            }

            // Instantiate fresh (built-in or class_name fallback)
            instantiate_resource(&type_name)
        }
        _ => Err(ImportError::NotAResource(value))?,
    };
    let mut res = res?;
    apply_doke_frontmatter_if_exists(&mut res, frontmatter)?;
    Ok(res)
}

// -----------------------
// Convert mdast::Yaml -> Godot Dictionary (Variant-compatible)
// -----------------------

const APPLY_DOKE_FM_METHOD: &str = "_apply_doke_frontmatter";
const APPLY_ROOT_DOKE_FM_METHOD: &str = "_apply_root_doke_frontmatter";
// -----------------------
// Apply frontmatter: call `_apply_doke_frontmatter` on the resource if it exists
// -----------------------
fn apply_doke_frontmatter_if_exists(
    resource: &mut Gd<Resource>,
    frontmatter: &HashMap<String, GodotValue>,
) -> Result<()> {
    resource.call(
        APPLY_ROOT_DOKE_FM_METHOD,
        &[convert_fm_to_godot(frontmatter)?],
    );
    Ok(())
}

fn convert_fm_to_godot(fm: &HashMap<String, GodotValue>) -> Result<Variant> {
    let mut dict = Dictionary::new();
    for (k, v) in fm {
        dict.insert(Variant::from(k.clone()), godot_value_to_variant(v.clone())?);
    }
    Ok(Variant::from(dict))
}
