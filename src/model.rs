use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub type TermFreq = HashMap<String, usize>;
pub type DocFreq = HashMap<String, usize>;
pub type TermFreqPreDoc = HashMap<PathBuf, (usize, TermFreq)>;

#[derive(Deserialize, Serialize, Default)]
pub struct Model {
    pub df: DocFreq,
    pub tfpd: TermFreqPreDoc,
}

pub fn compute_tf(t: &str, n: usize, d: &TermFreq) -> f32 {
    let a = d.get(t).cloned().unwrap_or(0) as f32;
    let b = n as f32;
    a / b
}

pub fn compute_idf(t: &str, n: usize, df: &DocFreq) -> f32 {
    let n = n as f32;
    // let m = d.values().filter(|tf| tf.contains_key(t)).count().max(1) as f32;
    let m = df.get(t).cloned().unwrap_or(1) as f32;
    (n / m).log10()
}

pub struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a [char]) -> Self {
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

    fn chop_while(&mut self, predicate: fn(&char) -> bool) -> &'a [char] {
        let mut n = 0;
        while n < self.content.len() && predicate(&self.content[n]) {
            n += 1;
        }
        self.chop(n)
    }

    pub fn next_token(&mut self) -> Option<String> {
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
                    .map(|x| x.to_ascii_uppercase())
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

pub fn search_query<'a>(model: &'a Model, query: &'a [char]) -> Vec<(&'a Path, f32)> {
    let mut result = Vec::<(&Path, f32)>::new();
    let tokens = Lexer::new(&query).collect::<Vec<_>>();
    let tf_index = &model.tfpd;
    for (path, (n, tf_table)) in tf_index {
        let mut rank = 0f32;
        for token in &tokens {
            rank +=
                compute_tf(&token, *n, &tf_table) * compute_idf(&token, tf_index.len(), &model.df);
        }
        result.push((path, rank));
    }
    result.sort_by(|(_, rank1), (_, rank2)| rank1.partial_cmp(rank2).unwrap());
    result.reverse();
    result
}
