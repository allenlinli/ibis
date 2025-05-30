use super::{DbArticleOrComment, Source, read_from_string_or_source, user::PersonWrapper};
use crate::validate::validate_comment_max_depth;
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::{object::NoteType, public},
    protocol::{
        helpers::{deserialize_one_or_many, deserialize_skip_error},
        values::MediaTypeMarkdownOrHtml,
        verification::{verify_domains_match, verify_is_remote_object},
    },
    traits::Object,
};
use chrono::{DateTime, Utc};
use ibis_database::{
    common::{article::Article, comment::Comment, user::Person},
    error::BackendError,
    impls::{IbisContext, comment::DbCommentInsertForm},
};
use ibis_markdown::render_comment_markdown;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::ops::Deref;
use url::Url;

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApubComment {
    #[serde(rename = "type")]
    pub kind: NoteType,
    pub id: ObjectId<CommentWrapper>,
    pub attributed_to: ObjectId<PersonWrapper>,
    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub to: Vec<Url>,
    content: String,
    pub in_reply_to: ObjectId<DbArticleOrComment>,
    pub published: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    pub(crate) media_type: Option<MediaTypeMarkdownOrHtml>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub(crate) source: Option<Source>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommentWrapper(pub Comment);

impl Deref for CommentWrapper {
    type Target = Comment;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Comment> for CommentWrapper {
    fn from(value: Comment) -> Self {
        CommentWrapper(value)
    }
}

#[async_trait::async_trait]
impl Object for CommentWrapper {
    type DataType = IbisContext;
    type Kind = ApubComment;
    type Error = BackendError;

    async fn read_from_id(
        object_id: Url,
        context: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        Ok(Comment::read_from_ap_id(&object_id.into(), context)
            .ok()
            .map(Into::into))
    }

    async fn into_json(self, context: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let creator = Person::read(self.creator_id, context)?;
        let in_reply_to = if let Some(parent_comment_id) = self.parent_id {
            let comment = Comment::read(parent_comment_id, context)?;
            comment.ap_id.into()
        } else {
            let article = Article::read(self.article_id, context)?;
            article.ap_id.into()
        };
        Ok(ApubComment {
            kind: NoteType::Note,
            id: self.ap_id.clone().into(),
            attributed_to: creator.ap_id.into(),
            to: vec![public()],
            content: render_comment_markdown(&self.content),
            in_reply_to,
            published: Some(self.published),
            updated: self.updated,
            media_type: Some(MediaTypeMarkdownOrHtml::Html),
            source: Some(Source::new(self.content.clone())),
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        context: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        verify_is_remote_object(&json.id, context)?;
        Ok(())
    }

    async fn from_json(
        json: Self::Kind,
        context: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        let parent = json.in_reply_to.dereference(context).await?;
        let (article_id, parent_id, depth) = match parent {
            DbArticleOrComment::Left(db_article) => (db_article.id, None, 0),
            DbArticleOrComment::Right(db_comment) => (
                db_comment.article_id,
                Some(db_comment.id),
                db_comment.depth + 1,
            ),
        };
        let creator = json.attributed_to.dereference(context).await?;
        validate_comment_max_depth(depth)?;
        let content = read_from_string_or_source(&json.content, &json.media_type, &json.source);

        let form = DbCommentInsertForm {
            article_id,
            creator_id: creator.id,
            parent_id,
            ap_id: Some(json.id.into()),
            local: false,
            deleted: false,
            published: json.published.unwrap_or_else(Utc::now),
            updated: json.updated,
            content,
            depth,
        };

        Ok(Comment::create(form, context).await?.into())
    }
}
