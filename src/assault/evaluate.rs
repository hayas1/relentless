use super::{destinations::Destinations, error::RequestResult, messages::Messages};

#[allow(async_fn_in_trait)] // TODO #[warn(async_fn_in_trait)] by default
pub trait Evaluate<Res> {
    type Message;
    async fn evaluate(&self, res: Destinations<RequestResult<Res>>, msg: &mut Messages<Self::Message>) -> bool;
}

pub trait Acceptable<T> {
    type Message;
    fn accept(&self, dest: &Destinations<T>, msg: &mut Messages<Self::Message>) -> bool;
    // TODO infallible
    fn sub_accept<U, A: Acceptable<U>, F: Fn(A::Message) -> Self::Message>(
        acceptable: &A,
        dest: &Destinations<U>,
        msg: &mut Messages<Self::Message>,
        convert: F,
    ) -> bool {
        let mut sub_msg = Messages::new();
        let accept = acceptable.accept(dest, &mut sub_msg);
        msg.extend(sub_msg.into_iter().map(convert));
        accept
    }

    fn assault_or_compare<F>(d: &Destinations<T>, f: F) -> bool
    where
        T: PartialEq,
        F: FnMut((&String, &T)) -> bool,
    {
        if d.len() == 1 {
            Self::validate_all(d, f)
        } else {
            Self::compare_all(d)
        }
    }
    fn validate_all<F>(d: &Destinations<T>, f: F) -> bool
    where
        F: FnMut((&String, &T)) -> bool,
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
