pub trait StringUtils {
    fn as_bytes_vec(&self) -> Vec<u8>;
}

impl StringUtils for String {
    fn as_bytes_vec(&self) -> Vec<u8> {
        self.as_bytes().iter().map(|byte| *byte).collect()
    }
}