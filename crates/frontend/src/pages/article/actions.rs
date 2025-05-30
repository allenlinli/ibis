use crate::pages::article_resource;
use ibis_api_client::{
    CLIENT,
    article::{ForkArticleParams, ProtectArticleParams},
    errors::FrontendResultExt,
};
use ibis_database::common::{article::Article, newtypes::ArticleId};
use ibis_frontend_components::{
    article_nav::{ActiveTab, ArticleNav},
    suspense_error::SuspenseError,
    utils::{formatting::article_path, resources::is_admin},
};
use leptos::{ev::KeyboardEvent, prelude::*};
use leptos_router::components::Redirect;

#[component]
pub fn ArticleActions() -> impl IntoView {
    let article = article_resource();
    let (new_title, set_new_title) = signal(String::new());
    let (fork_response, set_fork_response) = signal(Option::<Article>::None);
    let fork_action = Action::new(move |(article_id, new_title): &(ArticleId, String)| {
        let params = ForkArticleParams {
            article_id: *article_id,
            new_title: new_title.to_string(),
        };
        async move {
            CLIENT
                .fork_article(&params)
                .await
                .error_popup(|res| set_fork_response.set(Some(res.article)));
        }
    });
    let protect_action = Action::new(move |(id, protected): &(ArticleId, bool)| {
        let params = ProtectArticleParams {
            article_id: *id,
            protected: !protected,
        };
        async move {
            CLIENT
                .protect_article(&params)
                .await
                .error_popup(|_| article.refetch());
        }
    });
    let remove_action = Action::new(move |(id, removed): &(ArticleId, bool)| {
        let (id, removed) = (*id, *removed);
        async move {
            CLIENT
                .remove_article(id, !removed)
                .await
                .error_popup(|_| article.refetch());
        }
    });
    view! {
        <ArticleNav article=article active_tab=ActiveTab::Actions />
        <SuspenseError result=article>
            {move || Suspend::new(async move {
                article
                    .await
                    .map(|article| {
                        view! {
                            <div>
                                <Show when=move || { is_admin() && article.article.local }>
                                    <div class="m-4">
                                        <button
                                            class="btn btn-secondary"
                                            title="Protect a local article so that only admins can edit it"
                                            on:click=move |_| {
                                                protect_action
                                                    .dispatch((article.article.id, article.article.protected));
                                            }
                                        >
                                            Toggle Article Protection
                                        </button>
                                    </div>
                                    <div class="m-4">
                                        <button
                                            class="btn btn-secondary"
                                            on:click=move |_| {
                                                remove_action
                                                    .dispatch((article.article.id, article.article.removed));
                                            }
                                        >
                                            Toggle Article Removal
                                        </button>
                                    </div>
                                </Show>
                                <input
                                    class="input"
                                    placeholder="New Title"
                                    on:keyup=move |ev: KeyboardEvent| {
                                        let val = event_target_value(&ev);
                                        set_new_title.update(|v| *v = val);
                                    }
                                />

                                <button
                                    class="btn"
                                    disabled=move || new_title.get().is_empty()
                                    on:click=move |_| {
                                        fork_action.dispatch((article.article.id, new_title.get()));
                                    }
                                >

                                    Fork Article
                                </button>
                                <p>
                                    "You can fork a remote article to the local instance. This is useful if the original
                                    instance is dead, or if there are disagreements how the article should be written."
                                </p>
                            </div>
                        }
                    })
            })}
            {fork_response.get().map(|article| view! { <Redirect path=article_path(&article) /> })}
        </SuspenseError>
    }
}
