use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::result::Result;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use xml::common::{Position, TextPosition};
use xml::reader::{EventReader, XmlEvent};

struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn trim_left(&mut self) {
        while !self.content.is_empty() && self.content[0].is_whitespace() {
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        let token = &self.content[0..n];
        self.content = &self.content[n..];
        token
    }

    fn chop_while(&mut self, mut predicate: fn(&char) -> bool) -> &'a [char] {
        let mut n = 0;
        while n < self.content.len() && predicate(&self.content[n]) {
            n += 1;
        }
        self.chop(n)
    }

    fn next_token(&mut self) -> Option<String> {
        self.trim_left();
        if self.content.is_empty() {
            return None;
        }

        if self.content[0].is_numeric() {
            return Some(self.chop_while(|x| x.is_numeric()).iter().collect());
        }

        if self.content[0].is_alphabetic() {
            return Some(
                self.chop_while(|x| x.is_alphanumeric())
                    .iter()
                    .map(|c| c.to_ascii_uppercase())
                    .collect(),
            );
        }

        return Some(self.chop(1).iter().collect());
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn parse_entire_xml_file(file_path: &Path) -> Result<String, ()> {
    let file = File::open(file_path).map_err(|err| {
        eprintln!(
            "ERROR: could not open file {file_path}: {err}",
            file_path = file_path.display()
        );
    })?;
    let er = EventReader::new(BufReader::new(file));
    let mut content = String::new();
    for event in er.into_iter() {
        let event = event.map_err(|err| {
            let TextPosition { row, column } = err.position();
            let msg = err.msg();
            eprintln!(
                "{file_path}:{row}:{column}: ERROR: {msg}",
                file_path = file_path.display()
            );
        })?;

        if let XmlEvent::Characters(text) = event {
            content.push_str(&text);
            content.push(' ');
        }
    }
    Ok(content)
}

type TermFreq = HashMap<String, usize>;
type TermFreqIndex = HashMap<PathBuf, TermFreq>;

fn check_index(index_path: &str) -> Result<(), ()> {
    println!("Reading {index_path} index file...");

    let index_file = File::open(index_path).map_err(|err| {
        eprintln!("ERROR: could not open index file {index_path}: {err}");
    })?;

    let tf_index: TermFreqIndex = serde_json::from_reader(index_file).map_err(|err| {
        eprintln!("ERROR: could not parse index file {index_path}: {err}");
    })?;

    println!(
        "{index_path} contains {count} files",
        count = tf_index.len()
    );

    Ok(())
}

fn save_tf_index(tf_index: &TermFreqIndex, index_path: &str) -> Result<(), ()> {
    println!("Saving {index_path}...");

    let index_file = File::create(index_path).map_err(|err| {
        eprintln!("ERROR: could not create index file {index_path}: {err}");
    })?;

    serde_json::to_writer(BufWriter::new(index_file), &tf_index).map_err(|err| {
        eprintln!("ERROR: could not serialize index into file {index_path}: {err}");
    })?;

    Ok(())
}

fn tf_index_of_folder(dir_path: &Path, tf_index: &mut TermFreqIndex) -> Result<(), ()> {
    let dir = fs::read_dir(dir_path).map_err(|err| {
        eprintln!(
            "ERROR: could not open directory {dir_path} for indexing: {err}",
            dir_path = dir_path.display()
        );
    })?;

    'next_file: for file in dir {
        let file = file.map_err(|err| {
            eprintln!(
                "ERROR: could not read next file in directory {dir_path} during indexing: {err}",
                dir_path = dir_path.display()
            );
        })?;

        let file_path = file.path();

        let file_type = file.file_type().map_err(|err| {
            eprintln!(
                "ERROR: could not determine type of file {file_path}: {err}",
                file_path = file_path.display()
            );
        })?;

        if file_type.is_dir() {
            tf_index_of_folder(&file_path, tf_index)?;
            continue 'next_file;
        }

        // TODO: how does this work with symlinks?

        println!("Indexing {:?}...", &file_path);

        let content = match parse_entire_xml_file(&file_path) {
            Ok(content) => content.chars().collect::<Vec<_>>(),
            Err(()) => continue 'next_file,
        };

        let mut tf = TermFreq::new();

        for term in Lexer::new(&content) {
            *tf.entry(term).or_default() += 1;
        }

        tf_index.insert(file_path, tf);
    }

    Ok(())
}

fn usage(program: &str) {
    eprintln!("Usage: {program} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands:");
    eprintln!(
        "    index <folder>         index the <folder> and save the index to index.json file"
    );
    eprintln!("    search <index-file>    check how many documents are indexed in the file (searching is not implemented yet)");
    eprintln!("    serve <index-file> [address] start http serve");
}

