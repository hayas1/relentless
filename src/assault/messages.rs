use std::{fmt::Display, vec::IntoIter as VecIntoIter};

use serde::{Deserialize, Serialize};

use super::{
    destinations::Destinations,
    error::{RequestError, RequestResult},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Messages<M>(Vec<M>); // TODO size limit ?
impl<M> Default for Messages<M> {
    fn default() -> Self {
        // derive(Default) do not implement Default when T are not implement Default
        // https://github.com/rust-lang/rust/issues/26925
        Self(Default::default())
    }
}
impl<M: Display> Display for Messages<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (m, n) = (self.0.len(), 3);
        for (i, wrap) in self.0[..n.min(m)].iter().enumerate() {
            if i < n.min(m) - 1 {
                writeln!(f, "{}", wrap)?;
            } else {
                write!(f, "{}", wrap)?;
            }
        }
        if m > n {
            writeln!(f)?;
            write!(f, "... and {} more", m - n)?;
        }
        Ok(())
    }
}
impl<M> IntoIterator for Messages<M> {
    type Item = M;
    type IntoIter = VecIntoIter<M>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<M> From<Vec<M>> for Messages<M> {
    fn from(value: Vec<M>) -> Self {
        Self(value)
    }
}
impl<M> FromIterator<M> for Messages<M> {
    fn from_iter<T: IntoIterator<Item = M>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl<M> Extend<M> for Messages<M> {
    fn extend<T: IntoIterator<Item = M>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

/// TODO doc
impl<M> Messages<M> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn as_slice(&self) -> &[M] {
        &self.0
    }
    pub fn as_slice_mut(&mut self) -> &mut [M] {
        &mut self.0
    }
    pub fn to_vec(self) -> Vec<M> {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push_if_err<T>(&mut self, message: Result<T, M>) -> Option<T> {
        // message.map_or_else(|m| {self.0.push(m);None},|t| Some(t))
        match message {
            Ok(t) => Some(t),
            Err(m) => {
                self.0.push(m);
                None
            }
        }
    }
    pub fn push_err(&mut self, message: M) {
        self.push_if_err::<()>(Err(message));
    }
    pub fn push_unacceptable(&mut self, message: M) -> bool {
        !matches!(self.push_err(message), ()) // always return false
    }
    pub fn response_with<Res, F: Fn(RequestError) -> M>(&mut self, res: RequestResult<Res>, f: F) -> Option<Res> {
        self.push_if_err(res.map(|r| r.into_response()).map_err(f))
    }
    pub fn response_destinations_with<Res, F: Fn(RequestError) -> M + Clone>(
        &mut self,
        res: Destinations<RequestResult<Res>>,
        f: F,
    ) -> Option<Destinations<Res>> {
        res.into_iter().map(|(d, r)| Some((d, self.response_with(r, f.clone())?))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(Messages::from(vec!["Hello World!"; 0]).to_string(), "");
        assert_eq!(Messages::from(vec!["Hello World!"; 1]).to_string(), ["Hello World!"].join("\n"));
        assert_eq!(Messages::from(vec!["Hello World!"; 2]).to_string(), ["Hello World!", "Hello World!"].join("\n"));
        assert_eq!(
            Messages::from(vec!["Hello World!"; 3]).to_string(),
            ["Hello World!", "Hello World!", "Hello World!"].join("\n")
        );
        assert_eq!(
            Messages::from(vec!["Hello World!"; 4]).to_string(),
            ["Hello World!", "Hello World!", "Hello World!", "... and 1 more"].join("\n")
        );
        assert_eq!(
            Messages::from(vec!["Hello World!"; 5]).to_string(),
            ["Hello World!", "Hello World!", "Hello World!", "... and 2 more"].join("\n")
        );
        assert_eq!(
            Messages::from(vec!["Hello World!"; 100]).to_string(),
            ["Hello World!", "Hello World!", "Hello World!", "... and 97 more"].join("\n")
        );
    }

    #[test]
    fn test_push_if_err() {
        let mut messages = Messages::new();
        let Some(t) = messages.push_if_err(Ok("Hello World!")) else {
            panic!("Ok(x) does not cause error, so Some(x) was returned")
        };
        assert_eq!(t, "Hello World!");

        let Some(()) = messages.push_if_err(Err("Hello World!!")) else {
            assert_eq!(messages.to_vec(), vec!["Hello World!!"]);
            return;
        };
        panic!("Err(x) cause error, so message was pushed and None was returned");
    }
}
