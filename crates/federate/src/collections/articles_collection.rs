use crate::{activities::article::update_article::UpdateArticle, objects::article::ArticleWrapper};
use activitypub_federation::{
    config::Data,
    fetch::collection_id::CollectionId,
    kinds::collection::CollectionType,
    protocol::verification::verify_domains_match,
    traits::{ActivityHandler, Collection},
};
use futures::future::{join_all, try_join_all};
use ibis_database::{
    common::{article::Article, utils::http_protocol_str},
    error::{BackendError, BackendResult},
    impls::IbisContext,
};
use log::warn;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApubArticleCollection {
    pub r#type: CollectionType,
    pub id: Url,
    pub total_items: i32,
    pub items: Vec<UpdateArticle>,
}

#[derive(Clone, Debug)]
pub struct ArticleCollection(());

pub fn local_articles_url(domain: &str) -> BackendResult<CollectionId<ArticleCollection>> {
    Ok(CollectionId::parse(&format!(
        "{}://{domain}/all_articles",
        http_protocol_str()
    ))?)
}

#[async_trait::async_trait]
impl Collection for ArticleCollection {
    type Owner = ();
    type DataType = IbisContext;
    type Kind = ApubArticleCollection;
    type Error = BackendError;

    async fn read_local(
        _owner: &Self::Owner,
        context: &Data<Self::DataType>,
    ) -> Result<Self::Kind, Self::Error> {
        let local_articles = Article::read_all(Some(true), None, false, context)?;
        let articles = try_join_all(
            local_articles
                .into_iter()
                .map(ArticleWrapper)
                .map(|a| UpdateArticle::new(a, context))
                .collect::<Vec<_>>(),
        )
        .await?;
        let collection = ApubArticleCollection {
            r#type: Default::default(),
            id: local_articles_url(&context.conf.federation.domain)?.into(),
            total_items: articles.len() as i32,
            items: articles,
        };
        Ok(collection)
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _context: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(&json.id, expected_domain)?;
        Ok(())
    }

    async fn from_json(
        apub: Self::Kind,
        _owner: &Self::Owner,
        context: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        let articles = apub
            .items
            .into_iter()
            .filter(|i| !i.object.id.is_local(context))
            .map(|update| async {
                let id = update.object.id.clone();
                UpdateArticle::verify(&update, context).await?;
                let res = UpdateArticle::receive(update, context).await;
                if let Err(e) = &res {
                    warn!("Failed to synchronize article {id}: {e}");
                }
                res
            });
        join_all(articles).await;

        Ok(ArticleCollection(()))
    }
}