fn entry() -> Result<(), ()> {
    let mut args = env::args();
    let program = args.next().expect("path to program is provided");

    let subcommand = args.next().ok_or_else(|| {
        usage(&program);
        eprintln!("ERROR: no subcommand is provided");
    })?;

    match subcommand.as_str() {
        "index" => {
            let dir_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no directory is provided for {subcommand} subcommand");
            })?;

            let mut tf_index = TermFreqIndex::new();
            tf_index_of_folder(Path::new(&dir_path), &mut tf_index)?;
            save_tf_index(&tf_index, "index.json")?;
        }
        "search" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no path to index is provided for {subcommand} subcommand");
            })?;

            check_index(&index_path)?;
        }
        "serve" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: no path to index is provided for {subcommand} subcommand");
            })?;
            let index_file = File::open(&index_path).map_err(|err| {
                eprintln!("ERROR: could not open index file {index_path}: {err}");
            })?;

            let tf_index: TermFreqIndex = serde_json::from_reader(index_file).map_err(|err| {
                eprintln!("ERROR: could not parse index file {index_path}: {err}");
            })?;

            let addr = args.next().unwrap_or(String::from("0.0.0.0:8080"));
            let serve = Server::http(&addr).map_err(|e| {
                eprintln!("cloud not start serve on {addr} error: {e:?}");
            })?;
            println!("listening addr :{addr}");
            for request in serve.incoming_requests() {
                serve_request(&tf_index, request);
            }
        }
        _ => {
            usage(&program);
            eprintln!("ERROR: unknown subcommand {subcommand}");
            return Err(());
        }
    }

    Ok(())
}

fn serve_static_file(request: Request, file_path: &str, content_type: &str) -> Result<(), ()> {
    let index_file = File::open(file_path).map_err(|e| {
        eprintln!("can not serve file {file_path}:{e}");
    })?;
    let response = Response::from_file(index_file)
        .with_header(Header::from_bytes("content-type", content_type).unwrap());
    request.respond(response).map_err(|e| {
        eprintln!("response err:{e:?}");
    })?;
    Ok(())
}

fn serve_404(request: Request) -> Result<(), ()> {
    request
        .respond(Response::from_string("404").with_status_code(StatusCode(404)))
        .map_err(|e| {
            eprintln!("response err:{e:?}");
        })?;
    Ok(())
}

fn tf(t: &str, d: &TermFreq) -> f32 {
    d.get(t).map_or(0f32, |item| *item as f32) / (d.iter().map(|(_, f)| *f).sum::<usize>() as f32)
}

fn idf(t: &str, d: &TermFreqIndex) -> f32 {
    let N = d.len() as f32;
    let M = d.values().filter(|tf| tf.contains_key(t)).count().max(1) as f32;
    (N / M).log10()
}

fn serve_request(tf_index: &TermFreqIndex, mut request: Request) -> Result<(), ()> {
    println!(
        "req method: {:?} req url: {:?}",
        request.method(),
        request.url()
    );
    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            let mut bodyStr = String::new();
            request
                .as_reader()
                .read_to_string(&mut bodyStr)
                .map_err(|e| {
                    eprintln!("could not read_to_string err:{e}");
                })?;
            println!("bodyStr:{bodyStr}");
            let body = bodyStr.chars().collect::<Vec<_>>();
            let mut result = Vec::<(&PathBuf, f32)>::new();
            for (path, tf_table) in tf_index {
                let mut rank: f32 = 0f32;
                for token in Lexer::new(&body) {
                    rank += tf(&token, tf_table) * idf(&token, &tf_index);
                    // println!("{token} => {tf}", tf = tf(&token, tf_table));
                }
                result.push((path, rank));
                // println!("path:{path} => {total_tf}", path = path.display());
            }
            result.sort_by_key(|(_, rank)| (*rank * 1000_000f32) as usize);
            result.reverse();
            let json = serde_json::to_string(&result.iter().take(20).collect::<Vec<_>>()).map_err(
                |err| {
                    eprintln!("ERROR: could not convert search results to JSON: {err}");
                },
            )?;

            let content_type_header = Header::from_bytes("Content-Type", "application/json")
                .expect("That we didn't put any garbage in the headers");
            let response = Response::from_string(&json).with_header(content_type_header);
            request.respond(response).map_err(|err| {
                eprintln!("ERROR: could not serve a request {err}");
            })
        }
        (Method::Get, "/index.js") => serve_static_file(request, "index.js", "text/javascript"),
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, "index.html", "text/html")
        }
        _ => serve_404(request),
    }
}

fn main() -> ExitCode {
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
