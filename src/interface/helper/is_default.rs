pub trait IsDefault {
    fn is_default(&self) -> bool;
}
impl<T> IsDefault for T
where
    T: Default + PartialEq<T>,
{
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}
