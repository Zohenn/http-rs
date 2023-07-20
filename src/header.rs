use crate::token::is_valid_token;
use std::collections::HashMap;
use std::slice::Iter;

pub static HEADERS_WITH_NUMBER_VALUES: [&str; 1] = ["Content-Length"];

pub fn is_header_valid(header_name: &str, header_value: &str) -> bool {
    if !is_valid_token(header_name) {
        return false;
    }

    if HEADERS_WITH_NUMBER_VALUES.contains(&header_name) {
        return header_value.parse::<usize>().is_ok();
    }

    true
}

#[derive(Debug)]
pub struct Headers {
    inner: Vec<(String, String)>,
}

impl Headers {
    pub(crate) fn new() -> Self {
        Headers { inner: vec![] }
    }

    pub(crate) fn add(&mut self, header_name: &str, header_value: &str) {
        match self.has_inner(header_name, Some(header_value)) {
            Some(index) => {
                self.inner[index].1 = header_value.to_string();
            }
            None => {
                self.inner
                    .push((header_name.to_string(), header_value.to_string()));
            }
        }
    }

    pub(crate) fn has(&self, header_name: &str, header_value: Option<&str>) -> bool {
        self.has_inner(header_name, header_value).is_some()
    }

    fn has_inner(&self, header_name: &str, header_value: Option<&str>) -> Option<usize> {
        for (index, (name, value)) in self.inner.iter().enumerate() {
            if header_name.eq_ignore_ascii_case(name)
                && (header_value.is_none() || header_value.unwrap().eq_ignore_ascii_case(value))
            {
                return Some(index);
            }
        }

        None
    }

    pub(crate) fn get(&self, header_name: &str) -> Option<String> {
        self.has_inner(header_name, None)
            .map(|index| self.inner[index].1.clone())
    }

    pub(crate) fn iter(&self) -> Iter<'_, (String, String)> {
        self.inner.iter()
    }

    pub(crate) fn as_map(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();

        for (header_name, header_value) in self.inner.iter() {
            out.insert(header_name.clone(), header_value.clone());
        }

        out
    }
}

impl<const N: usize> From<[(String, String); N]> for Headers {
    fn from(value: [(String, String); N]) -> Self {
        Headers {
            inner: Vec::from(value),
        }
    }
}
