use std::collections::HashMap;
use std::fs::{read, File};
use std::path::Path;
use std::process::exit;
use std::thread::sleep;
use std::{fs, io};
use xml::reader::XmlEvent;
use xml::EventReader;

#[derive(Debug)]
struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn trim_left(&mut self) {
        while self.content.len() > 0 && self.content[0].is_whitespace() {
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        let token = &self.content[0..n];
        self.content = &self.content[n..];
        token
    }

    fn chop_while<P>(&mut self, predicate: P) -> &'a [char]
    where
        P: Fn(&char) -> bool,
    {
        let mut n = 0;
        while self.content.len() > 0 && predicate(&self.content[n]) {
            n += 1;
        }
        self.chop(n)
    }

    fn next_token(&mut self) -> Option<&'a [char]> {
        self.trim_left();
        if self.content.len() == 0 {
            return None;
        }
        if self.content[0].is_numeric() {
            return Some(self.chop_while(|x| x.is_numeric()));
        }
        if self.content[0].is_alphabetic() {
            return Some(self.chop_while(|x| x.is_alphanumeric()));
        }
        Some(self.chop(1))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn index_document(_doc_content: &str) -> HashMap<String, usize> {
    todo!()
}

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path)?;
    let reader = EventReader::new(file);
    let mut content = String::default();
    reader
        .into_iter()
        .filter_map(|event| event.ok())
        .filter(|event| matches!(event, XmlEvent::Characters(_)))
        .for_each(|e| {
            if let XmlEvent::Characters(text) = e {
                content.push_str(&text);
                content.push(' ');
            }
        });
    Ok(content)
}

fn main() -> io::Result<()> {
    /*let file_path = "docs.gl/gl4/glClear.xhtml";
    println!(
        "{content}",
        content = read_entire_xml_file(file_path).unwrap()
    );*/
    let content = read_entire_xml_file("docs.gl/gl4/glClear.xhtml")?
        .chars()
        .collect::<Vec<_>>();
    let lexer = Lexer::new(&content);
    for token in lexer {
        println!("{token}", token = token.iter().collect::<String>())
    }
    /*let all_documents = HashMap::<String, HashMap<String, usize>>::new();
    let dir_path = "docs.gl/gl4/";
    let dir = fs::read_dir(dir_path)?;
    for file in dir {
        let file_path = file?.path();
        let content = read_entire_xml_file(&file_path)?;
        println!("{file_path:?} => size {size}", size = content.len());
    }*/

    Ok(())
}
