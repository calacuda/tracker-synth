// TODO: Make a macro to build a "LessThan" type for any given numeric type

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsizeLessThan<const LT: usize>(pub usize);

impl<const LT: usize> TryFrom<usize> for UsizeLessThan<LT> {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < LT {
            Ok(Self(value))
        } else {
            Err(format!(
                "Too big Error! {value} was expected to be less than {LT}"
            ))
        }
    }
}
