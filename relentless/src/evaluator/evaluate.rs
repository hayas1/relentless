use crate::{error::EvaluateError, shot::destinations::Destinations};

// TODO error handling
pub trait Evaluator<S> {
    type Error;
    fn evaluate_shot(&self, res: &S) -> Result<(), Self::Error>;
    fn evaluate_compare(&self, res1: &S, res2: &S) -> Result<(), Self::Error>;

    fn evaluate<F: Fn(bool) -> Self::Error>(&self, judge: bool, e: F) -> Result<(), Self::Error> {
        if judge {
            Ok(())
        } else {
            Err(e(judge))
        }
    }
    fn evaluate_shots(&self, res: Destinations<S>) -> Result<(), Self::Error>
    where
        Self::Error: From<EvaluateError>,
    {
        let mut popper = res.into_iter();
        let (_, resp) = popper.next().ok_or(EvaluateError::EmptyTarget)?;
        match popper.next() {
            Some(_) => Err(EvaluateError::ShouldCompare)?,
            None => self.evaluate_shot(&resp),
        }
    }
    fn evaluate_compares(&self, res: Destinations<S>) -> Result<(), Self::Error>
    where
        Self::Error: From<EvaluateError>,
    {
        match res.len() {
            0 => Err(EvaluateError::EmptyTarget)?,
            1 => Err(EvaluateError::ShouldShot)?,
            _ => (),
        }
        let v: Vec<_> = res.into_iter().collect();
        let ok = v.windows(2).try_fold((), |(), w| {
            let ((_, a), (_, b)) = (&w[0], &w[1]);
            self.evaluate_compare(a, b)
        });
        ok
    }
}
