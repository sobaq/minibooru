/// Transposes an Option<T, U> into an (Option<T>, Option<U>)
pub trait TransposeValues<T, U> {
    fn transpose_values(self) -> (Option<T>, Option<U>);
}

impl<T, U> TransposeValues<T, U> for Option<(T, U)> {
    fn transpose_values(self) -> (Option<T>, Option<U>) {
        match self {
            Some((t, u)) => (Some(t), Some(u)),
            None => (None, None),
        }
    }
}