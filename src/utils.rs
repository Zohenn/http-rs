use std::str::Utf8Error;

pub trait StringUtils {
    fn as_bytes_vec(&self) -> Vec<u8>;

    fn try_from_vec(value: Vec<u8>) -> Result<String, Utf8Error>;

    fn from_vec(value: Vec<u8>) -> String;
}

impl StringUtils for String {
    fn as_bytes_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn try_from_vec(value: Vec<u8>) -> Result<Self, Utf8Error> {
        let result = std::str::from_utf8(&value);

        if let Ok(s) = result {
            Ok(String::from(s))
        } else {
            Err(result.unwrap_err())
        }
    }

    fn from_vec(value: Vec<u8>) -> Self {
        String::try_from_vec(value).unwrap_or(String::new())
    }
}

pub trait IteratorUtils<'a, T: 'a>: Iterator<Item = &'a T>
where
    T: Copy,
{
    fn take_while_copy(&mut self, predicate: impl FnMut(&Self::Item) -> bool) -> Vec<T>;
}

impl<'a, It, T: 'a + Copy> IteratorUtils<'a, T> for It
where
    It: Iterator<Item = &'a T>,
{
    fn take_while_copy(&mut self, predicate: impl FnMut(&&'a T) -> bool) -> Vec<T> {
        self.take_while(predicate).copied().collect()
    }
}
