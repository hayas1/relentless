use std::time::Duration;

use super::destinations::Destinations;

pub enum RequestResult<Res> {
    Response(Res),
    Timeout(Duration),
}

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluator<Res> {
    type Message;
    async fn evaluate(&self, res: Destinations<RequestResult<Res>>, msg: &mut Vec<Self::Message>) -> bool;
}

pub trait Acceptable<T> {
    type Message;
    fn accept(&self, dest: &Destinations<T>, msg: &mut Vec<Self::Message>) -> bool;

    fn assault_or_compare<F>(d: &Destinations<T>, f: F) -> bool
    where
        T: PartialEq,
        F: Fn((&String, &T)) -> bool,
    {
        if d.len() == 1 {
            Self::validate_all(d, f)
        } else {
            Self::compare_all(d)
        }
    }
    fn validate_all<F>(d: &Destinations<T>, f: F) -> bool
    where
        F: Fn((&String, &T)) -> bool,
    {
        d.iter().all(f)
    }
    fn compare_all(status: &Destinations<T>) -> bool
    where
        T: PartialEq,
    {
        let v: Vec<_> = status.values().collect();
        v.windows(2).all(|w| w[0] == w[1])
    }
}
