use std::io;

use itertools::Itertools;
use strsim::damerau_levenshtein;

use crate::{
    nodes::Word,
    pages::file_explorer::{FileTree, MdFile},
};

pub fn find_md_files() -> Result<FileTree, io::Error> {
    let mut tree = FileTree::new();
    let mut stack = vec![std::path::PathBuf::from(".")];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().unwrap_or_default() == "md" {
                let name = path.file_name().unwrap().to_str().unwrap().to_string();
                let path = path.to_str().unwrap().to_string();
                tree.add_file(MdFile::new(path, name));
            }
        }
    }
    tree.sort_2();
    Ok(tree)
}

pub fn find_files(files: &[MdFile], query: &str) -> Vec<MdFile> {
    if query.is_empty() {
        return files.to_vec();
    }

    // Check if any char in the query is uppercase, making the search case sensitive
    let case_sensitive = query.chars().any(|c| c.is_uppercase());

    files
        .iter()
        .filter(|file| {
            let file_path = if case_sensitive {
                file.path.to_owned()
            } else {
                file.path.to_lowercase()
            };
            let res = char_windows(&file_path, query.len())
                .any(|window| damerau_levenshtein(window, query) == 0);
            res
        })
        .cloned()
        .collect()
}

pub fn find_with_backoff(query: &str, text: &str) -> Vec<usize> {
    let precision = 0;
    let mut result = find(query, text, precision);
    if result.is_empty() {
        let precision = 1;
        result = find(query, text, precision);
    }
    result
}

pub fn find(query: &str, text: &str, precision: usize) -> Vec<usize> {
    let mut result = Vec::new();

    let case_sensitive = query.chars().any(|c| c.is_uppercase());

    char_windows(text, query.len())
        .enumerate()
        .for_each(|(i, window)| {
            let window = if case_sensitive {
                window.to_owned()
            } else {
                window.to_lowercase()
            };
            let score = damerau_levenshtein(query, &window);
            if score <= precision {
                result.push(i);
            }
        });

    result
}

/// Returns line numbers that match the query with the given precision.
pub fn line_match(query: &str, text: Vec<&str>, precision: usize) -> Vec<usize> {
    text.iter()
        .enumerate()
        .filter_map(|(i, line)| {
            if find(query, line, precision).is_empty() {
                None
            } else {
                Some(i)
            }
        })
        .collect()
}

pub fn find_line_match_and_index(
    query: &str,
    lines: Vec<&str>,
    precision: usize,
) -> Vec<(usize, usize)> {
    lines
        .iter()
        .enumerate()
        .flat_map(|(i, line)| {
            find(query, line, precision)
                .into_iter()
                .map(move |j| (i, j))
        })
        .collect()
}

fn char_windows(src: &str, win_size: usize) -> impl Iterator<Item = &'_ str> {
    src.char_indices().flat_map(move |(from, _)| {
        src[from..]
            .char_indices()
            .nth(win_size - 1)
            .map(|(to, c)| &src[from..from + to + c.len_utf8()])
    })
}

pub fn compare_heading(link_header: &str, header: &Vec<Vec<Word>>) -> bool {
    let header: String = header
        .iter()
        .flatten()
        .map(|word| word.content().to_lowercase())
        .collect::<Vec<String>>()
        .join("-")
        .replace(['(', ')', '[', ']', '{', '}', '<', '>', '"', '\'', ' ', '/'], "")
        .chars()
        .dedup_by(|a, b| *a == '-' && *b == '-')
        .collect();

    link_header == header
}

#[cfg(test)]
#[test]
fn test_find() {
    let text = "Hello, world!";
    let query = "world";
    let precision = 0;
    let result = find(query, text, precision);
    assert_eq!(result, vec![7]);
}

#[test]
fn test_find_with_backoff() {
    let text = "Hello, world!";
    let query = "world";
    let result = find_with_backoff(query, text);
    assert_eq!(result, vec![7]);
}

#[test]
fn test_find_with_backoff_with_typo() {
    let text = "Hello, world!";
    let query = "wrold";
    let result = find_with_backoff(query, text);
    assert_eq!(result, vec![7]);
}

#[test]
fn test_vec_find() {
    let text = vec!["Hello", "hello", "world", "World"];
    let query = "world";
    let precision = 0;
    let result = line_match(query, text, precision);
    assert_eq!(result, vec![2]);
}

#[test]
fn test_vec_find_less_precision() {
    let text = vec!["Hello", "hello", "world", "World"];
    let query = "world";
    let precision = 1;
    let result = line_match(query, text, precision);
    assert_eq!(result, vec![2, 3]);
}

#[test]
fn test_vec_find_with_typo() {
    let text = vec!["Hello", "hello", "world", "World"];
    let query = "wrold";
    let precision = 2;
    let result = line_match(query, text, precision);
    assert_eq!(result, vec![2, 3]);
}

#[test]
fn test_find_line_match_and_index() {
    let text = vec!["Hello", "hello", "world", "hello world"];
    let query = "world";
    let precision = 0;
    let result = find_line_match_and_index(query, text, precision);
    assert_eq!(result, vec![(2, 0), (3, 6)]);
}
