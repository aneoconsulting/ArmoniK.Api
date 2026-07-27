pub(crate) trait IntoCollection<T> {
    fn into_collect(self) -> T;
}

impl<X, Y, TX, TY> IntoCollection<TY> for TX
where
    X: Into<Y>,
    TX: IntoIterator<Item = X>,
    TY: IntoIterator<Item = Y>,
    TY: std::iter::FromIterator<Y>,
{
    fn into_collect(self) -> TY {
        self.into_iter().map(Into::into).collect()
    }
}
