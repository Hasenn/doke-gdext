// doke_importer.rs
// GDExtension class to hold Rust Markdown parsers and provide a method
// to parse markdown files into Godot resources using previously defined import logic.
mod import;
use doke::{
    DokeParser, DokePipe, GodotValue,
    file_builder::{self, ResourceBuilder},
    parsers::{self, TypedSentencesParser},
};
use godot::{global::push_error, prelude::*};

use std::{collections::HashMap, io::BufRead, path::Path, sync::Arc};

use crate::import::ImportError;

// -----------------------
// NativeClass for Godot
// -----------------------
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DokeImporter {
    parsers: HashMap<String, Arc<DokePipe>>,
    builders: HashMap<String, Arc<ResourceBuilder>>,
}

#[godot_api]
impl DokeImporter {
    #[func]
    ///Loads parsers for a filetype
    fn load_parser_for_filetype(&mut self, file_type: String, config_path: String) -> i64 {
        return self.load_file_builder(file_type.clone(), config_path.clone())
            + self.load_sentence_parser(file_type, config_path);
    }
    // Load a TypedSentencesParser and add it to the parser map
    fn load_sentence_parser(&mut self, file_type: String, config_path: String) -> i64 {
        let typed_parser = TypedSentencesParser::from_config_file(&Path::new(&config_path));
        match typed_parser {
            Ok(parser) => {
                let pipe = DokePipe::new()
                    .add(parsers::FrontmatterTemplateParser)
                    .add(parser)
                    .add(parsers::DebugPrinter);
                self.parsers.insert(file_type, pipe.into());
                0
            }
            Err(e) => {
                push_error(&[Variant::from(e.to_string())]);
                1
            }
        }
    }

    // Load a ResourceBuilder from the same config file
    fn load_file_builder(&mut self, file_type: String, config_path: String) -> i64 {
        let builder = ResourceBuilder::from_file(&Path::new(&config_path));
        match builder {
            Ok(builder) => {
                self.builders.insert(file_type, builder.into());
                0
            }
            Err(e) => {
                push_error(&[Variant::from(e.to_string())]);
                1
            }
        }
    }

    #[func]
    fn import_doke(&self, file_type: String, md_path: String) -> Option<Gd<Resource>> {
        match self.__import_doke(file_type, md_path) {
            Ok(v) => Some(v),
            Err(e) => {push_error(&[Variant::from(e.to_string())]); None},
        }
    }

    fn __import_doke(
        &self,
        file_type: String,
        md_path: String,
    ) -> Result<Gd<Resource>, ImportError> {
        match self.import_doke_as_gd_value(file_type, md_path) {
            Ok(value) => {
                let res = import::godot_value_to_variant(value)?.try_to::<Gd<Resource>>();
                Ok(res?)
            }
            Err(_) => todo!(),
        }
    }

    fn import_doke_as_gd_value(
        &self,
        file_type: String,
        md_path: String,
    ) -> Result<GodotValue, ImportError> {
        // Only process .md files
        if !md_path.ends_with(".md") {
            return Err(ImportError::InvalidExtension(md_path.to_string()));
        }

        let mut input = String::new();
        // Open the file
        let file = std::fs::File::open(&md_path)?;
        let reader = std::io::BufReader::new(file);

        let mut separator_count = 0;

        for line in reader.lines() {
            let line = line?;
            if line.trim() == "---" {
                separator_count += 1;
                if separator_count == 3 {
                    break; // stop reading after the third "---"
                }
            }
            input.push_str(&line);
            input.push('\n');
        }

        // Get the parser for this file type
        if let Some(parser) = self.parsers.get(&file_type)
            && let Some(builder) = self.builders.get(&file_type)
        {
            let parsed = parser.validate(&input)?;
            let final_value = builder.build_file_resource(parsed)?;
            Ok(final_value)
        } else {
            Err(ImportError::MissingParserError())
        }
    }
}
