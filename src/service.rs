use crate::graphql::Schema;
use std::{
    future::Future,
    task::{Context, Poll},
};
use tower::Service;

#[derive(Clone)]
pub struct GraphQLService<T> {
    f: T,
    schema: Schema,
}

impl<T> GraphQLService<T> {
    pub fn new(f: T, schema: Schema) -> Self {
        Self { f, schema }
    }
}

impl<T, F, Request, R, E> Service<Request> for GraphQLService<T>
where
    T: FnMut(Request, Schema) -> F,
    F: Future<Output = Result<R, E>>,
{
    type Response = R;
    type Error = E;
    type Future = F;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), E>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        (self.f)(req, self.schema.clone())
    }
}
