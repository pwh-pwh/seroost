use std::collections::HashMap;
use std::fs::{read, File};
use std::path::{Path, PathBuf};
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

    fn chop_while(&mut self, predicate: fn(&char) -> bool) -> &'a [char] {
        let mut n = 0;
        while !self.content.is_empty() && predicate(&self.content[n]) {
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

type TermFreq = HashMap<String, usize>;
type TermFreqIndex = HashMap<PathBuf, TermFreq>;

fn main() -> io::Result<()> {
    let index_path = "index.json";
    let index_file = File::open(index_path)?;
    println!("Reading {index_file:?}");
    let tf_index: TermFreqIndex = serde_json::from_reader(index_file)?;
    for (path, tf) in tf_index {
        println!("{path:?} has {count} tokens", count = tf.len());
    }
    Ok(())
}

fn main2() -> io::Result<()> {
    /*let file_path = "docs.gl/gl4/glClear.xhtml";
    println!(
        "{content}",
        content = read_entire_xml_file(file_path).unwrap()
    );*/
    //read one file
    /*let file_path = "docs.gl/gl4/glClear.xhtml";
    let content = read_entire_xml_file(file_path)?.chars().collect::<Vec<_>>();
    let lexer = Lexer::new(&content);
    let mut tf = HashMap::<String, usize>::new();
    for token in lexer {
        let term = token
            .iter()
            .map(|t| t.to_ascii_uppercase())
            .collect::<String>();
        *tf.entry(term).or_default() += 1;
    }
    let mut stats = tf.iter().collect::<Vec<_>>();
    stats.sort_by_key(|(t, f)| *f);
    stats.reverse();
    println!("{file_path}");
    for (t, f) in stats.iter().take(10) {
        println!("   {t}:{f}");
    }*/
    // let all_documents = HashMap::<String, HashMap<String, usize>>::new();
    let dir_path = "docs.gl/gl4/";
    let top_n = 10;
    let dir = fs::read_dir(dir_path)?;
    let mut tf_index = TermFreqIndex::new();
    for file in dir {
        let file_path = file?.path();
        println!("Indexing {file_path:?}");
        let content = read_entire_xml_file(&file_path)?
            .chars()
            .collect::<Vec<_>>();
        let lexer = Lexer::new(&content);
        let mut tf = TermFreq::new();
        for token in lexer {
            let term = token
                .iter()
                .map(|t| t.to_ascii_uppercase())
                .collect::<String>();
            *tf.entry(term).or_default() += 1;
        }
        let mut stats = tf.iter().collect::<Vec<_>>();
        stats.sort_by_key(|(t, f)| *f);
        stats.reverse();
        tf_index.insert(file_path, tf);
    }
    let index_path = "index.json";
    println!("saving {index_path}");
    let index_file = File::create(index_path)?;
    serde_json::to_writer(index_file, &tf_index).expect("TODO: panic message");

    Ok(())
}
