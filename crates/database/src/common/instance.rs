use super::{
    article::Article,
    newtypes::InstanceId,
    user::{LocalUserView, Person},
};
use crate::DbUrl;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use url::Url;
#[cfg(feature = "ssr")]
use {
    crate::config::OAuthProvider,
    diesel::{Identifiable, Queryable, Selectable},
    doku::Document,
    ibis_database_schema::instance,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable, Selectable, Identifiable))]
#[cfg_attr(feature = "ssr", diesel(table_name = instance, check_for_backend(diesel::pg::Pg)))]
pub struct Instance {
    pub id: InstanceId,
    pub domain: String,
    pub ap_id: DbUrl,
    pub topic: Option<String>,
    pub articles_url: Option<DbUrl>,
    pub inbox_url: String,
    #[serde(skip)]
    pub public_key: String,
    #[serde(skip)]
    pub private_key: Option<String>,
    pub last_refreshed_at: DateTime<Utc>,
    pub local: bool,
    #[cfg(feature = "ssr")]
    pub instances_url: DbUrl,
    pub name: Option<String>,
}

impl Instance {
    pub fn inbox_url(&self) -> Url {
        Url::parse(&self.inbox_url).expect("can parse inbox url")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
pub struct InstanceView {
    pub instance: Instance,
    pub articles: Vec<Article>,
    pub following: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
pub struct InstanceFollow {
    pub instance: Instance,
    pub pending: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, SmartDefault)]
#[serde(default)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ssr", derive(Queryable, Document))]
#[cfg_attr(feature = "ssr", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Options {
    /// Whether users can create new accounts
    #[default = true]
    #[cfg_attr(feature = "ssr", doku(example = "true"))]
    pub registration_open: bool,
    /// Whether new users have to provide an email address to register
    #[default = false]
    #[cfg_attr(feature = "ssr", doku(example = "false"))]
    pub email_required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
#[cfg_attr(feature = "ssr", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct SiteView {
    pub my_profile: Option<LocalUserView>,
    pub config: Options,
    pub admin: Person,
    pub instance: Instance,
    pub oauth_providers: Vec<OAuthProviderPublic>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OAuthProviderPublic {
    pub issuer: Url,
    pub display_name: String,
    pub authorization_endpoint: Url,
    pub client_id: String,
    pub scopes: String,
}

#[cfg(feature = "ssr")]
impl From<OAuthProvider> for OAuthProviderPublic {
    fn from(provider: OAuthProvider) -> Self {
        Self {
            issuer: provider.issuer,
            display_name: provider.display_name,
            authorization_endpoint: provider.authorization_endpoint,
            client_id: provider.client_id,
            scopes: provider.scopes,
        }
    }
}
