use crate::{
    generate_activity_id,
    objects::instance::InstanceWrapper,
    routes::AnnouncableActivities,
    send_ibis_activity,
};
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::{activity::AnnounceType, public},
    protocol::helpers::deserialize_one_or_many,
    traits::{ActivityHandler, Actor},
};
use ibis_database::{
    common::instance::Instance,
    error::{BackendError, BackendResult},
    impls::IbisContext,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnounceActivity {
    pub(crate) actor: ObjectId<InstanceWrapper>,
    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub(crate) to: Vec<Url>,
    #[serde(deserialize_with = "deserialize_one_or_many")]
    pub(crate) cc: Vec<Url>,
    pub(crate) object: AnnouncableActivities,
    #[serde(rename = "type")]
    pub(crate) kind: AnnounceType,
    pub(crate) id: Url,
}

impl AnnounceActivity {
    pub async fn send(
        object: AnnouncableActivities,
        context: &Data<IbisContext>,
    ) -> BackendResult<()> {
        let id = generate_activity_id(context)?;
        let instance: InstanceWrapper = Instance::read_local(context)?.into();
        let announce = AnnounceActivity {
            actor: instance.id().into(),
            to: vec![public()],
            cc: vec![instance.followers_url()?],
            object,
            kind: AnnounceType::Announce,
            id,
        };

        // Send to followers of instance
        let follower_inboxes: Vec<_> = Instance::read_followers(instance.id, context)?
            .into_iter()
            .map(|f| f.inbox_url())
            .collect();
        send_ibis_activity(&instance, announce, follower_inboxes, context).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl ActivityHandler for AnnounceActivity {
    type DataType = IbisContext;
    type Error = BackendError;

    fn id(&self) -> &Url {
        &self.id
    }

    fn actor(&self) -> &Url {
        self.actor.inner()
    }

    async fn verify(&self, _context: &Data<Self::DataType>) -> BackendResult<()> {
        Ok(())
    }

    async fn receive(self, context: &Data<Self::DataType>) -> BackendResult<()> {
        self.object.verify(context).await?;
        self.object.receive(context).await
    }
}
