use async_graphql::{Context, Object, Result, Schema, ID};
use slab::Slab;

use crate::state::AppState;

use super::{MutationRoot, MutationType, QueryRoot, SubscriptionRoot};

pub type ContentState = Slab<Content>;
pub type ContentSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

#[derive(Clone)]
pub struct Content {
    pub id: ID,
    pub name: String,
}

#[Object]
impl Content {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone)]
pub struct ContentChanged {
    pub mutation_type: MutationType,
    pub id: ID,
}

#[Object]
impl ContentChanged {
    async fn mutation_type(&self) -> MutationType {
        self.mutation_type
    }

    async fn id(&self) -> &ID {
        &self.id
    }

    async fn content(&self, ctx: &Context<'_>) -> Result<Option<Content>> {
        let contents = ctx.data_unchecked::<AppState>().contents.lock().await;
        let id = self.id.parse::<usize>()?;
        Ok(contents.get(id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_graphql::value;
    use futures::lock::Mutex;

    use super::*;

    #[tokio::test]
    async fn test_content() {
        let contents: Arc<Mutex<Slab<Content>>> = Default::default();
        contents.lock().await.insert(Content { id: "0".into(), name: "test".into() });
        let state = AppState { contents, ..Default::default() };
        let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).data(state).finish();

        let query = r#"
            query {
                content(id: "0") {
                    id
                    name
                }
            }
        "#;
        let res = schema.execute(query).await.data;
        assert_eq!(res, value!({"content": {"id": "0", "name": "test" }}));
    }

    #[tokio::test]
    async fn test_create_content() {
        let contents: Arc<Mutex<Slab<Content>>> = Default::default();
        let state = AppState { contents, ..Default::default() };
        let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).data(state).finish();

        let mutation0 = r#"
            mutation {
              createContent(name :"test0")
            }
        "#;
        let mutation1 = r#"
            mutation {
              createContent(name :"test1")
            }
        "#;
        let mutation2 = r#"
            mutation {
              createContent(name :"test2")
            }
        "#;
        schema.execute(mutation0).await;
        schema.execute(mutation1).await;
        schema.execute(mutation2).await;

        let query = r#"
            query {
                contents {
                    id
                }
            }
        "#;
        let res = schema.execute(query).await.data;
        assert_eq!(res, value!({"contents": [{"id": "0"}, {"id": "1"}, {"id": "2"}]}));
    }

    #[tokio::test]
    async fn test_delete_content() {
        let contents: Arc<Mutex<Slab<Content>>> = Default::default();
        contents.lock().await.insert(Content { id: "0".into(), name: "test".into() });
        let state = AppState { contents, ..Default::default() };
        let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).data(state).finish();

        let query = r#"
            query {
                contents {
                    id
                }
            }
        "#;
        let res = schema.execute(query).await.data;
        assert_eq!(res, value!({"contents": [{"id": "0"}]}));

        let mutation = r#"
            mutation {
              deleteContent(id: "0")
            }
        "#;
        schema.execute(mutation).await;

        let query = r#"
            query {
                contents {
                    id
                }
            }
        "#;
        let res = schema.execute(query).await.data;
        assert_eq!(res, value!({"contents": []}));
    }
}
