use std::fs;
use std::fs::{read, File};
use std::process::exit;
use xml::reader::XmlEvent;
use xml::EventReader;

fn main() {
    let file_path = "docs.gl/gl4/glClear.xhtml";
    let file = File::open(file_path).unwrap();
    let reader = EventReader::new(file);
    let mut content = String::default();
    reader
        .into_iter()
        .filter_map(|event| event.ok())
        .filter(|event| matches!(event, XmlEvent::Characters(_)))
        .for_each(|e| {
            if let XmlEvent::Characters(text) = e {
                content.push_str(&text);
            }
        });
    println!("{content}");
}
