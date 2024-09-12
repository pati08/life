use std::{fs::File, marker::PhantomData};

pub struct DataHandle<T> {
    file: File,
    _phantom_data: PhantomData<T>,
}

impl<T> DataHandle<T> {
    fn update<F: Fn(T) -> T>(with: F) -> Result
}
